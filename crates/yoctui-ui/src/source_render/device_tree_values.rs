fn highlight_device_tree_values(value: &str, palette: &ThemePalette) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut start = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for (index, character) in value.char_indices() {
        if character == '"' && !escaped {
            if quoted {
                let end = index + character.len_utf8();
                spans.push(Span::styled(
                    value[start..end].to_owned(),
                    Style::default().fg(palette.syntax_value),
                ));
                start = end;
            } else {
                if start < index {
                    spans.extend(highlight_device_tree_scalar(&value[start..index], palette));
                }
                start = index;
            }
            quoted = !quoted;
        }
        escaped = quoted && character == '\\' && !escaped;
        if character != '\\' {
            escaped = false;
        }
    }
    if start < value.len() {
        if quoted {
            spans.push(Span::styled(
                value[start..].to_owned(),
                Style::default().fg(palette.syntax_value),
            ));
        } else {
            spans.extend(highlight_device_tree_scalar(&value[start..], palette));
        }
    }
    spans
}

fn highlight_device_tree_scalar(value: &str, palette: &ThemePalette) -> Vec<Span<'static>> {
    value
        .split_inclusive(|character: char| character.is_whitespace())
        .map(|token| {
            let word = token.trim_matches(char::is_whitespace);
            let style = if word.starts_with('&')
                || word.starts_with('<')
                || word.starts_with('[')
                || word
                    .chars()
                    .next()
                    .is_some_and(|value| value.is_ascii_digit())
            {
                Style::default().fg(palette.syntax_value)
            } else if word
                .chars()
                .any(|character| "{};<>[],:".contains(character))
            {
                Style::default().fg(palette.syntax_operator)
            } else {
                Style::default()
            };
            Span::styled(token.to_owned(), style)
        })
        .collect()
}

pub(crate) fn numbered_source_preview(content: &str, file_name: &str, app: &App) -> Text<'static> {
    let palette = ThemePalette::for_app(app);
    let mut source = source_preview(content, file_name, app);
    for (index, line) in source.lines.iter_mut().enumerate() {
        line.spans.insert(
            0,
            Span::styled(
                format!("{:>4} │ ", index + 1),
                palette.role(palette.disabled, Modifier::DIM),
            ),
        );
    }
    source
}
