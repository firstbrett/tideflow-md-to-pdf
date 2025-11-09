/// Image operation commands: importing and managing images
use crate::utils;
use base64::Engine;
use std::fs;
use std::path::Path;
use tauri::AppHandle;
use uuid::Uuid;

#[tauri::command]
pub async fn import_image(
    app_handle: AppHandle,
    image_data: &str,
    file_name: Option<String>,
) -> Result<String, String> {
    // Extract base64 data (remove data:image/png;base64, prefix)
    let base64_data = if image_data.contains("base64,") {
        image_data.split("base64,").nth(1).unwrap_or(image_data)
    } else {
        image_data
    };

    // Decode base64 image data
    let image_bytes = base64::engine::general_purpose::STANDARD
        .decode(base64_data)
        .map_err(|e| format!("Failed to decode image: {}", e))?;

    // Get assets directory
    let assets_dir = utils::get_assets_dir(&app_handle).map_err(|e| e.to_string())?;

    // Ensure assets directory exists
    fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;

    // Generate unique filename if not provided
    let filename = match file_name {
        Some(name) => utils::sanitize_filename(&name),
        None => {
            let uuid = Uuid::new_v4();
            format!("image-{}.png", uuid)
        }
    };

    // Construct full path
    let image_path = assets_dir.join(&filename);

    // Write image to file
    fs::write(&image_path, image_bytes).map_err(|e| e.to_string())?;

    // Return relative path for Markdown insertion
    Ok(format!("assets/{}", filename))
}

/// Import an image by copying it from a local filesystem path into the app's assets directory.
/// Returns a relative path like "assets/<filename>" suitable for Markdown insertion.
#[tauri::command]
pub async fn import_image_from_path(
    app_handle: AppHandle,
    source_path: &str,
) -> Result<String, String> {
    let src = Path::new(source_path);
    if !src.exists() {
        return Err(format!("Source image does not exist: {}", source_path));
    }

    // Read source bytes
    let image_bytes = fs::read(src).map_err(|e| format!("Failed to read image: {}", e))?;

    // Determine destination directory and filename
    let assets_dir = utils::get_assets_dir(&app_handle).map_err(|e| e.to_string())?;
    fs::create_dir_all(&assets_dir).map_err(|e| e.to_string())?;

    let orig_name = src
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("image.png");
    let mut base = utils::sanitize_filename(orig_name);

    // Ensure unique filename to avoid accidental overwrite
    let mut dest_path = assets_dir.join(&base);
    if dest_path.exists() {
        // Insert a short UUID before extension
        let (stem, ext) = match dest_path.file_stem().and_then(|s| s.to_str()) {
            Some(stem) => {
                let ext = dest_path.extension().and_then(|e| e.to_str()).unwrap_or("");
                (stem.to_string(), ext.to_string())
            }
            None => ("image".to_string(), "".to_string()),
        };
        let short = Uuid::new_v4().to_string();
        let short = &short[0..8];
        base = if ext.is_empty() {
            format!("{}-{}", stem, short)
        } else {
            format!("{}-{}.{}", stem, short, ext)
        };
        dest_path = assets_dir.join(&base);
    }

    fs::write(&dest_path, image_bytes).map_err(|e| e.to_string())?;

    Ok(format!("assets/{}", base))
}
