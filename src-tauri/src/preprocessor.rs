use anyhow::Result;
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Parser, Tag};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize)]
pub struct EditorPosition {
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PdfPosition {
    pub page: usize,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnchorEntry {
    pub id: String,
    pub editor: EditorPosition,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf: Option<PdfPosition>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SourceMapPayload {
    pub anchors: Vec<AnchorEntry>,
}

#[derive(Debug, Clone)]
pub struct AnchorMeta {
    pub id: String,
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone)]
pub struct PreprocessorOutput {
    pub markdown: String,
    pub anchors: Vec<AnchorMeta>,
}

/// Transform user markdown by injecting invisible Typst anchors used for scroll synchronisation.
pub fn preprocess_markdown(markdown: &str) -> Result<PreprocessorOutput> {
    let transformed = inject_tikz_blocks(markdown);
    let result = inject_anchors(&transformed)?;
    Ok(result)
}

fn inject_tikz_blocks(markdown: &str) -> String {
    let mut replacements: Vec<(usize, usize, String)> = Vec::new();
    let mut current: Option<TikzBlockInProgress> = None;

    let parser = Parser::new_ext(
        markdown,
        Options::ENABLE_FOOTNOTES
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TABLES
            | Options::ENABLE_SMART_PUNCTUATION
            | Options::ENABLE_HEADING_ATTRIBUTES,
    );

    for (event, range) in parser.into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) => {
                if let Some(options) = parse_tikz_fence(&info) {
                    current = Some(TikzBlockInProgress {
                        start: range.start,
                        options,
                        content: String::new(),
                    });
                }
            }
            Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(_))) => {
                if let Some(active) = current.take() {
                    let placeholder = build_tikz_placeholder(&active.content, &active.options);
                    replacements.push((active.start, range.end, placeholder));
                }
            }
            Event::Text(text) | Event::Code(text) => {
                if let Some(active) = current.as_mut() {
                    active.content.push_str(&text);
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if let Some(active) = current.as_mut() {
                    active.content.push('\n');
                }
            }
            _ => {}
        }
    }

    if replacements.is_empty() {
        return markdown.to_owned();
    }

    let mut output = markdown.to_owned();
    for (start, end, replacement) in replacements.into_iter().rev() {
        if start <= end && end <= output.len() {
            output.replace_range(start..end, &replacement);
        }
    }

    output
}

#[derive(Debug, Clone, Default)]
struct TikzFenceOptions {
    scale: Option<String>,
    preamble: Option<String>,
    format: Option<String>,
}

#[derive(Debug, Clone)]
struct TikzBlockInProgress {
    start: usize,
    options: TikzFenceOptions,
    content: String,
}

fn parse_tikz_fence(info: &CowStr<'_>) -> Option<TikzFenceOptions> {
    let raw = info.trim();
    if raw.is_empty() {
        return None;
    }

    let mut tokens = tokenize_fence_info(raw);
    if tokens.is_empty() {
        return None;
    }

    let language = tokens.remove(0);
    let normalized_lang = language
        .trim_matches(|c: char| matches!(c, ',' | '{' | '}' | '[' | ']' | '(' | ')' | ';'))
        .to_ascii_lowercase();
    if normalized_lang != "tikz" {
        return None;
    }

    let mut options = TikzFenceOptions::default();

    for token in tokens {
        let cleaned = token.trim();
        if cleaned.is_empty() {
            continue;
        }

        if let Some((key, value)) = cleaned.split_once('=') {
            let key = key
                .trim()
                .trim_matches(|c: char| matches!(c, ',' | '{' | '}' | '[' | ']' | '(' | ')' | ';'))
                .to_ascii_lowercase();
            let normalized_value = normalize_option_value(value);

            match key.as_str() {
                "scale" => {
                    if let Some(value) = normalized_value {
                        options.scale = Some(value);
                    }
                }
                "preamble" => {
                    if let Some(value) = normalized_value {
                        options.preamble = Some(value);
                    }
                }
                "format" => {
                    if let Some(value) = normalized_value {
                        options.format = Some(value);
                    }
                }
                _ => {}
            }
        }
    }

    Some(options)
}

fn build_tikz_placeholder(content: &str, options: &TikzFenceOptions) -> String {
    let mut args = Vec::new();
    args.push(format!("diagram: \"{}\"", escape_typst_string(content)));

    if let Some(scale) = options.scale.as_ref() {
        if !scale.trim().is_empty() {
            args.push(format!("scale: {}", format_scale_value(scale)));
        }
    }

    if let Some(preamble) = options.preamble.as_ref() {
        args.push(format!("preamble: \"{}\"", escape_typst_string(preamble)));
    }

    if let Some(format) = options.format.as_ref() {
        if !format.is_empty() {
            args.push(format!("format: \"{}\"", escape_typst_string(format)));
        }
    }

    let mut placeholder = String::new();
    placeholder.push_str("<!--raw-typst #tikz_render(");
    placeholder.push_str(&args.join(", "));
    placeholder.push_str(") -->\n");
    placeholder
}

fn escape_typst_string(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn format_scale_value(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return "none".to_string();
    }

    if trimmed.eq_ignore_ascii_case("none") {
        return "none".to_string();
    }

    if trimmed.eq_ignore_ascii_case("auto") {
        return "auto".to_string();
    }

    if let Ok(number) = trimmed.parse::<f32>() {
        // Preserve simple numeric strings while normalising to avoid trailing zeros from parse+format.
        let mut formatted = format!("{}", number);
        if formatted.contains('.') {
            while formatted.ends_with('0') {
                formatted.pop();
            }
            if formatted.ends_with('.') {
                formatted.push('0');
            }
        }
        formatted
    } else {
        format!("\"{}\"", escape_typst_string(trimmed))
    }
}

fn tokenize_fence_info(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_quotes = false;
    let mut quote_char = '\0';

    while let Some(ch) = chars.next() {
        match ch {
            '\'' | '"' => {
                if in_quotes {
                    current.push(ch);
                    if ch == quote_char {
                        in_quotes = false;
                        quote_char = '\0';
                    }
                } else {
                    in_quotes = true;
                    quote_char = ch;
                    current.push(ch);
                }
            }
            '\\' => {
                current.push(ch);
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            }
            c if c.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    tokens.push(current.trim().to_string());
                    current.clear();
                }
            }
            ',' | ';' if !in_quotes => {
                if !current.is_empty() {
                    tokens.push(current.trim().to_string());
                    current.clear();
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        tokens.push(current.trim().to_string());
    }

    tokens
}

fn normalize_option_value(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_matches(|c: char| matches!(c, ',' | ';'));
    if trimmed.is_empty() {
        return Some(String::new());
    }

    let bytes = trimmed.as_bytes();
    if bytes.len() >= 2 && (trimmed.starts_with('"') && trimmed.ends_with('"')) {
        return Some(unescape_quoted(&trimmed[1..trimmed.len() - 1]));
    }
    if bytes.len() >= 2 && (trimmed.starts_with('\'') && trimmed.ends_with('\'')) {
        return Some(unescape_quoted(&trimmed[1..trimmed.len() - 1]));
    }

    Some(trimmed.to_string())
}

fn unescape_quoted(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(next) = chars.next() {
                match next {
                    'n' => result.push('\n'),
                    'r' => result.push('\r'),
                    't' => result.push('\t'),
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    '\'' => result.push('\''),
                    other => {
                        result.push('\\');
                        result.push(other);
                    }
                }
            } else {
                result.push('\\');
            }
        } else {
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::CowStr;

    #[test]
    fn parse_tikz_options_with_quotes() {
        let info =
            CowStr::from(r#"tikz scale=0.75 preamble="\usetikzlibrary{calc}" format='vector'"#);
        let options = parse_tikz_fence(&info).expect("tikz fence should parse");
        assert_eq!(options.scale.as_deref(), Some("0.75"));
        assert_eq!(options.preamble.as_deref(), Some(r"\usetikzlibrary{calc}"));
        assert_eq!(options.format.as_deref(), Some("vector"));
    }

    #[test]
    fn injects_placeholder_with_multiple_options() {
        let markdown = r#"Intro
```tikz scale=auto preamble="\usetikzlibrary{arrows.meta}" format=png
\begin{tikzpicture}
  \draw (0,0) -- (1,1);
\end{tikzpicture}
```
"#;

        let transformed = inject_tikz_blocks(markdown);
        let expected_preamble = format!(
            "preamble: \"{}\"",
            escape_typst_string("\\usetikzlibrary{arrows.meta}")
        );
        assert!(transformed.contains("#tikz_render"));
        assert!(transformed.contains("scale: auto"));
        assert!(transformed.contains(&expected_preamble));
        assert!(transformed.contains("format: \"png\""));
        assert!(!transformed.contains("```tikz"));
    }
}

fn inject_anchors(markdown: &str) -> Result<PreprocessorOutput> {
    let mut insertions: Vec<(usize, String)> = Vec::new();
    let mut anchors: Vec<AnchorMeta> = Vec::new();
    let mut seen_offsets: HashSet<usize> = HashSet::new();

    // Ensure there's always a document-start anchor so preview can scroll to
    // the top even when a cover page is rendered above content.
    let doc_id = "tf-doc-start".to_string();
    if !seen_offsets.contains(&0) {
        let doc_anchor = build_anchor_markup(markdown, 0, &doc_id);
        insertions.push((0, doc_anchor));
        anchors.push(AnchorMeta {
            id: doc_id.clone(),
            offset: 0,
            line: 0,
            column: 0,
        });
        seen_offsets.insert(0usize);
    }

    let parser = Parser::new_ext(
        markdown,
        Options::ENABLE_FOOTNOTES
            | Options::ENABLE_TASKLISTS
            | Options::ENABLE_STRIKETHROUGH
            | Options::ENABLE_TABLES
            | Options::ENABLE_SMART_PUNCTUATION
            | Options::ENABLE_HEADING_ATTRIBUTES,
    );
    for (event, range) in parser.into_offset_iter() {
        if let Event::Start(tag) = event {
            if !is_block_level(&tag) {
                continue;
            }

            // SKIP blockquote tags - they cause issues because the anchor gets inserted
            // between the '>' and the content. We'll still get anchors from the paragraphs
            // inside the blockquote, which is sufficient for scrolling.
            if matches!(tag, Tag::BlockQuote) {
                continue;
            }

            // SKIP table-related tags - injecting anchors inside tables breaks markdown table syntax.
            // Tables need to be continuous without interruption. We'll get an anchor before the table
            // starts, which is sufficient for scrolling to table content.
            if matches!(
                tag,
                Tag::Table(_) | Tag::TableHead | Tag::TableRow | Tag::TableCell
            ) {
                continue;
            }

            let insertion_offset = range.start;

            // If we're inserting into a blockquote line (starts with '>'), SKIP it entirely.
            // Blockquotes (including admonitions) will get anchored via their inner paragraphs.
            let mut line_start = insertion_offset;
            while line_start > 0 && markdown.as_bytes()[line_start - 1] != b'\n' {
                line_start -= 1;
            }

            // Check if this line starts with '>' (possibly with whitespace before)
            let line_text = &markdown[line_start..];
            let first_line = line_text.split('\n').next().unwrap_or("");
            if first_line.trim_start().starts_with('>') {
                // Skip this anchor entirely - don't insert into blockquote lines
                continue;
            }

            if !seen_offsets.insert(insertion_offset) {
                continue;
            }
            let id = format!("tf-{}-{}", range.start, anchors.len());
            let (line, column) = offset_to_line_column(markdown, range.start);
            let anchor_markup = build_anchor_markup(markdown, insertion_offset, &id);
            insertions.push((insertion_offset, anchor_markup));
            anchors.push(AnchorMeta {
                id,
                offset: range.start,
                line,
                column,
            });
        }
    }

    insertions.sort_by_key(|(offset, _)| *offset);
    let mut output = markdown.to_owned();
    for (offset, snippet) in insertions.into_iter().rev() {
        output.insert_str(offset, &snippet);
    }

    Ok(PreprocessorOutput {
        markdown: output,
        anchors,
    })
}

fn is_block_level(tag: &Tag<'_>) -> bool {
    matches!(
        tag,
        Tag::Paragraph
            | Tag::Heading(..)
            | Tag::BlockQuote
            | Tag::CodeBlock(_)
            | Tag::List(_)
            | Tag::Item
            | Tag::FootnoteDefinition(_)
            | Tag::Table(_)
            | Tag::TableHead
            | Tag::TableRow
            | Tag::TableCell
    )
}

fn offset_to_line_column(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 0;
    let mut column = 0;
    for ch in source[..offset].chars() {
        if ch == '\n' {
            line += 1;
            column = 0;
        } else {
            column += 1;
        }
    }
    (line, column)
}

fn build_anchor_markup(source: &str, offset: usize, id: &str) -> String {
    let mut snippet = String::new();

    // Original logic - ensure we're on a new line
    if offset > 0 {
        let preceding = &source[..offset];
        if !preceding.ends_with('\n') {
            snippet.push('\n');
        }
    }
    // Inject a literal label call so Typst sees a label node it can query.
    // A bare label is accepted by Typst and will be discoverable by `typst query`.
    snippet.push_str("<!--raw-typst #label(\"");
    snippet.push_str(id);
    snippet.push_str("\") -->\n");
    snippet
}

pub fn attach_pdf_positions(
    anchors: &[AnchorMeta],
    positions: &HashMap<String, PdfPosition>,
) -> SourceMapPayload {
    let entries = anchors
        .iter()
        .map(|anchor| AnchorEntry {
            id: anchor.id.clone(),
            editor: EditorPosition {
                offset: anchor.offset,
                line: anchor.line,
                column: anchor.column,
            },
            pdf: positions.get(&anchor.id).cloned(),
        })
        .collect();

    SourceMapPayload { anchors: entries }
}

#[allow(dead_code)]
pub fn anchors_to_lookup(anchors: &[AnchorMeta]) -> HashMap<String, EditorPosition> {
    anchors
        .iter()
        .map(|anchor| {
            (
                anchor.id.clone(),
                EditorPosition {
                    offset: anchor.offset,
                    line: anchor.line,
                    column: anchor.column,
                },
            )
        })
        .collect()
}

pub fn pdf_positions_from_query(json_bytes: &[u8]) -> Result<HashMap<String, PdfPosition>> {
    let value: serde_json::Value = serde_json::from_slice(json_bytes)?;
    let mut map = HashMap::new();
    if let Some(entries) = value.as_array() {
        for entry in entries {
            if let Some(label) = find_label(entry) {
                if !label.starts_with("tf-") {
                    continue;
                }
                // Try to find a location object anywhere under this entry. Typst
                // output varies by version and query shape; search for common
                // variants such as { location: { page, position: { x,y } } }
                // or nested fields like 'point', 'pos', or 'rect'.
                if let Some((page, x, y)) = find_location(entry) {
                    map.insert(label, PdfPosition { page, x, y });
                }
            }
        }
    }
    Ok(map)
}

fn find_label(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(label) = map.get("label").and_then(|v| v.as_str()) {
                return Some(label.to_owned());
            }
            for key in ["value", "target", "node", "fields"] {
                if let Some(child) = map.get(key) {
                    if let Some(found) = find_label(child) {
                        return Some(found);
                    }
                }
            }
            None
        }
        serde_json::Value::Array(arr) => arr.iter().find_map(find_label),
        _ => None,
    }
}

/// Recursively search a serde_json::Value for a location-like object and extract
/// (page, x, y) if possible. Supports keys: location, page, position, point,
/// pos, rect (rect may provide [x0,y0,x1,y1] coords; we use y0 as baseline).
fn find_location(value: &serde_json::Value) -> Option<(usize, f32, f32)> {
    match value {
        serde_json::Value::Object(map) => {
            // Direct location field
            if let Some(loc) = map.get("location") {
                if let Some(res) = extract_page_xy(loc) {
                    return Some(res);
                }
            }
            // Some outputs might put page/position at top-level
            if let Some(res) = extract_page_xy(&serde_json::Value::Object(map.clone())) {
                return Some(res);
            }
            // Recurse into children
            for (_k, v) in map.iter() {
                if let Some(found) = find_location(v) {
                    return Some(found);
                }
            }
            None
        }
        serde_json::Value::Array(arr) => arr.iter().find_map(find_location),
        _ => None,
    }
}

fn extract_page_xy(v: &serde_json::Value) -> Option<(usize, f32, f32)> {
    if let Some(obj) = v.as_object() {
        // Page
        // Page may be numeric or string; accept both
        let page = obj
            .get("page")
            .and_then(|p| {
                p.as_u64()
                    .or_else(|| p.as_str().and_then(|s| s.parse::<u64>().ok()))
            })
            .unwrap_or(1) as usize;

        // Position variants
        if let Some(pos) = obj
            .get("position")
            .or_else(|| obj.get("point"))
            .or_else(|| obj.get("pos"))
        {
            if let Some(pos_obj) = pos.as_object() {
                let x = pos_obj
                    .get("x")
                    .and_then(|v| {
                        v.as_f64()
                            .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
                    })
                    .unwrap_or(0.0) as f32;
                let y = pos_obj
                    .get("y")
                    .and_then(|v| {
                        v.as_f64()
                            .or_else(|| v.as_str().and_then(|s| s.parse::<f64>().ok()))
                    })
                    .unwrap_or(0.0) as f32;
                return Some((page, x, y));
            }
        }

        // rect variant: [x0, y0, x1, y1]
        if let Some(rect) = obj.get("rect") {
            if let Some(arr) = rect.as_array() {
                if arr.len() >= 2 {
                    let x = arr[0]
                        .as_f64()
                        .or_else(|| arr[0].as_str().and_then(|s| s.parse::<f64>().ok()))
                        .unwrap_or(0.0) as f32;
                    let y = arr[1]
                        .as_f64()
                        .or_else(|| arr[1].as_str().and_then(|s| s.parse::<f64>().ok()))
                        .unwrap_or(0.0) as f32;
                    return Some((page, x, y));
                }
            }
        }
    }
    None
}
