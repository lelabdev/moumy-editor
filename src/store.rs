use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::recipe::{Recipe, RecipeFrontmatter, RecipeInput};

/// Parse a .md recipe file into a Recipe struct.
pub fn parse_recipe(path: &Path) -> Option<Recipe> {
    let content = fs::read_to_string(path).ok()?;
    let filename = path.file_name()?.to_str()?.to_string();

    // Split frontmatter and body
    if !content.starts_with("---") {
        return None;
    }

    let rest = &content[3..];
    let end = rest.find("---")?;
    let yaml_str = &rest[..end];
    let body = rest[end + 3..].trim();

    let mut frontmatter: RecipeFrontmatter = serde_yaml::from_str(yaml_str).ok()?;

    // Derive slug from filename if not set
    let slug = frontmatter
        .slug
        .clone()
        .unwrap_or_else(|| filename.trim_end_matches(".md").to_string());
    frontmatter.slug = Some(slug.clone());

    // Parse numbered steps from body
    let steps = parse_steps(body);

    Some(Recipe {
        filename,
        slug,
        frontmatter,
        steps,
    })
}

/// Parse numbered steps like "1. Do something\n2. Do another thing"
fn parse_steps(body: &str) -> Vec<String> {
    body.lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
        })
        .map(|line| {
            let trimmed = line.trim();
            // Remove leading "N. " prefix
            if let Some(dot_pos) = trimmed.find(". ") {
                if trimmed[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
                    return trimmed[dot_pos + 2..].to_string();
                }
            }
            trimmed.to_string()
        })
        .collect()
}

/// Serialize a Recipe back to a .md file string.
pub fn serialize_recipe(input: &RecipeInput, slug: &str) -> String {
    let fm = RecipeFrontmatter {
        title: input.title.clone(),
        slug: Some(slug.to_string()),
        manuscript: Some(String::new()),
        category: input.category.clone(),
        excerpt: input.excerpt.clone(),
        prep_time: input.prep_time.clone(),
        cook_time: input.cook_time.clone(),
        servings: input.servings.clone(),
        difficulty: input.difficulty.clone(),
        ingredients: input.ingredients.clone(),
        ingredients2_title: input.ingredients2_title.clone(),
        ingredients2: input.ingredients2.clone(),
        notes: input.notes.clone(),
    };

    let mut yaml = serde_yaml::to_string(&fm).unwrap();
    // serde_yaml adds trailing newline, strip it for clean output
    yaml = yaml.trim_end().to_string();

    let steps: String = input
        .steps
        .iter()
        .enumerate()
        .map(|(i, step)| format!("{}. {}", i + 1, step))
        .collect::<Vec<_>>()
        .join("\n\n");

    format!("---\n{}\n---\n\n{}\n", yaml, steps)
}

/// List all .md files in a directory, parsed as recipes.
pub fn list_recipes(dir: &Path) -> Vec<Recipe> {
    WalkDir::new(dir)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .is_some_and(|ext| ext == "md")
        })
        .filter_map(|e| parse_recipe(e.path()))
        .collect()
}

/// Get a single recipe by slug.
pub fn get_recipe(dir: &Path, slug: &str) -> Option<Recipe> {
    let path = find_recipe_path(dir, slug)?;
    parse_recipe(&path)
}

/// Save a recipe (create or update).
pub fn save_recipe(dir: &Path, slug: &str, input: &RecipeInput) -> std::io::Result<PathBuf> {
    let filename = format!("{}.md", slug);
    let path = dir.join(&filename);
    let content = serialize_recipe(input, slug);
    fs::write(&path, content)?;
    Ok(path)
}

/// Delete a recipe by slug.
pub fn delete_recipe(dir: &Path, slug: &str) -> std::io::Result<()> {
    let path = find_recipe_path(dir, slug).ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "Recipe not found")
    })?;
    fs::remove_file(path)
}

/// Find the .md file path for a given slug.
fn find_recipe_path(dir: &Path, slug: &str) -> Option<PathBuf> {
    // Try exact match first
    let exact = dir.join(format!("{}.md", slug));
    if exact.exists() {
        return Some(exact);
    }

    // Try case-insensitive or partial match
    for entry in fs::read_dir(dir).ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name().to_str()?.to_string();
        if name.to_lowercase().starts_with(&slug.to_lowercase())
            && name.ends_with(".md")
        {
            return Some(entry.path());
        }
    }

    None
}
