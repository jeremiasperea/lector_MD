// Hides the extra console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{fs, path::Path};

use serde::Serialize;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

#[derive(Serialize)]
struct Document {
    name: String,
    html: String,
}

#[tauri::command]
async fn open_file(path: String) -> Result<Document, String> {
    let path = Path::new(&path);
    let markdown =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    Ok(Document {
        name: path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
        html: mdv_core::to_html(&markdown),
    })
}

#[tauri::command]
async fn pick_file(app: tauri::AppHandle) -> Option<String> {
    app.dialog()
        .file()
        .add_filter("Markdown", &["md", "markdown"])
        .blocking_pick_file()
        .map(|path| path.to_string())
}

/// Documents are untrusted, so a link may only reach the browser or mail client,
/// never launch local files or other URL handlers.
fn is_external_url(url: &str) -> bool {
    let url = url.to_ascii_lowercase();
    ["http://", "https://", "mailto:"]
        .iter()
        .any(|scheme| url.starts_with(scheme))
}

#[tauri::command]
fn open_external(app: tauri::AppHandle, url: String) -> Result<(), String> {
    if !is_external_url(&url) {
        return Err(format!("refused to open {url}"));
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| e.to_string())
}

/// The file passed on the command line, if any: `mdv-desktop notes.md`.
#[tauri::command]
fn initial_file() -> Option<String> {
    std::env::args().nth(1)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            open_file,
            pick_file,
            open_external,
            initial_file
        ])
        .run(tauri::generate_context!())
        .expect("failed to run the application");
}

#[cfg(test)]
mod tests {
    use super::is_external_url;

    #[test]
    fn allows_only_browser_and_mail_links() {
        for url in ["https://a.dev", "HTTP://a.dev", "mailto:a@b.dev"] {
            assert!(is_external_url(url), "{url}");
        }
        for url in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "notes.md",
            "smb://x/y",
            "",
        ] {
            assert!(!is_external_url(url), "{url}");
        }
    }
}
