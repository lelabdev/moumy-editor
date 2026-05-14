use std::env;
use std::path::PathBuf;

const VERSION: &str = "1.3.0";
const REPO: &str = "lelabdev/moumy-editor";

pub fn current_version() -> String {
    VERSION.to_string()
}

/// Check for updates. Returns Some(message) if an update was applied and restart is needed.
pub async fn check_and_update() -> Option<String> {
    let exe_path = match env::current_exe() {
        Ok(p) => p,
        Err(_) => return None,
    };

    // Check for staged update from previous run
    let staged = exe_path.with_extension("update");
    if staged.exists() {
        println!("📦 Staged update found, applying...");
        // Try to replace current binary with staged one
        match apply_staged_update(&exe_path, &staged) {
            Ok(msg) => return Some(msg),
            Err(e) => {
                eprintln!("⚠️  Failed to apply staged update: {}", e);
                let _ = std::fs::remove_file(&staged);
            }
        }
    }

    // Check GitHub for latest release
    let latest = match fetch_latest_release().await {
        Some(v) => v,
        None => return None,
    };

    if !is_newer(&latest.version, VERSION) {
        return None;
    }

    println!("🆕 New version available: {} (current: {})", latest.version, VERSION);

    // Download the appropriate binary
    let asset_name = if cfg!(windows) {
        "moumy-editor.exe"
    } else {
        "moumy-editor"
    };

    let download_url = match latest.asset_url(asset_name) {
        Some(u) => u,
        None => {
            eprintln!("⚠️  No matching binary found for your platform");
            return None;
        }
    };

    println!("⬇️  Downloading v{}...", latest.version);

    let response = match reqwest::get(&download_url).await {
        Ok(r) if r.status().is_success() => r,
        Ok(r) => {
            eprintln!("⚠️  Download failed: HTTP {}", r.status());
            return None;
        }
        Err(e) => {
            eprintln!("⚠️  Download failed: {}", e);
            return None;
        }
    };

    let bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            eprintln!("⚠️  Download failed: {}", e);
            return None;
        }
    };

    // Write to staged location
    if let Err(e) = std::fs::write(&staged, &bytes) {
        eprintln!("⚠️  Failed to write update: {}", e);
        return None;
    }

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) = std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)) {
            eprintln!("⚠️  Failed to set permissions: {}", e);
            let _ = std::fs::remove_file(&staged);
            return None;
        }
    }

    println!("✅ Update downloaded — restarting...");

    // Restart with the new binary
    let msg = format!("Updated to v{}", latest.version);
    restart_with_update(&exe_path, &staged);
    Some(msg)
}

fn apply_staged_update(exe_path: &PathBuf, staged: &PathBuf) -> Result<String, String> {
    // On Windows, we can't replace a running exe, so we rename the old one
    #[cfg(windows)]
    {
        let old = exe_path.with_extension("old.exe");
        let _ = std::fs::remove_file(&old);
        std::fs::rename(exe_path, &old).map_err(|e| format!("rename old: {}", e))?;
        std::fs::rename(staged, exe_path).map_err(|e| format!("rename new: {}", e))?;
        let _ = std::fs::remove_file(&old);
    }

    #[cfg(not(windows))]
    {
        std::fs::rename(staged, exe_path).map_err(|e| format!("rename: {}", e))?;
    }

    Ok("Update applied".to_string())
}

fn restart_with_update(exe_path: &PathBuf, staged: &PathBuf) {
    #[cfg(windows)]
    {
        // On Windows: launch a cmd that waits for us to exit, then swaps and restarts
        let exe = exe_path.to_string_lossy().to_string();
        let staged_str = staged.to_string_lossy().to_string();
        let script = format!(
            "ping localhost -n 3 >nul & move /y \"{}\" \"{}\" & start \"\" \"{}\"",
            staged_str, exe, exe
        );
        let _ = std::process::Command::new("cmd")
            .args(["/C", &script])
            .spawn();
        std::process::exit(0);
    }

    #[cfg(not(windows))]
    {
        // On Unix: just exec the new binary
        let exe_str = exe_path.to_string_lossy().to_string();
        let _ = std::process::Command::new(&exe_str)
            .spawn();
        std::process::exit(0);
    }
}

struct Release {
    version: String,
    assets: Vec<(String, String)>, // (name, url)
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

/// Compare semver-like versions: "0.9.1" > "0.9.0"
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
