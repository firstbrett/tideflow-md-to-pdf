//! Path resolution utilities for app directories.

use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Get the app's base directory
pub fn get_app_dir(app_handle: &AppHandle) -> Result<PathBuf> {
    let app_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| anyhow!("Failed to get app directory: {}", e))?;

    Ok(app_dir)
}

/// Get the content directory
pub fn get_content_dir(app_handle: &AppHandle) -> Result<PathBuf> {
    let app_dir = get_app_dir(app_handle)?;
    let content_dir = app_dir.join("content");

    if !content_dir.exists() {
        fs::create_dir_all(&content_dir)?;
    }

    Ok(content_dir)
}

/// Get the assets directory
pub fn get_assets_dir(app_handle: &AppHandle) -> Result<PathBuf> {
    let content_dir = get_content_dir(app_handle)?;
    let assets_dir = content_dir.join("assets");

    if !assets_dir.exists() {
        fs::create_dir_all(&assets_dir)?;
    }

    Ok(assets_dir)
}

/// Get the templates directory
pub fn get_templates_dir(app_handle: &AppHandle) -> Result<PathBuf> {
    let app_dir = get_app_dir(app_handle)?;
    let templates_dir = app_dir.join("templates");

    if !templates_dir.exists() {
        fs::create_dir_all(&templates_dir)?;
    }

    Ok(templates_dir)
}

/// Get the styles directory
pub fn get_styles_dir(app_handle: &AppHandle) -> Result<PathBuf> {
    let app_dir = get_app_dir(app_handle)?;
    let styles_dir = app_dir.join("styles");

    if !styles_dir.exists() {
        fs::create_dir_all(&styles_dir)?;
    }

    Ok(styles_dir)
}

/// Get the Typst binary path based on platform
pub fn get_typst_path(app_handle: &AppHandle) -> Result<PathBuf> {
    // First, try to find typst on the system PATH
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let typst_path = if cfg!(target_os = "windows") {
                dir.join("typst.exe")
            } else {
                dir.join("typst")
            };

            if typst_path.exists() {
                return Ok(typst_path);
            }
        }
    }

    // On Unix-like systems, try `which typst` as an additional check (covers AppImage environments)
    #[cfg(unix)]
    {
        if let Ok(output) = std::process::Command::new("sh")
            .arg("-c")
            .arg("which typst || true")
            .output()
        {
            if output.status.success() {
                if let Ok(found) = String::from_utf8(output.stdout) {
                    let found = found.trim();
                    if !found.is_empty() {
                        let p = PathBuf::from(found);
                        if p.exists() {
                            return Ok(p);
                        }
                    }
                }
            }
        }

        // Also check common system locations that some distributions and AppImages use
        let common_paths = [
            "/usr/bin/typst",
            "/bin/typst",
            "/usr/local/bin/typst",
            "/snap/bin/typst",
        ];
        for cp in &common_paths {
            let p = PathBuf::from(cp);
            if p.exists() {
                return Ok(p);
            }
        }
    }

    // Fall back to bundled binary in resource directory
    let resource_dir = app_handle
        .path()
        .resource_dir()
        .map_err(|e| anyhow!("Failed to get resource directory: {}", e))?;

    // Search inside bin/typst/<platform>
    let platform_dir = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };

    let platform_base = resource_dir.join("bin").join("typst").join(platform_dir);
    let mut attempted: Vec<PathBuf> = Vec::new();
    let mut candidates: Vec<PathBuf> = Vec::new();

    if cfg!(target_os = "windows") {
        candidates.push(platform_base.join("typst.exe"));
    } else {
        candidates.push(platform_base.join("typst"));
    }

    for c in &candidates {
        attempted.push(c.clone());
        if c.exists() {
            return Ok(c.clone());
        }
    }

    let attempted_list = attempted
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    // As a final fallback, check user preferences for an explicit typst_path
    if let Ok(content_dir) = get_content_dir(app_handle) {
        let prefs_path = content_dir.join("prefs.json");
        if prefs_path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&prefs_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
                    if let Some(tp) = json.get("typst_path").and_then(|v| v.as_str()) {
                        let p = PathBuf::from(tp);
                        if p.exists() {
                            return Ok(p);
                        }
                    }
                }
            }
        }
    }

    Err(anyhow!(
        "Typst binary not found. Download Typst binary and place in appropriate platform directory, or install Typst system-wide. Looked for: {}",
        attempted_list
    ))
}

/// Locate the bundled Pdfium dynamic library for the current platform.
pub fn get_pdfium_library_path(app_handle: &AppHandle) -> Result<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(
            current_dir
                .join("src-tauri")
                .join("bin")
                .join("pdfium")
                .join(platform_dir())
                .join(library_name()),
        );
    }

    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        candidates.push(
            resource_dir
                .join("bin")
                .join("pdfium")
                .join(platform_dir())
                .join(library_name()),
        );
    }

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(anyhow!(
        "Pdfium binary not found. Ensure the platform library is placed under src-tauri/bin/pdfium/<platform>/"
    ))
}

#[cfg(target_os = "windows")]
fn library_name() -> &'static str {
    "pdfium.dll"
}

#[cfg(target_os = "macos")]
fn library_name() -> &'static str {
    "libpdfium.dylib"
}

#[cfg(target_os = "linux")]
fn library_name() -> &'static str {
    "libpdfium.so"
}

#[cfg(target_os = "windows")]
fn platform_dir() -> &'static str {
    "windows"
}

#[cfg(target_os = "macos")]
fn platform_dir() -> &'static str {
    "macos"
}

#[cfg(target_os = "linux")]
fn platform_dir() -> &'static str {
    "linux"
}

/// Locate the bundled or system Tectonic binary.
pub fn get_tectonic_path(app_handle: &AppHandle) -> Result<PathBuf> {
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = if cfg!(target_os = "windows") {
                dir.join("tectonic.exe")
            } else {
                dir.join("tectonic")
            };
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    #[cfg(unix)]
    {
        if let Ok(output) = std::process::Command::new("sh")
            .arg("-c")
            .arg("which tectonic || true")
            .output()
        {
            if output.status.success() {
                if let Ok(found) = String::from_utf8(output.stdout) {
                    let found = found.trim();
                    if !found.is_empty() {
                        let path = PathBuf::from(found);
                        if path.exists() {
                            return Ok(path);
                        }
                    }
                }
            }
        }
    }

    let resource_dir = app_handle
        .path()
        .resource_dir()
        .map_err(|e| anyhow!("Failed to get resource directory: {}", e))?;

    let platform_dir = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    };

    let mut attempted = Vec::new();
    let platform_base = resource_dir.join("bin").join("tectonic").join(platform_dir);
    let binary_name = if cfg!(target_os = "windows") {
        "tectonic.exe"
    } else {
        "tectonic"
    };
    let bundled = platform_base.join(binary_name);
    attempted.push(bundled.clone());
    if bundled.exists() {
        return Ok(bundled);
    }

    Err(anyhow!(
        "Tectonic binary not found. Install it system-wide or place the executable in: {}",
        attempted
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}
