fn device_tree_source_preview(content: &str, palette: &ThemePalette) -> Text<'static> {
    let mut in_block_comment = false;
    let lines = content
        .lines()
        .map(|line| {
            let mut spans = Vec::new();
            let mut remaining = line;
            while !remaining.is_empty() {
                if in_block_comment {
                    if let Some(end) = remaining.find("*/") {
                        let boundary = end + 2;
                        spans.push(Span::styled(
                            remaining[..boundary].to_owned(),
                            Style::default().fg(palette.syntax_comment),
                        ));
                        remaining = &remaining[boundary..];
                        in_block_comment = false;
                    } else {
                        spans.push(Span::styled(
                            remaining.to_owned(),
                            Style::default().fg(palette.syntax_comment),
                        ));
                        remaining = "";
                    }
                    continue;
                }

                let comment = device_tree_comment(remaining);
                if let Some((start, block)) = comment {
                    spans.extend(highlight_device_tree_code(&remaining[..start], palette));
                    if block {
                        remaining = &remaining[start..];
                        in_block_comment = true;
                    } else {
                        spans.push(Span::styled(
                            remaining[start..].to_owned(),
                            Style::default().fg(palette.syntax_comment),
                        ));
                        remaining = "";
                    }
                } else {
                    spans.extend(highlight_device_tree_code(remaining, palette));
                    remaining = "";
                }
            }
            Line::from(spans)
        })
        .collect::<Vec<_>>();
    Text::from(lines)
}

fn highlight_device_tree_code(code: &str, palette: &ThemePalette) -> Vec<Span<'static>> {
    if code.is_empty() {
        return Vec::new();
    }
    if code.trim().is_empty() {
        return vec![Span::raw(code.to_owned())];
    }
    let leading = code.len().saturating_sub(code.trim_start().len());
    let trailing = code.trim_end().len();
    let body = &code[leading..trailing];
    let mut spans = Vec::new();
    if leading > 0 {
        spans.push(Span::raw(code[..leading].to_owned()));
    }
    if body.is_empty() {
        spans.push(Span::raw(code[leading..].to_owned()));
        return spans;
    }

    if let Some(directive_end) = device_tree_directive_end(body) {
        spans.push(Span::styled(
            body[..directive_end].to_owned(),
            Style::default().fg(palette.syntax_keyword),
        ));
        spans.extend(highlight_device_tree_values(
            &body[directive_end..],
            palette,
        ));
    } else if let Some(equals) = body.find('=') {
        let name_end = body[..equals].trim_end().len();
        spans.push(Span::styled(
            body[..name_end].to_owned(),
            Style::default().fg(palette.syntax_name),
        ));
        spans.push(Span::raw(body[name_end..equals].to_owned()));
        spans.push(Span::styled(
            "=".to_owned(),
            Style::default().fg(palette.syntax_operator),
        ));
        spans.extend(highlight_device_tree_values(&body[equals + 1..], palette));
    } else if let Some(open) = body.find('{') {
        let declaration = body[..open].trim_end();
        if let Some(colon) = declaration.find(':') {
            spans.push(Span::styled(
                declaration[..colon].to_owned(),
                Style::default().fg(palette.syntax_name),
            ));
            spans.push(Span::styled(
                ":".to_owned(),
                Style::default().fg(palette.syntax_operator),
            ));
            spans.push(Span::styled(
                declaration[colon + 1..].to_owned(),
                Style::default().fg(palette.syntax_keyword),
            ));
        } else {
            spans.push(Span::styled(
                declaration.to_owned(),
                Style::default().fg(palette.syntax_keyword),
            ));
        }
        spans.push(Span::raw(body[declaration.len()..open].to_owned()));
        spans.push(Span::styled(
            "{".to_owned(),
            Style::default().fg(palette.syntax_operator),
        ));
        spans.extend(highlight_device_tree_values(&body[open + 1..], palette));
    } else if let Some(property) = body.strip_suffix(';').filter(|property| {
        property
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ",._+?#-".contains(character))
    }) {
        spans.push(Span::styled(
            property.to_owned(),
            Style::default().fg(palette.syntax_name),
        ));
        spans.push(Span::styled(
            ";".to_owned(),
            Style::default().fg(palette.syntax_operator),
        ));
    } else {
        spans.extend(highlight_device_tree_values(body, palette));
    }
    if trailing < code.len() {
        spans.push(Span::raw(code[trailing..].to_owned()));
    }
    spans
}

fn device_tree_comment(value: &str) -> Option<(usize, bool)> {
    let mut quoted = false;
    let mut escaped = false;
    let bytes = value.as_bytes();
    for (index, byte) in bytes.iter().copied().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        match byte {
            b'\\' if quoted => escaped = true,
            b'"' => quoted = !quoted,
            b'/' if !quoted && bytes.get(index + 1) == Some(&b'/') => {
                return Some((index, false));
            }
            b'/' if !quoted && bytes.get(index + 1) == Some(&b'*') => {
                return Some((index, true));
            }
            _ => {}
        }
    }
    None
}

fn device_tree_directive_end(body: &str) -> Option<usize> {
    [
        "/dts-v1/",
        "/plugin/",
        "/include/",
        "/memreserve/",
        "/delete-node/",
        "/delete-property/",
        "/omit-if-no-ref/",
        "/bits/",
    ]
    .into_iter()
    .find(|directive| body.starts_with(directive))
    .map(str::len)
    .or_else(|| {
        body.starts_with('#')
            .then(|| body.find(char::is_whitespace).unwrap_or(body.len()))
    })
}
