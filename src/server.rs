use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{Html, IntoResponse, Json};
use axum::routing::{delete, get, post};
use axum::Router;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

use crate::recipe::{RecipeInput, Recipe};
use crate::store;

pub struct AppState {
    pub recipes_dir: PathBuf,
    pub site_url: Option<String>,
    pub mistral_api_key: Option<String>,
}

impl AppState {
    /// Image directory: sibling `img/` folder next to the recettes folder
    pub fn img_dir(&self) -> PathBuf {
        self.recipes_dir.parent()
            .map(|p| p.join("img"))
            .unwrap_or_else(|| self.recipes_dir.join("../img"))
    }
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/api/recipes", get(list_recipes))
        .route("/api/recipes/{slug}", get(get_recipe))
        .route("/api/recipes", post(create_recipe))
        .route("/api/recipes/{slug}", post(update_recipe))
        .route("/api/recipes/{slug}", delete(delete_recipe))
        .route("/api/images/{slug}", get(get_image))
        .route("/api/orphan-images", get(list_orphan_images))
        .route("/api/site-url", get(get_site_url))
        .route("/api/ocr/{slug}", post(ocr_image))
        .route("/api/ocr-status", get(ocr_status))
        .route("/api/git-status", get(git_status))
        .route("/api/git-push", post(git_push))
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

async fn get_image(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let img_dir = state.img_dir();

    // Try candidates: full slug first, then base slug (before _)
    let base_slug = slug.split('_').next().unwrap_or(&slug);
    let candidates = if base_slug != slug {
        vec![slug.as_str(), base_slug]
    } else {
        vec![slug.as_str()]
    };

    for candidate in candidates {
        for ext in &["jpg", "jpeg", "png", "webp"] {
            let path = img_dir.join(format!("{}.{}", candidate, ext));
            if path.exists() {
                let bytes = std::fs::read(&path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                let mime = match *ext {
                    "png" => "image/png",
                    "webp" => "image/webp",
                    _ => "image/jpeg",
                };
                return Ok((
                    [(header::CONTENT_TYPE, mime)],
                    Body::from(bytes),
                ));
            }
        }
    }
    Err(StatusCode::NOT_FOUND)
}

async fn list_orphan_images(State(state): State<Arc<AppState>>) -> Json<Value> {
    let img_dir = state.img_dir();

    // Collect all image basenames (without extension)
    let mut image_stems: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&img_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name);
                image_stems.push(stem.to_string());
            }
        }
    }

    // Collect all recipe slugs + their base slugs
    let recipes = store::list_recipes(&state.recipes_dir);
    let mut used_images: std::collections::HashSet<String> = std::collections::HashSet::new();
    for r in &recipes {
        used_images.insert(r.slug.clone());
        if let Some(base) = r.slug.split('_').next() {
            used_images.insert(base.to_string());
        }
    }

    // Orphan = image stem not used by any recipe
    let orphans: Vec<Value> = image_stems
        .iter()
        .filter(|stem| !used_images.contains(*stem))
        .map(|stem| json!({ "slug": stem }))
        .collect();

    Json(json!({ "orphanImages": orphans }))
}

async fn get_site_url(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({
        "siteUrl": state.site_url,
    }))
}

async fn git_status() -> Json<Value> {
    let dir = std::env::current_dir().unwrap_or_default();

    let output = std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&dir)
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let count = stdout.lines().filter(|l| !l.trim().is_empty()).count();
            Json(json!({ "changes": count }))
        }
        Err(e) => Json(json!({ "changes": 0, "error": e.to_string() }))
    }
}

async fn git_push() -> Json<Value> {
    let dir = std::env::current_dir().unwrap_or_default();

    // git add -A
    let add = std::process::Command::new("git")
        .args(["add", "-A"])
        .current_dir(&dir)
        .output();

    if let Err(e) = add {
        return Json(json!({ "error": format!("git add failed: {}", e) }));
    }

    // git commit
    let commit = std::process::Command::new("git")
        .args(["commit", "-m", "editor: recipe changes"])
        .current_dir(&dir)
        .output();

    let nothing_to_commit = match &commit {
        Ok(out) => {
            let combined = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
            combined.contains("nothing to commit")
        }
        Err(_) => false,
    };

    if let Err(e) = commit {
        return Json(json!({ "error": format!("git commit failed: {}", e) }));
    }

    if nothing_to_commit {
        return Json(json!({ "ok": true, "message": "Rien à pousser" }));
    }

    // git push
    let push = std::process::Command::new("git")
        .args(["push"])
        .current_dir(&dir)
        .output();

    match push {
        Ok(out) if out.status.success() => {
            Json(json!({ "ok": true, "message": "Pushé sur GitHub ✓" }))
        }
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            Json(json!({ "error": format!("push failed: {}", stderr) }))
        }
        Err(e) => Json(json!({ "error": format!("push failed: {}", e) }))
    }
}

async fn ocr_status(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({
        "available": state.mistral_api_key.is_some(),
    }))
}

async fn ocr_image(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let api_key = state.mistral_api_key.as_ref().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({ "error": "MISTRAL_API_KEY not configured" })),
    ))?;

    // Read the image file
    let img_dir = state.img_dir();
    let base_slug = slug.split('_').next().unwrap_or(&slug);
    let candidates = if base_slug != slug {
        vec![slug.as_str(), base_slug]
    } else {
        vec![slug.as_str()]
    };

    let mut image_bytes = None;
    let mut mime = "image/jpeg";
    for candidate in &candidates {
        for ext in &["jpg", "jpeg", "png", "webp"] {
            let path = img_dir.join(format!("{}.{}", candidate, ext));
            if path.exists() {
                match std::fs::read(&path) {
                    Ok(bytes) => {
                        mime = match *ext {
                            "png" => "image/png",
                            "webp" => "image/webp",
                            _ => "image/jpeg",
                        };
                        image_bytes = Some(bytes);
                        break;
                    }
                    Err(_) => continue,
                }
            }
        }
        if image_bytes.is_some() { break; }
    }

    let image_bytes = image_bytes.ok_or((
        StatusCode::NOT_FOUND,
        Json(json!({ "error": "Image not found" })),
    ))?;

    let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &image_bytes);
    let data_uri = format!("data:{};base64,{}", mime, b64);

    // Call Mistral OCR API
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "model": "mistral-ocr-latest",
        "document": {
            "type": "image_url",
            "image_url": data_uri,
        }
    });

    let resp = client
        .post("https://api.mistral.ai/v1/ocr")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            (StatusCode::BAD_GATEWAY, Json(json!({ "error": format!("Mistral request failed: {}", e) })))
        })?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err((
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": format!("Mistral API error {}: {}", status, text) })),
        ));
    }

    let result: serde_json::Value = resp.json().await.map_err(|e| {
        (StatusCode::BAD_GATEWAY, Json(json!({ "error": format!("Parse error: {}", e) })))
    })?;

    // Extract markdown text from pages
    let pages = result.get("pages").and_then(|p| p.as_array()).cloned().unwrap_or_default();
    let text: String = pages
        .iter()
        .filter_map(|p| p.get("markdown").and_then(|m| m.as_str()))
        .collect::<Vec<_>>()
        .join("\n\n");

    Ok(Json(json!({ "text": text })))
}

fn recipe_to_json(recipe: &Recipe) -> Value {
    json!({
        "slug": recipe.slug,
        "filename": recipe.filename,
        "title": recipe.frontmatter.title,
        "category": recipe.frontmatter.category,
        "excerpt": recipe.frontmatter.excerpt,
        "prepTime": recipe.frontmatter.prep_time,
        "cookTime": recipe.frontmatter.cook_time,
        "servings": recipe.frontmatter.servings,
        "difficulty": recipe.frontmatter.difficulty,
        "ingredients": recipe.frontmatter.ingredients,
        "ingredients2Title": recipe.frontmatter.ingredients2_title,
        "ingredients2": recipe.frontmatter.ingredients2,
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
