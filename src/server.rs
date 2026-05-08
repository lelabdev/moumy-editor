use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, Json};
use axum::routing::{delete, get, post};
use axum::Router;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

use crate::recipe::{RecipeInput, Recipe};
use crate::store;

pub struct AppState {
    pub recipes_dir: PathBuf,
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/api/recipes", get(list_recipes))
        .route("/api/recipes/{slug}", get(get_recipe))
        .route("/api/recipes", post(create_recipe))
        .route("/api/recipes/{slug}", post(update_recipe))
        .route("/api/recipes/{slug}", delete(delete_recipe))
        .with_state(state)
}

async fn index() -> Html<&'static str> {
    Html(include_str!("frontend/index.html"))
}

async fn list_recipes(State(state): State<Arc<AppState>>) -> Json<Value> {
    let recipes = store::list_recipes(&state.recipes_dir);
    let list: Vec<Value> = recipes
        .iter()
        .map(|r| {
            json!({
                "slug": r.slug,
                "title": r.frontmatter.title,
                "category": r.frontmatter.category,
                "categoryLabel": r.frontmatter.category_label,
                "difficulty": r.frontmatter.difficulty,
                "excerpt": r.frontmatter.excerpt,
            })
        })
        .collect();
    Json(json!({ "recipes": list }))
}

async fn get_recipe(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let recipe = store::get_recipe(&state.recipes_dir, &slug)
        .ok_or(StatusCode::NOT_FOUND)?;

    Ok(Json(recipe_to_json(&recipe)))
}

async fn create_recipe(
    State(state): State<Arc<AppState>>,
    Json(input): Json<RecipeInput>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let slug = input.slug.clone().unwrap_or_else(|| slugify(&input.title));

    if store::get_recipe(&state.recipes_dir, &slug).is_some() {
        return Err((
            StatusCode::CONFLICT,
            format!("Recipe '{}' already exists", slug),
        ));
    }

    store::save_recipe(&state.recipes_dir, &slug, &input)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let recipe = store::get_recipe(&state.recipes_dir, &slug)
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Failed to read back".into()))?;

    Ok(Json(recipe_to_json(&recipe)))
}

async fn update_recipe(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
    Json(input): Json<RecipeInput>,
) -> Result<Json<Value>, (StatusCode, String)> {
    store::save_recipe(&state.recipes_dir, &slug, &input)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let recipe = store::get_recipe(&state.recipes_dir, &slug)
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Failed to read back".into()))?;

    Ok(Json(recipe_to_json(&recipe)))
}

async fn delete_recipe(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    store::delete_recipe(&state.recipes_dir, &slug)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(json!({ "deleted": slug })))
}

fn recipe_to_json(recipe: &Recipe) -> Value {
    // Split ingredients2: first item = title, rest = items
    let (i2_title, i2_items): (Option<String>, Option<Vec<String>>) = match &recipe.frontmatter.ingredients2 {
        Some(items) if !items.is_empty() => {
            let title = Some(items[0].clone());
            let rest = if items.len() > 1 { Some(items[1..].to_vec()) } else { None };
            (title, rest)
        }
        _ => (None, None),
    };

    json!({
        "slug": recipe.slug,
        "filename": recipe.filename,
        "title": recipe.frontmatter.title,
        "category": recipe.frontmatter.category,
        "categoryLabel": recipe.frontmatter.category_label,
        "excerpt": recipe.frontmatter.excerpt,
        "prepTime": recipe.frontmatter.prep_time,
        "cookTime": recipe.frontmatter.cook_time,
        "servings": recipe.frontmatter.servings,
        "difficulty": recipe.frontmatter.difficulty,
        "ingredients": recipe.frontmatter.ingredients,
        "ingredients2Title": i2_title,
        "ingredients2": i2_items,
        "notes": recipe.frontmatter.notes,
        "steps": recipe.steps,
    })
}

fn slugify(title: &str) -> String {
    // Simple slug: take the title, make it filesystem-safe
    let slug: String = title
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c
            } else if c == ' ' || c == '-' || c == '_' {
                '-'
            } else {
                // Strip accents roughly
                match c {
                    'à' | 'â' | 'ä' => 'a',
                    'é' | 'è' | 'ê' | 'ë' => 'e',
                    'î' | 'ï' => 'i',
                    'ô' | 'ö' => 'o',
                    'ù' | 'û' | 'ü' => 'u',
                    'ç' => 'c',
                    _ => '-',
                }
            }
        })
        .collect();

    // Collapse multiple dashes
    let mut result = String::new();
    let mut prev_dash = false;
    for c in slug.chars() {
        if c == '-' {
            if !prev_dash {
                result.push(c);
            }
            prev_dash = true;
        } else {
            result.push(c);
            prev_dash = false;
        }
    }

    result.trim_matches('-').to_string()
}
