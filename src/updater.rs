use std::env;
use std::path::PathBuf;

const VERSION: &str = "1.6.1";
const REPO: &str = "lelabdev/moumy-editor";

pub fn current_version() -> String {
    VERSION.to_string()
}

/// Check for updates and apply if available.
/// Returns true if we restarted (caller should exit without launching browser).
pub async fn check_and_update() -> bool {
    let exe_path = match env::current_exe() {
        Ok(p) => p,
        Err(_) => return false,
    };

    // 1. Check for staged update from previous run
    let staged = exe_path.with_extension("update");
    if staged.exists() {
        println!("📦 Mise à jour en attente, application...");
        match apply_staged_update(&exe_path, &staged) {
            Ok(()) => {
                let _ = std::fs::remove_file(&staged);
                println!("✅ Mise à jour appliquée ! Redémarrage...");
                restart(&exe_path);
                return true;
            }
            Err(e) => {
                eprintln!("⚠️  Échec de la mise à jour: {}", e);
                let _ = std::fs::remove_file(&staged);
            }
        }
    }

    // 2. Check GitHub for latest release
    let latest = match fetch_latest_release().await {
        Some(v) => v,
        None => return false,
    };

    if !is_newer(&latest.version, VERSION) {
        return false;
    }

    println!("🆕 Nouvelle version disponible: v{} (actuelle: v{})", latest.version, VERSION);

    // Download the appropriate binary
    let asset_name = if cfg!(windows) {
        "moumy-editor.exe"
    } else {
        "moumy-editor"
    };

    let download_url = match latest.asset_url(asset_name) {
        Some(u) => u,
        None => {
            eprintln!("⚠️  Aucun binaire trouvé pour votre plateforme");
            return false;
        }
    };

    println!("⬇️  Téléchargement de la v{}...", latest.version);

    let response = match reqwest::get(&download_url).await {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            eprintln!("⚠️  Échec du téléchargement: HTTP {}", r.status());
            return false;
        }
        Err(e) => {
            eprintln!("⚠️  Échec du téléchargement: {}", e);
            return false;
        }
    };

    let bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("⚠️  Échec du téléchargement: {}", e);
            return false;
        }
    };

    // Write to staged location
    if let Err(e) = std::fs::write(&staged, &bytes) {
        eprintln!("⚠️  Échec de l'écriture: {}", e);
        return false;
    }

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)) {
            eprintln!("⚠️  Échec des permissions: {}", e);
            let _ = std::fs::remove_file(&staged);
            return false;
        }
    }

    println!("✅ Téléchargement terminé. Redémarrage pour appliquer...");

    // Swap and restart
    match apply_staged_update(&exe_path, &staged) {
        Ok(()) => {
            let _ = std::fs::remove_file(&staged);
            restart(&exe_path);
            return true;
        }
        Err(e) => {
            eprintln!("⚠️  Échec de l'application: {}", e);
            // Keep .update file — will retry on next launch
            println!("ℹ️  La mise à jour sera appliquée au prochain démarrage");
            false
        }
    }
}

fn apply_staged_update(exe_path: &PathBuf, staged: &PathBuf) -> Result<(), String> {
    #[cfg(windows)]
    {
        // On Windows: rename running exe (allowed), then move new one in place
        let old = exe_path.with_extension("old");
        let _ = std::fs::remove_file(&old);
        std::fs::rename(exe_path, &old).map_err(|e| format!("rename old: {}", e))?;
        std::fs::rename(staged, exe_path).map_err(|e| format!("rename new: {}", e))?;
        // Clean up old file (best effort)
        let _ = std::fs::remove_file(&old);
    }

    #[cfg(not(windows))]
    {
        std::fs::rename(staged, exe_path).map_err(|e| format!("rename: {}", e))?;
    }

    Ok(())
}

fn restart(exe_path: &PathBuf) {
    let exe_str = exe_path.to_string_lossy().to_string();
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "", &exe_str])
            .spawn();
    }

    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new(&exe_str).spawn();
    }

    std::process::exit(0);
}

struct Release {
    version: String,
    assets: Vec<(String, String)>,
}

impl Release {
    fn asset_url(&self, name: &str) -> Option<String> {
        self.assets.iter()
            .find(|(n, _)| n == name)
            .map(|(_, u)| u.clone())
    }
}

async fn fetch_latest_release() -> Option<Release> {
    let url = format!("https://api.github.com/repos/{}/releases/latest", REPO);
    let client = reqwest::Client::new();
    let resp = client.get(&url)
        .header("User-Agent", "moumy-editor")
        .send()
        .await
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    let json: serde_json::Value = resp.json().await.ok()?;
    let tag = json.get("tag_name")?.as_str()?.to_string();
    let version = tag.strip_prefix('v').unwrap_or(&tag).to_string();

    let assets = json.get("assets")?
        .as_array()?
        .iter()
        .filter_map(|a| {
            let name = a.get("name")?.as_str()?.to_string();
            let url = a.get("browser_download_url")?.as_str()?.to_string();
            Some((name, url))
        })
        .collect();

    Some(Release { version, assets })
}

/// Check if a newer version is available (without downloading)
pub async fn check_latest() -> Option<String> {
    let latest = fetch_latest_release().await?;
    if is_newer(&latest.version, VERSION) {
        Some(latest.version)
    } else {
        None
    }
}

fn is_newer(remote: &str, local: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|p| p.parse().ok())
            .collect()
    };
    let r = parse(remote);
    let l = parse(local);
    r > l
}
