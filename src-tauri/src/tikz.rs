use crate::preprocessor::TikzBlockMeta;
use crate::utils;
use anyhow::{anyhow, Context, Result};
use log::error;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs;
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
    let cache_dir = build_dir.join("tikz-cache");
    let work_dir = build_dir.join("tikz-work");
    fs::create_dir_all(&cache_dir)?;
    fs::create_dir_all(&work_dir)?;

    let mut active_outputs = HashSet::new();

    for block in blocks {
        let key = cache_key(block);
        let cache_file = cache_dir.join(format!("{}.{}", key, block.asset_extension));
        if !cache_file.exists() {
            match compile_block(&tectonic_path, &work_dir, &key, block) {
                Ok(bytes) => {
                    fs::write(&cache_file, bytes)?;
                }
                Err(err) => {
                    error!("[tikz] failed to compile block {}: {}", block.id, err);
                    let fallback =
                        build_error_artifact(&tectonic_path, &work_dir, &key, &err.to_string())
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
    work_dir: &Path,
    cache_key: &str,
    block: &TikzBlockMeta,
) -> Result<Vec<u8>> {
    let mut latex = String::from(
        r"\documentclass[border=2pt]{standalone}
\usepackage{tikz}
",
    );

    if let Some(preamble) = block.preamble.as_ref() {
        latex.push_str(preamble);
        if !preamble.ends_with('\n') {
            latex.push('\n');
        }
    }

    latex.push_str(
        r"\begin{document}
",
    );
    latex.push_str(&block.diagram);
    if !block.diagram.ends_with('\n') {
        latex.push('\n');
    }
    latex.push_str(
        r"\end{document}
",
    );

    compile_tex(tectonic_path, work_dir, cache_key, &latex)
}

fn compile_tex(
    tectonic_path: &Path,
    work_dir: &Path,
    base_name: &str,
    tex_source: &str,
) -> Result<Vec<u8>> {
    let tex_path = work_dir.join(format!("{base_name}.tex"));
    fs::write(&tex_path, tex_source)?;

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
    hasher.update(block.requested_format.as_str().as_bytes());
    hex::encode(hasher.finalize())
}

fn build_error_artifact(
    tectonic_path: &Path,
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
    compile_tex(tectonic_path, work_dir, cache_key, &latex)
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
