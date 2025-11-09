use crate::preprocessor::TikzBlockMeta;
use crate::utils;
use anyhow::{anyhow, Context, Result};
use image as image_crate;
use log::error;
use pdfium_render::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::process::Command;
use tauri::AppHandle;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

/// Ensure all TikZ assets referenced in the current markdown exist in the build directory.
/// Compiles each diagram with the external Tectonic CLI and caches the PDF output so that
/// repeated renders reuse prior results without re-running LaTeX.
pub fn prepare_tikz_assets(
    app_handle: &AppHandle,
    blocks: &[TikzBlockMeta],
    build_dir: &Path,
) -> Result<()> {
    if blocks.is_empty() {
        return Ok(());
    }

    let tectonic_path = utils::get_tectonic_path(app_handle)?;
    let pdfium_lib = utils::get_pdfium_library_path(app_handle)?;
    let pdfium_bindings = Pdfium::bind_to_library(&pdfium_lib)
        .or_else(|_| Pdfium::bind_to_system_library())
        .map_err(|e| anyhow!("Failed to load Pdfium: {e}"))?;
    let pdfium = Pdfium::new(pdfium_bindings);
    let cache_dir = build_dir.join("tikz-cache");
    let work_dir = build_dir.join("tikz-work");
    fs::create_dir_all(&cache_dir)?;
    fs::create_dir_all(&work_dir)?;

    let mut active_outputs = HashSet::new();

    for block in blocks {
        let key = cache_key(block);
        let cache_file = cache_dir.join(format!("{}.{}", key, block.asset_extension));
        if !cache_file.exists() {
            match compile_block(&tectonic_path, &pdfium, &work_dir, &key, block) {
                Ok(bytes) => {
                    fs::write(&cache_file, bytes)?;
                }
                Err(err) => {
                    error!("[tikz] failed to compile block {}: {}", block.id, err);
                    let fallback = build_error_artifact(
                        &tectonic_path,
                        &pdfium,
                        &work_dir,
                        &key,
                        &err.to_string(),
                    )
                    .with_context(|| {
                        format!(
                            "failed to create fallback artifact for TikZ block {}",
                            block.id
                        )
                    })?;
                    fs::write(&cache_file, fallback)?;
                }
            }
        }

        let dest_path = build_dir.join(&block.asset_path);
        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }
        if dest_path.exists() {
            fs::remove_file(&dest_path)?;
        }
        fs::copy(&cache_file, &dest_path)?;
        active_outputs.insert(dest_path);
    }

    // Remove stale files from the tikz output directory to avoid bloat
    let tikz_dir = build_dir.join("tikz");
    if tikz_dir.exists() {
        if let Ok(entries) = fs::read_dir(&tikz_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if entry.file_type().map(|t| t.is_file()).unwrap_or(false)
                    && !active_outputs.contains(&path)
                {
                    let _ = fs::remove_file(path);
                }
            }
        }
    }

    Ok(())
}

fn compile_block(
    tectonic_path: &Path,
    pdfium: &Pdfium,
    work_dir: &Path,
    cache_key: &str,
    block: &TikzBlockMeta,
) -> Result<Vec<u8>> {
    let (extracted_preamble, body) = split_tikz_preamble_from_body(&block.diagram);

    let mut latex = String::from(
        r"\documentclass[border=2pt]{standalone}
\usepackage{tikz}
",
    );

    if let Some(user_preamble) = block.preamble.as_ref() {
        latex.push_str(user_preamble);
        if !user_preamble.ends_with('\n') {
            latex.push('\n');
        }
    }
    if !extracted_preamble.is_empty() {
        latex.push_str(&extracted_preamble);
        if !extracted_preamble.ends_with('\n') {
            latex.push('\n');
        }
    }

    latex.push_str("\\begin{document}\n");
    latex.push_str(&body);
    if !body.ends_with('\n') {
        latex.push('\n');
    }
    latex.push_str("\\end{document}\n");

    let pdf_bytes = compile_tex(tectonic_path, work_dir, cache_key, &latex)?;
    pdf_bytes_to_png(pdfium, &pdf_bytes)
}

fn split_tikz_preamble_from_body(diagram: &str) -> (String, String) {
    let mut preamble_lines = Vec::new();
    let mut body_lines = Vec::new();

    // Heuristics: move any obvious preamble commands to the preamble, no matter
    // where they appear in the fence. Also strip any stray document begin/end.
    for line in diagram.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            body_lines.push(line);
            continue;
        }
        // Skip document delimiters entirely
        if trimmed.starts_with("\\begin{document}") || trimmed.starts_with("\\end{document}") {
            continue;
        }

        // Identify preamble-only markers if they appear anywhere in the line
        let is_preamble_only = [
            "\\documentclass",
            "\\usepackage",
            "\\RequirePackage",
            "\\PassOptionsToPackage",
            "\\usetikzlibrary",
        ]
        .iter()
        .any(|needle| trimmed.contains(needle));

        if is_preamble_only {
            preamble_lines.push(line);
        } else {
            body_lines.push(line);
        }
    }

    let mut preamble = preamble_lines.join("\n");
    if !preamble.is_empty() && !preamble.ends_with('\n') {
        preamble.push('\n');
    }
    let mut body = body_lines.join("\n");

    // If user provided bare drawing commands, wrap them in a tikzpicture env.
    let has_tikz_env = body.contains("\\begin{tikzpicture}") || body.contains("\\end{tikzpicture}");
    if !has_tikz_env && !body.trim().is_empty() {
        body = format!("\\begin{{tikzpicture}}\n{}\n\\end{{tikzpicture}}\n", body);
    }

    (preamble, body)
}

fn compile_tex(
    tectonic_path: &Path,
    work_dir: &Path,
    base_name: &str,
    tex_source: &str,
) -> Result<Vec<u8>> {
    let tex_path = work_dir.join(format!("{base_name}.tex"));
    fs::write(&tex_path, tex_source)?;

    // Also emit a developer-visible copy for debugging in workspace when running dev.
    if let Ok(cwd) = std::env::current_dir() {
        let dbg_dir = cwd.join("src-tauri").join("gen_debug").join("tikz");
        let _ = fs::create_dir_all(&dbg_dir);
        let _ = fs::write(dbg_dir.join(format!("{base_name}.tex")), tex_source);
    }

    let output_dir = work_dir.join("out");
    fs::create_dir_all(&output_dir)?;
    let pdf_path = output_dir.join(format!("{base_name}.pdf"));
    if pdf_path.exists() {
        let _ = fs::remove_file(&pdf_path);
    }

    let mut command = tectonic_command(tectonic_path);
    command
        .current_dir(work_dir)
        .arg("--synctex=0")
        .arg("--keep-intermediates=false")
        .arg("--outdir")
        .arg(&output_dir)
        .arg(&tex_path);

    let output = command.output()?;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "Tectonic failed (status {}).\nSTDOUT:\n{}\nSTDERR:\n{}",
            output.status,
            stdout.trim(),
            stderr.trim()
        ));
    }

    let bytes = fs::read(&pdf_path)?;
    let _ = fs::remove_file(&tex_path);
    let _ = fs::remove_file(&pdf_path);
    Ok(bytes)
}

fn cache_key(block: &TikzBlockMeta) -> String {
    let mut hasher = Sha256::new();
    hasher.update(block.diagram.as_bytes());
    if let Some(preamble) = block.preamble.as_ref() {
        hasher.update(preamble.as_bytes());
    }
    hex::encode(hasher.finalize())
}

fn build_error_artifact(
    tectonic_path: &Path,
    pdfium: &Pdfium,
    work_dir: &Path,
    cache_key: &str,
    message: &str,
) -> Result<Vec<u8>> {
    let preview = truncate_message(message);
    let escaped = escape_latex_text(&preview);
    let latex = format!(
        r"\documentclass[border=6pt]{{standalone}}
\usepackage{{xcolor}}
\begin{{document}}
\color{{red}}\ttfamily TikZ render failed:\par {}
\end{{document}}
",
        escaped
    );
    let pdf = compile_tex(tectonic_path, work_dir, cache_key, &latex)?;
    pdf_bytes_to_png(pdfium, &pdf)
}

fn pdf_bytes_to_png(pdfium: &Pdfium, pdf_bytes: &[u8]) -> Result<Vec<u8>> {
    let document = pdfium
        .load_pdf_from_byte_slice(pdf_bytes, None)
        .map_err(|e| anyhow!("Failed to load TikZ PDF: {e}"))?;
    let page = document
        .pages()
        .get(0)
        .map_err(|_| anyhow!("TikZ PDF did not contain any pages"))?;

    let dpi = 288.0;
    let width_px = ((page.width().value / 72.0) * dpi)
        .clamp(1.0, 4096.0)
        .round() as i32;
    let height_px = ((page.height().value / 72.0) * dpi)
        .clamp(1.0, 4096.0)
        .round() as i32;

    let render_config = PdfRenderConfig::new()
        .set_target_width(width_px)
        .set_target_height(height_px)
        .use_print_quality(true)
        .render_annotations(true)
        .render_form_data(true);

    let bitmap = page
        .render_with_config(&render_config)
        .map_err(|e| anyhow!("Failed to rasterize TikZ PDF: {e}"))?;

    let image = bitmap.as_image();
    let rgba = image.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    let data = rgba.into_raw();

    let mut png_bytes = Vec::new();
    {
        let mut cursor = Cursor::new(&mut png_bytes);
        let mut encoder = image_crate::codecs::png::PngEncoder::new(&mut cursor);
        use image_crate::ColorType;
        encoder
            .encode(&data, w, h, ColorType::Rgba8)
            .map_err(|e| anyhow!("Failed to encode TikZ PNG: {e}"))?;
    }

    Ok(png_bytes)
}

fn truncate_message(message: &str) -> String {
    const MAX_LEN: usize = 240;
    let mut result = message.trim().replace('\n', " ");
    if result.len() > MAX_LEN {
        result.truncate(MAX_LEN);
        result.push_str("…");
    }
    result
}

fn escape_latex_text(input: &str) -> String {
    input
        .chars()
        .flat_map(|ch| match ch {
            '\\' => "\\textbackslash{}".chars().collect::<Vec<_>>(),
            '{' => "\\{".chars().collect(),
            '}' => "\\}".chars().collect(),
            '%' => "\\%".chars().collect(),
            '$' => "\\$".chars().collect(),
            '#' => "\\#".chars().collect(),
            '_' => "\\_".chars().collect(),
            '&' => "\\&".chars().collect(),
            '^' => "\\^{}".chars().collect(),
            '~' => "\\~{}".chars().collect(),
            _ => vec![ch],
        })
        .collect()
}

fn tectonic_command(exe: &Path) -> Command {
    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let mut cmd = Command::new(exe);
        cmd.creation_flags(CREATE_NO_WINDOW);
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        Command::new(exe)
    }
}
