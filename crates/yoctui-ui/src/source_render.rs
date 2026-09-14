//! Source render.
use super::*;

pub(crate) fn source_preview(content: &str, file_name: &str, app: &App) -> Text<'static> {
    let bitbake_source = ["bb", "bbappend", "inc", "conf", "wks", "wks.in"]
        .iter()
        .any(|extension| file_name.ends_with(&format!(".{extension}")));
    let markdown = file_name.ends_with(".md") || file_name.ends_with(".markdown");
    let palette = ThemePalette::for_app(app);
    if palette.attribute_only {
        return Text::from(content.to_owned());
    }
    if !bitbake_source && !markdown {
        return generic_source_preview(
            content,
            yoctui_model::SourceLanguage::from_path(Path::new(file_name)),
            app,
        );
    }
    Text::from(
        content
            .lines()
            .map(|line| {
                if markdown {
                    let style = if line.starts_with('#') {
                        Style::default().fg(palette.syntax_keyword)
                    } else if line.starts_with("```") {
                        Style::default().fg(palette.syntax_operator)
                    } else {
                        Style::default()
                    };
                    return Line::from(Span::styled(line.to_owned(), style));
                }
                let (code, comment) = line
                    .split_once('#')
                    .map_or((line, None), |(code, comment)| (code, Some(comment)));
                let mut spans = Vec::new();
                let trimmed = code.trim_start();
                let indent_len = code.len().saturating_sub(trimmed.len());
                if indent_len > 0 {
                    spans.push(Span::raw(code[..indent_len].to_owned()));
                }
                if [
                    "inherit",
                    "require",
                    "include",
                    "export",
                    "addtask",
                    "deltask",
                    "part",
                    "partition",
                ]
                .iter()
                .any(|keyword| trimmed.starts_with(keyword))
                {
                    let keyword_end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
                    spans.push(Span::styled(
                        trimmed[..keyword_end].to_owned(),
                        Style::default().fg(palette.syntax_keyword),
                    ));
                    spans.push(Span::raw(trimmed[keyword_end..].to_owned()));
                } else if let Some(equals) = trimmed.find('=') {
                    let lhs_end = trimmed[..equals]
                        .trim_end_matches([' ', '?', '+', ':'])
                        .len();
                    spans.push(Span::styled(
                        trimmed[..lhs_end].to_owned(),
                        Style::default().fg(palette.syntax_name),
                    ));
                    spans.push(Span::styled(
                        trimmed[lhs_end..=equals].to_owned(),
                        Style::default().fg(palette.syntax_operator),
                    ));
                    spans.push(Span::styled(
                        trimmed[equals + 1..].to_owned(),
                        Style::default().fg(palette.syntax_value),
                    ));
                } else {
                    spans.push(Span::raw(trimmed.to_owned()));
                }
                if let Some(comment) = comment {
                    spans.push(Span::styled(
                        format!("#{comment}"),
                        Style::default().fg(palette.syntax_comment),
                    ));
                }
                Line::from(spans)
            })
            .collect::<Vec<_>>(),
    )
}

pub(crate) fn generic_source_preview(
    content: &str,
    language: yoctui_model::SourceLanguage,
    app: &App,
) -> Text<'static> {
    let palette = ThemePalette::for_app(app);
    let comment_prefix = match language {
        yoctui_model::SourceLanguage::C
        | yoctui_model::SourceLanguage::Cpp
        | yoctui_model::SourceLanguage::Rust
        | yoctui_model::SourceLanguage::JavaScript
        | yoctui_model::SourceLanguage::TypeScript => Some("//"),
        yoctui_model::SourceLanguage::Python
        | yoctui_model::SourceLanguage::Shell
        | yoctui_model::SourceLanguage::Yaml
        | yoctui_model::SourceLanguage::Make => Some("#"),
        _ => None,
    };
    let keywords: &[&str] = match language {
        yoctui_model::SourceLanguage::C | yoctui_model::SourceLanguage::Cpp => &[
            "auto",
            "bool",
            "break",
            "case",
            "char",
            "class",
            "const",
            "continue",
            "default",
            "do",
            "double",
            "else",
            "enum",
            "extern",
            "float",
            "for",
            "if",
            "int",
            "long",
            "namespace",
            "private",
            "protected",
            "public",
            "return",
            "short",
            "signed",
            "sizeof",
            "static",
            "struct",
            "switch",
            "template",
            "typedef",
            "union",
            "unsigned",
            "using",
            "virtual",
            "void",
            "volatile",
            "while",
        ],
        yoctui_model::SourceLanguage::Rust => &[
            "as", "async", "await", "break", "const", "continue", "crate", "else", "enum",
            "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
            "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super",
            "trait", "true", "type", "unsafe", "use", "where", "while",
        ],
        yoctui_model::SourceLanguage::Python => &[
            "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del",
            "elif", "else", "except", "False", "finally", "for", "from", "global", "if", "import",
            "in", "is", "lambda", "None", "nonlocal", "not", "or", "pass", "raise", "return",
            "True", "try", "while", "with", "yield",
        ],
        yoctui_model::SourceLanguage::Shell => &[
            "case", "do", "done", "elif", "else", "esac", "export", "fi", "for", "function", "if",
            "in", "local", "return", "then", "while",
        ],
        yoctui_model::SourceLanguage::JavaScript | yoctui_model::SourceLanguage::TypeScript => &[
            "async",
            "await",
            "break",
            "case",
            "catch",
            "class",
            "const",
            "continue",
            "default",
            "delete",
            "do",
            "else",
            "export",
            "extends",
            "false",
            "finally",
            "for",
            "from",
            "function",
            "if",
            "import",
            "in",
            "instanceof",
            "interface",
            "let",
            "new",
            "null",
            "return",
            "static",
            "super",
            "switch",
            "this",
            "throw",
            "true",
            "try",
            "type",
            "typeof",
            "var",
            "void",
            "while",
            "yield",
        ],
        _ => &[],
    };
    Text::from(
        content
            .lines()
            .map(|line| {
                if comment_prefix.is_some_and(|prefix| line.trim_start().starts_with(prefix)) {
                    return Line::styled(
                        line.to_owned(),
                        Style::default().fg(palette.syntax_comment),
                    );
                }
                let mut spans = Vec::new();
                for token in line.split_inclusive(char::is_whitespace) {
                    let word = token.trim_matches(|character: char| {
                        character.is_whitespace() || "(){}[];,:*&<>".contains(character)
                    });
                    let style = if keywords.contains(&word) {
                        Style::default().fg(palette.syntax_keyword)
                    } else if word.starts_with(['\"', '\'']) {
                        Style::default().fg(palette.syntax_value)
                    } else {
                        Style::default()
                    };
                    spans.push(Span::styled(token.to_owned(), style));
                }
                Line::from(spans)
            })
            .collect::<Vec<_>>(),
    )
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
