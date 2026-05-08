mod recipe;
mod server;
mod store;

use std::env;
use std::path::PathBuf;
use std::sync::Arc;

use server::AppState;

#[tokio::main]
async fn main() {
    let dir = env::current_dir().expect("Cannot get current directory");

    // Try: RECIPES_DIR env > ./src/data/recettes/ > current dir
    let recipes_dir = if let Ok(custom) = env::var("RECIPES_DIR") {
        PathBuf::from(custom)
    } else {
        let default = dir.join("src/data/recettes");
        if default.exists() {
            default
        } else {
            dir
        }
    };

    if !recipes_dir.exists() {
        eprintln!("Error: recipes directory not found: {}", recipes_dir.display());
        eprintln!("Run this binary from the recipes directory or set RECIPES_DIR");
        std::process::exit(1);
    }

    println!("📂 Recipes directory: {}", recipes_dir.display());

    let state = Arc::new(AppState { recipes_dir });
    let app = server::router(state);

    let addr = "0.0.0.0:3210";
    println!("🧁 Moumy Editor running at http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
