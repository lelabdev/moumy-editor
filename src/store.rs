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
/// Uses custom YAML formatting to match Obsidian's style (no serde_yaml).
pub fn serialize_recipe(input: &RecipeInput, slug: &str) -> String {
    let mut yaml = String::new();

    // title — always present, quote if it contains special YAML chars
    yaml.push_str(&format!("title: {}\n", yaml_value(&input.title)));
    yaml.push_str(&format!("slug: {}\n", slug));
    yaml.push_str(&format!("manuscript: {}\n", yaml_value("")));

    yaml.push_str(&format!("category: {}\n", yaml_value(&input.category)));

    // excerpt
    match &input.excerpt {
        Some(v) if !v.is_empty() => yaml.push_str(&format!("excerpt: {}\n", yaml_value(v))),
        _ => yaml.push_str("excerpt:\n"),
    }

    // Numeric-like fields: no quotes
    yaml.push_str(&format!(
        "prepTime: {}\n",
        yaml_plain(&input.prep_time)
    ));
    yaml.push_str(&format!(
        "cookTime: {}\n",
        yaml_plain(&input.cook_time)
    ));
    yaml.push_str(&format!(
        "servings: {}\n",
        yaml_plain(&input.servings)
    ));

    // difficulty
    match &input.difficulty {
        Some(v) if !v.is_empty() => yaml.push_str(&format!("difficulty: {}\n", yaml_value(v))),
        _ => yaml.push_str("difficulty:\n"),
    }

    // ingredients — indented list
    yaml.push_str("ingredients:\n");
    for ing in &input.ingredients {
        if !ing.is_empty() {
            yaml.push_str(&format!("  - {}\n", yaml_value(ing)));
        }
    }

    // ingredients2Title
    match &input.ingredients2_title {
        Some(v) if !v.is_empty() => {
            yaml.push_str(&format!("ingredients2Title: {}\n", yaml_value(v)))
        }
        _ => yaml.push_str("ingredients2Title:\n"),
    }

    // ingredients2 — indented list
    match &input.ingredients2 {
        Some(list) if !list.is_empty() => {
            yaml.push_str("ingredients2:\n");
            for ing in list {
                if !ing.is_empty() {
                    yaml.push_str(&format!("  - {}\n", yaml_value(ing)));
                }
            }
        }
        _ => yaml.push_str("ingredients2:\n"),
    }

    // notes
    match &input.notes {
        Some(v) if !v.is_empty() => yaml.push_str(&format!("notes: {}\n", yaml_value(v))),
        _ => yaml.push_str("notes:\n"),
    }

    // legende
    match &input.legende {
        Some(v) if !v.is_empty() => yaml.push_str(&format!("legende: {}\n", yaml_value(v))),
        _ => {} // omit if empty — optional field
    }

    // sourceImage
    match &input.source_image {
        Some(v) if !v.is_empty() => {
            yaml.push_str(&format!("sourceImage: {}\n", yaml_value(v)))
        }
        _ => yaml.push_str("sourceImage:\n"),
    }

    // Steps
    let steps: String = input
        .steps
        .iter()
        .enumerate()
        .map(|(i, step)| format!("{}. {}", i + 1, step))
        .collect::<Vec<_>>()
        .join("\n\n");

    format!("---\n{}---\n\n{}\n", yaml, steps)
}

/// Format a string as a YAML value.
/// Empty → "\"\"", needs quoting → double-quoted, otherwise bare.
fn yaml_value(s: &str) -> String {
    if s.is_empty() {
        return "\"\"".to_string();
    }
    // Quote if it contains YAML-special characters
    if s.contains(':')
        || s.contains('#')
        || s.contains('"')
        || s.contains('\'')
        || s.contains('\n')
        || s.contains('{')
        || s.contains('}')
        || s.contains('[')
        || s.contains(']')
        || s.contains(',')
        || s.starts_with('-')
        || s.starts_with('*')
        || s.starts_with('&')
        || s.starts_with('!')
        || s.starts_with('%')
        || s.starts_with('@')
        || s.starts_with('`')
        || s.starts_with(' ')
        || s.ends_with(' ')
        // Looks like a boolean or null
        || matches!(s.to_lowercase().as_str(), "true" | "false" | "yes" | "no" | "null")
        // Looks like a number (don't want YAML to parse as int/float)
        || s.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-')
    {
        // Double-quote and escape internal double quotes
        format!("\"{}\"", s.replace('"', "\\\""))
    } else {
        s.to_string()
    }
}

/// Format an optional string as a plain YAML value (no quotes for numbers).
fn yaml_plain(s: &Option<String>) -> String {
    match s {
        Some(v) if !v.is_empty() => v.to_string(),
        _ => String::new(),
    }
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
