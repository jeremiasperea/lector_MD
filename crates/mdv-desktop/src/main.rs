// Hides the extra console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{fs, path::Path};

use serde::Serialize;
use tauri_plugin_dialog::DialogExt;

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

/// The file passed on the command line, if any: `mdv-desktop notes.md`.
#[tauri::command]
fn initial_file() -> Option<String> {
    std::env::args().nth(1)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![open_file, pick_file, initial_file])
        .run(tauri::generate_context!())
        .expect("failed to run the application");
}
