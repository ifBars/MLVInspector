//! Lightweight C# syntax highlighting for the decompiled source view.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CSharpTokenKind {
    Plain,
    Keyword,
    Type,
    String,
    Comment,
    Number,
    Preprocessor,
    Attribute,
}

impl CSharpTokenKind {
    pub fn class_name(self) -> &'static str {
        match self {
            Self::Plain => "csharp-token plain",
            Self::Keyword => "csharp-token keyword",
            Self::Type => "csharp-token type",
            Self::String => "csharp-token string",
            Self::Comment => "csharp-token comment",
            Self::Number => "csharp-token number",
            Self::Preprocessor => "csharp-token preprocessor",
            Self::Attribute => "csharp-token attribute",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HighlightedSegment {
    pub text: String,
    pub kind: CSharpTokenKind,
}

impl HighlightedSegment {
    fn new(text: String, kind: CSharpTokenKind) -> Self {
        Self { text, kind }
    }
}

pub fn highlight_csharp(source: &str) -> Vec<Vec<HighlightedSegment>> {
    let chars: Vec<char> = source.chars().collect();
    let mut lines = vec![Vec::new()];
    let mut index = 0;
    let mut in_block_comment = false;

    while index < chars.len() {
        let ch = chars[index];

        if ch == '\n' {
            lines.push(Vec::new());
            index += 1;
            continue;
        }

        if in_block_comment {
            let start = index;
            while index < chars.len() {
                if chars[index] == '\n' {
                    push_segment(&mut lines, &chars[start..index], CSharpTokenKind::Comment);
                    break;
                }
                if chars[index] == '*' && chars.get(index + 1) == Some(&'/') {
                    index += 2;
                    push_segment(&mut lines, &chars[start..index], CSharpTokenKind::Comment);
                    in_block_comment = false;
                    break;
                }
                index += 1;
            }

            if index >= chars.len() {
                push_segment(&mut lines, &chars[start..index], CSharpTokenKind::Comment);
            }
            continue;
        }

        if is_line_start(&chars, index) {
            let mut lookahead = index;
            while lookahead < chars.len() && (chars[lookahead] == ' ' || chars[lookahead] == '\t') {
                lookahead += 1;
            }
            if chars.get(lookahead) == Some(&'#') {
                let start = index;
                while index < chars.len() && chars[index] != '\n' {
                    index += 1;
                }
                push_segment(
                    &mut lines,
                    &chars[start..index],
                    CSharpTokenKind::Preprocessor,
                );
                continue;
            }
        }

        if ch == '/' && chars.get(index + 1) == Some(&'/') {
            let start = index;
            while index < chars.len() && chars[index] != '\n' {
                index += 1;
            }
            push_segment(&mut lines, &chars[start..index], CSharpTokenKind::Comment);
            continue;
        }

        if ch == '/' && chars.get(index + 1) == Some(&'*') {
            let start = index;
            index += 2;
            while index < chars.len() {
                if chars[index] == '\n' {
                    push_segment(&mut lines, &chars[start..index], CSharpTokenKind::Comment);
                    in_block_comment = true;
                    break;
                }
                if chars[index] == '*' && chars.get(index + 1) == Some(&'/') {
                    index += 2;
                    push_segment(&mut lines, &chars[start..index], CSharpTokenKind::Comment);
                    break;
                }
                index += 1;
            }

            if index >= chars.len() {
                push_segment(&mut lines, &chars[start..index], CSharpTokenKind::Comment);
            }
            continue;
        }

        if let Some(end) = string_end(&chars, index) {
            push_segment(&mut lines, &chars[index..end], CSharpTokenKind::String);
            index = end;
            continue;
        }

        if ch == '[' {
            let start = index;
            index += 1;
            while index < chars.len() && (chars[index].is_ascii_whitespace() || chars[index] == ':')
            {
                index += 1;
            }
            while index < chars.len() && is_identifier_part(chars[index]) {
                index += 1;
            }
            push_segment(&mut lines, &chars[start..index], CSharpTokenKind::Attribute);
            continue;
        }

        if ch.is_ascii_digit() {
            let start = index;
            index += 1;
            while index < chars.len() && is_number_part(chars[index]) {
                index += 1;
            }
            push_segment(&mut lines, &chars[start..index], CSharpTokenKind::Number);
            continue;
        }

        if is_identifier_start(ch) {
            let start = index;
            index += 1;
            while index < chars.len() && is_identifier_part(chars[index]) {
                index += 1;
            }
            let ident: String = chars[start..index].iter().collect();
            let kind = classify_identifier(&ident);
            push_segment(&mut lines, &chars[start..index], kind);
            continue;
        }

        let start = index;
        index += 1;
        while index < chars.len()
            && chars[index] != '\n'
            && !is_identifier_start(chars[index])
            && !chars[index].is_ascii_digit()
            && string_end(&chars, index).is_none()
            && !(chars[index] == '/' && matches!(chars.get(index + 1), Some('/') | Some('*')))
            && chars[index] != '['
        {
            index += 1;
        }
        push_segment(&mut lines, &chars[start..index], CSharpTokenKind::Plain);
    }

    lines
}

fn push_segment(lines: &mut [Vec<HighlightedSegment>], slice: &[char], kind: CSharpTokenKind) {
    if slice.is_empty() {
        return;
    }

    let text: String = slice.iter().collect();
    if let Some(line) = lines.last_mut() {
        line.push(HighlightedSegment::new(text, kind));
    }
}

fn is_line_start(chars: &[char], index: usize) -> bool {
    index == 0 || chars[index - 1] == '\n'
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_identifier_part(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn is_number_part(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | 'x' | 'X')
}

fn classify_identifier(ident: &str) -> CSharpTokenKind {
    if CSHARP_KEYWORDS.contains(&ident) {
        CSharpTokenKind::Keyword
    } else if BUILTIN_TYPES.contains(&ident) || ident.chars().next().is_some_and(char::is_uppercase)
    {
        CSharpTokenKind::Type
    } else {
        CSharpTokenKind::Plain
    }
}

fn string_end(chars: &[char], index: usize) -> Option<usize> {
    let prefixes = [
        ('@', '"', 2usize),
        ('$', '"', 2usize),
        ('\0', '"', 1usize),
        ('\0', '\'', 1usize),
    ];

    for (prefix, delimiter, width) in prefixes {
        let matches = if prefix == '\0' {
            chars.get(index) == Some(&delimiter)
        } else {
            chars.get(index) == Some(&prefix) && chars.get(index + 1) == Some(&delimiter)
        };

        if !matches {
            continue;
        }

        let mut cursor = index + width;
        let verbatim = prefix == '@';
        while cursor < chars.len() {
            if chars[cursor] == '\n' && !verbatim {
                return Some(cursor);
            }

            if chars[cursor] == delimiter {
                if verbatim && chars.get(cursor + 1) == Some(&delimiter) {
                    cursor += 2;
                    continue;
                }

                if !verbatim {
                    let mut backslashes = 0usize;
                    let mut scan = cursor;
                    while scan > index + width - 1 && chars[scan - 1] == '\\' {
                        backslashes += 1;
                        scan -= 1;
                    }
                    if backslashes % 2 == 1 {
                        cursor += 1;
                        continue;
                    }
                }

                return Some(cursor + 1);
            }
            cursor += 1;
        }

        return Some(chars.len());
    }

    if chars.get(index) == Some(&'$')
        && chars
            .get(index + 1)
            .is_some_and(|ch| *ch == '@' || *ch == '$')
        && chars.get(index + 2) == Some(&'"')
    {
        return string_end(chars, index + 1).map(|end| end + 1);
    }

    if chars.get(index) == Some(&'@')
        && chars.get(index + 1) == Some(&'$')
        && chars.get(index + 2) == Some(&'"')
    {
        return string_end(chars, index + 1).map(|end| end + 1);
    }

    None
}

const CSHARP_KEYWORDS: &[&str] = &[
    "abstract",
    "as",
    "async",
    "await",
    "base",
    "break",
    "case",
    "catch",
    "checked",
    "class",
    "const",
    "continue",
    "default",
    "delegate",
    "do",
    "else",
    "enum",
    "event",
    "explicit",
    "extern",
    "false",
    "finally",
    "fixed",
    "for",
    "foreach",
    "goto",
    "if",
    "implicit",
    "in",
    "interface",
    "internal",
    "is",
    "lock",
    "namespace",
    "new",
    "null",
    "operator",
    "out",
    "override",
    "params",
    "private",
    "protected",
    "public",
    "readonly",
    "record",
    "ref",
    "required",
    "return",
    "sealed",
    "sizeof",
    "stackalloc",
    "static",
    "struct",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "unchecked",
    "unsafe",
    "using",
    "virtual",
    "void",
    "volatile",
    "while",
    "yield",
];

const BUILTIN_TYPES: &[&str] = &[
    "bool", "byte", "char", "decimal", "double", "dynamic", "float", "int", "long", "nint",
    "nuint", "object", "sbyte", "short", "string", "uint", "ulong", "ushort", "var",
];

#[cfg(test)]
mod tests {
    use super::{highlight_csharp, CSharpTokenKind};

    #[test]
    fn highlight_csharp_marks_common_token_kinds() {
        let lines = highlight_csharp("public sealed class Demo { string name = \"hi\"; // note\n}");

        assert!(lines[0]
            .iter()
            .any(|segment| segment.text == "public" && segment.kind == CSharpTokenKind::Keyword));
        assert!(lines[0]
            .iter()
            .any(|segment| segment.text == "Demo" && segment.kind == CSharpTokenKind::Type));
        assert!(lines[0]
            .iter()
            .any(|segment| segment.text == "string" && segment.kind == CSharpTokenKind::Type));
        assert!(lines[0]
            .iter()
            .any(|segment| segment.text == "\"hi\"" && segment.kind == CSharpTokenKind::String));
        assert!(lines[0].iter().any(|segment| {
            segment.text == "// note" && segment.kind == CSharpTokenKind::Comment
        }));
    }

    #[test]
    fn highlight_csharp_keeps_multiline_block_comments_colored() {
        let lines = highlight_csharp("/* first\nsecond */\nreturn 42;");

        assert!(lines[0]
            .iter()
            .all(|segment| segment.kind == CSharpTokenKind::Comment));
        assert!(lines[1]
            .iter()
            .all(|segment| segment.kind == CSharpTokenKind::Comment));
        assert!(lines[2]
            .iter()
            .any(|segment| segment.text == "return" && segment.kind == CSharpTokenKind::Keyword));
        assert!(lines[2]
            .iter()
            .any(|segment| segment.text == "42" && segment.kind == CSharpTokenKind::Number));
    }
}
