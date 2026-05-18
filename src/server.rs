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

    /// Content directory: src/data/content/ relative to project root
    /// recipes_dir = <project>/src/data/recettes/
    /// project_root = recipes_dir.parent().parent().parent()
    pub fn content_dir(&self) -> PathBuf {
        self.recipes_dir.parent() // src/data/
            .and_then(|p| p.parent()) // src/
            .and_then(|p| p.parent()) // project root
            .map(|p| p.join("src/data/content"))
            .unwrap_or_else(|| self.recipes_dir.join("../../content"))
    }
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/styles.css", get(styles))
        .route("/api/recipes", get(list_recipes))
        .route("/api/recipes/{slug}", get(get_recipe))
        .route("/api/recipes", post(create_recipe))
        .route("/api/recipes/{slug}", post(update_recipe))
        .route("/api/recipes/{slug}", delete(delete_recipe))
        .route("/api/images/{slug}", get(get_image))
        .route("/api/images-group/{slug}", get(list_images_group))
        .route("/api/orphan-images", get(list_orphan_images))
        .route("/api/site-url", get(get_site_url))
        .route("/api/update-check", get(check_update))
        .route("/api/ocr/{slug}", post(ocr_image))
        .route("/api/ocr-status", get(ocr_status))
        .route("/api/git-status", get(git_status))
        .route("/api/git-push", post(git_push))
        .route("/api/content", get(list_content))
        .route("/api/content/{path}", get(get_content))
        .route("/api/content/{path}", post(save_content))
        .with_state(state)
}

async fn index() -> Html<&'static str> {
    Html(include_str!("frontend/index.html"))
}

async fn styles() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css")],
        include_str!("frontend/styles.css"),
    )
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

/// List all images matching a base slug (e.g. "EdM-Abri01" matches EdM-Abri01.jpg, EdM-Abri01_A.jpg, ...)
async fn list_images_group(
    State(state): State<Arc<AppState>>,
    Path(slug): Path<String>,
) -> Json<Value> {
    let img_dir = state.img_dir();
    let base_slug = slug.split('_').next().unwrap_or(&slug).to_lowercase();

    let mut matching: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&img_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                let stem = name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name);
                let stem_base = stem.split('_').next().unwrap_or(stem);
                if stem_base.to_lowercase() == base_slug {
                    matching.push(stem.to_string());
                }
            }
        }
    }

    matching.sort();
    Json(json!({ "images": matching }))
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

    // Collect all recipe slugs + their base slugs + sourceImage
    let recipes = store::list_recipes(&state.recipes_dir);
    let mut used_images: std::collections::HashSet<String> = std::collections::HashSet::new();
    for r in &recipes {
        used_images.insert(r.slug.clone());
        if let Some(base) = r.slug.split('_').next() {
            used_images.insert(base.to_string());
        }
        // Also count sourceImage as used
        if let Some(ref src) = r.frontmatter.source_image {
            used_images.insert(src.clone());
            if let Some(base) = src.split('_').next() {
                used_images.insert(base.to_string());
            }
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

async fn check_update() -> Json<Value> {
    let current = crate::updater::current_version();
    match crate::updater::check_latest().await {
        Some(latest) => Json(json!({ "current": current, "latest": latest, "updateAvailable": true })),
        None => Json(json!({ "current": current, "latest": current, "updateAvailable": false })),
    }
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
            let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
            let count = lines.len();

            // Extract dirty slugs from file paths like "M  src/data/recettes/crepes.md"
            let dirty_slugs: Vec<String> = lines.iter()
                .filter_map(|line| {
                    let path = line.trim_start_matches(|c: char| c.is_uppercase() || c == ' ' || c == '?');
                    path.strip_prefix("src/data/recettes/")
                        .and_then(|f| f.strip_suffix(".md"))
                        .map(|s| s.to_string())
                })
                .collect();

            Json(json!({ "changes": count, "dirtySlugs": dirty_slugs }))
        }
        Err(e) => Json(json!({ "changes": 0, "dirtySlugs": [], "error": e.to_string() }))
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

// --- Content editor endpoints ---

/// Sanitize a user-provided path: strip `..` components and ensure it stays within base_dir.
/// Returns the resolved safe path, or an error.
fn sanitize_content_path(base_dir: &std::path::Path, raw: &str) -> Result<PathBuf, StatusCode> {
    // Split into components and reject any that are ".." or empty
    let mut safe_parts: Vec<std::ffi::OsString> = Vec::new();
    for comp in std::path::Path::new(raw).components() {
        match comp {
            std::path::Component::CurDir => {},
            std::path::Component::Normal(c) => safe_parts.push(c.to_owned()),
            _ => return Err(StatusCode::BAD_REQUEST), // rejects .. and root
        }
    }

    let full_path = base_dir.join(std::path::PathBuf::from_iter(safe_parts));

    // Verify the resolved path stays within base_dir
    let canonical_base = base_dir.canonicalize().unwrap_or_else(|_| base_dir.to_path_buf());
    if full_path.starts_with(&canonical_base) || full_path.parent().map_or(false, |p| p.starts_with(&canonical_base)) {
        Ok(full_path)
    } else {
        Err(StatusCode::BAD_REQUEST)
    }
}

/// Recursively list files in the content directory
async fn list_content(State(state): State<Arc<AppState>>) -> Result<Json<Value>, StatusCode> {
    let content_dir = state.content_dir();
    if !content_dir.exists() {
        return Ok(Json(json!({ "files": [] })));
    }

    let mut files: Vec<Value> = Vec::new();
    collect_files(&content_dir, &content_dir, &mut files)?;
    files.sort_by(|a, b| {
        let pa = a["path"].as_str().unwrap_or("");
        let pb = b["path"].as_str().unwrap_or("");
        pa.cmp(pb)
    });
    Ok(Json(json!({ "files": files })))
}

fn collect_files(base: &std::path::Path, dir: &std::path::Path, files: &mut Vec<Value>) -> Result<(), StatusCode> {
    for entry in std::fs::read_dir(dir).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)? {
        let entry = entry.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(base, &path, files)?;
        } else {
            let relative = path.strip_prefix(base).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            files.push(json!({
                "path": relative.to_string_lossy().to_string(),
                "name": entry.file_name().to_string_lossy().to_string(),
            }));
        }
    }
    Ok(())
}

/// Read a content file
async fn get_content(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    let content_dir = state.content_dir();
    let full_path = sanitize_content_path(&content_dir, &path)?;

    if !full_path.exists() {
        return Err(StatusCode::NOT_FOUND);
    }

    let content = std::fs::read_to_string(&full_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "path": path, "content": content })))
}

/// Write a content file
async fn save_content(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let content_dir = state.content_dir();
    let full_path = sanitize_content_path(&content_dir, &path)?;

    let content = body.get("content")
        .and_then(|v| v.as_str())
        .ok_or(StatusCode::BAD_REQUEST)?;

    // Ensure parent directory exists
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    std::fs::write(&full_path, content).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(json!({ "ok": true })))
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
        "legende": recipe.frontmatter.legende,
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
