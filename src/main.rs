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

    // Detect site project root (src/data/recettes/ -> project root) and start bun dev
    let site_dir = recipes_dir.parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .filter(|p| p.join("package.json").exists())
        .map(|p| p.to_path_buf());

    let site_url = if let Some(ref sdir) = site_dir {
        let port = env::var("SITE_PORT").unwrap_or_else(|_| "5173".into());
        let site_addr = format!("http://localhost:{}", port);

        match tokio::process::Command::new("bun")
            .args(["dev", "--host", "0.0.0.0", "--port", &port])
            .current_dir(sdir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
        {
            Ok(_) => {
                println!("🍞 Site dev server started at {}", site_addr);
                Some(site_addr)
            }
            Err(e) => {
                eprintln!("⚠️  Could not start bun dev: {}", e);
                eprintln!("   The editor works fine without it — the site preview won't be available.");
                None
            }
        }
    } else {
        println!("ℹ️  No site project found nearby — site preview disabled");
        None
    };

    let state = Arc::new(AppState { recipes_dir, site_url });
    let app = server::router(state);

    let addr = "0.0.0.0:3210";
    let editor_url = "http://localhost:3210";
    println!("🧁 Moumy Editor running at {}", editor_url);

    // Auto-open browser
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(editor_url).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(editor_url).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("cmd").args(["/C", "start", editor_url]).spawn();
    }

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
