fn udev_source_preview(content: &str, palette: &ThemePalette) -> Text<'static> {
    Text::from(
        content.lines().map(|line| {
            let mut spans = Vec::new();
            let mut remaining = line;
            while !remaining.is_empty() {
                let first = remaining.chars().next().expect("nonempty rule");
                let (length, color) = if first == '#' {
                    (remaining.len(), Some(palette.syntax_comment))
                } else if first == '"'
                    || (matches!(first, 'e' | 'i') && remaining[1..].starts_with('"'))
                {
                    let prefix = usize::from(first != '"');
                    let mut escaped = false;
                    let mut length = remaining.len();
                    for (index, character) in remaining.char_indices().skip(prefix + 1) {
                        if escaped {
                            escaped = false;
                        } else if character == '\\' {
                            escaped = true;
                        } else if character == '"' {
                            length = index + 1;
                            break;
                        }
                    }
                    (length, Some(palette.syntax_value))
                } else if let Some(operator) = ["==", "!=", "+=", "-=", ":=", "=", ",", "\\"]
                    .into_iter().find(|operator| remaining.starts_with(operator))
                {
                    (operator.len(), Some(palette.syntax_operator))
                } else if first.is_ascii_alphabetic() || first == '_' {
                    let mut length = remaining.find(|character: char| {
                        !character.is_ascii_alphanumeric() && character != '_'
                    }).unwrap_or(remaining.len());
                    if remaining[length..].starts_with('{')
                        && let Some(close) = remaining[length..].find('}')
                    {
                        length += close + 1;
                    }
                    (length, Some(palette.syntax_name))
                } else {
                    let length = if first.is_whitespace() {
                        remaining.find(|character: char| !character.is_whitespace())
                            .unwrap_or(remaining.len())
                    } else {
                        first.len_utf8()
                    };
                    (length, None)
                };
                let text = remaining[..length].to_owned();
                spans.push(match color {
                    Some(color) => Span::styled(text, Style::default().fg(color)),
                    None => Span::raw(text),
                });
                remaining = &remaining[length..];
            }
            Line::from(spans)
        }).collect::<Vec<_>>(),
    )
}
