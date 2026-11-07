// Hides the extra console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use serde::Serialize;
use tauri::State;
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

#[derive(Serialize)]
struct Document {
    name: String,
    html: String,
}

/// Files the user chose, through the picker or the command line.
///
/// `open_file` reads only these, so script running in the window could not
/// use the command to read arbitrary files, even if it got past the sanitizer.
#[derive(Default)]
struct ChosenFiles(Mutex<HashSet<PathBuf>>);

impl ChosenFiles {
    fn allow(&self, path: &str) {
        self.0.lock().unwrap().insert(PathBuf::from(path));
    }

    fn contains(&self, path: &str) -> bool {
        self.0.lock().unwrap().contains(Path::new(path))
    }
}

#[tauri::command]
async fn open_file(chosen: State<'_, ChosenFiles>, path: String) -> Result<Document, String> {
    read_document(&chosen, &path)
}

fn read_document(chosen: &ChosenFiles, path: &str) -> Result<Document, String> {
    if !chosen.contains(path) {
        return Err(format!("refused to read {path}: not chosen by the user"));
    }
    let path = Path::new(path);
    let markdown =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    Ok(Document {
        name: path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
        html: mdv_core::to_html(&markdown),
    })
}

// Cancelling the dialog is `Ok(None)`, not an error. The `Result` is only there
// because Tauri requires async commands that borrow `State` to return one.
#[tauri::command]
async fn pick_file(
    app: tauri::AppHandle,
    chosen: State<'_, ChosenFiles>,
) -> Result<Option<String>, ()> {
    let path = app
        .dialog()
        .file()
        .add_filter("Markdown", &["md", "markdown"])
        .blocking_pick_file()
        .map(|path| path.to_string());
    if let Some(path) = &path {
        chosen.allow(path);
    }
    Ok(path)
}

/// Documents are untrusted, so a link may only reach the browser or mail client,
/// never launch local files or other URL handlers. Mail links keep only the
/// address before `?`, so a document cannot add cc, bcc, subject or body; a mail
/// link without an address there (such as `mailto:?to=x`) is refused.
fn external_url(url: &str) -> Option<&str> {
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        Some(url)
    } else if lower.starts_with("mailto:") {
        let target = url.split('?').next().unwrap_or(url);
        (target.len() > "mailto:".len()).then_some(target)
    } else {
        None
    }
}

#[tauri::command]
fn open_external(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let target = external_url(&url).ok_or_else(|| format!("refused to open {url}"))?;
    app.opener()
        .open_url(target, None::<&str>)
        .map_err(|e| e.to_string())
}

/// The file passed on the command line, if any: `mdv-desktop notes.md`.
#[tauri::command]
fn initial_file(chosen: State<'_, ChosenFiles>) -> Option<String> {
    let path = std::env::args().nth(1)?;
    chosen.allow(&path);
    Some(path)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(ChosenFiles::default())
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
    use std::fs;

    use super::{ChosenFiles, external_url, read_document};

    #[test]
    fn allows_only_browser_and_mail_links() {
        for url in ["https://a.dev", "HTTP://a.dev", "mailto:a@b.dev"] {
            assert_eq!(external_url(url), Some(url));
        }
        for url in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "notes.md",
            "smb://x/y",
            "",
        ] {
            assert_eq!(external_url(url), None, "{url}");
        }
    }

    #[test]
    fn mail_links_drop_prefilled_fields() {
        assert_eq!(
            external_url("MAILTO:a@b.dev?cc=x@e.dev&body=hi"),
            Some("MAILTO:a@b.dev")
        );
        assert_eq!(
            external_url("https://a.dev/?q=1"),
            Some("https://a.dev/?q=1")
        );
    }

    #[test]
    fn refuses_mail_links_without_an_address() {
        for url in ["mailto:?to=a@b.dev", "mailto:", "MAILTO:?subject=hi"] {
            assert_eq!(external_url(url), None, "{url}");
        }
    }

    #[test]
    fn reads_only_files_the_user_chose() {
        let chosen = ChosenFiles::default();
        assert!(!chosen.contains("/etc/passwd"));
        chosen.allow("/home/me/notes.md");
        assert!(chosen.contains("/home/me/notes.md"));
        assert!(!chosen.contains("/home/me/other.md"));
    }

    #[test]
    fn open_file_reads_a_file_only_after_the_user_chose_it() {
        let dir = std::env::temp_dir().join(format!("mdv-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("notes.md");
        fs::write(&file, "# Hi").unwrap();
        let path = file.to_str().unwrap();
        let chosen = ChosenFiles::default();

        let refused = read_document(&chosen, path);

        chosen.allow(path);
        let document = read_document(&chosen, path);
        fs::remove_dir_all(&dir).unwrap();

        let error = refused.err().unwrap();
        assert!(error.contains("not chosen by the user"), "{error}");
        let document = document.unwrap();
        assert_eq!(document.name, "notes.md");
        assert!(document.html.contains("<h1>"), "{}", document.html);
    }

    #[test]
    fn mail_links_keep_every_recipient() {
        let url = "mailto:a@b.dev,c@d.dev";
        assert_eq!(external_url(url), Some(url));
        assert_eq!(external_url("mailto:a@b.dev,c@d.dev?cc=x@e.dev"), Some(url));
    }
}
