/// Raw token kinds produced by the lexer
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// A bare word / identifier: `build`, `CC`, `if`
    Ident(String),
    Equals,
    LBrace,
    RBrace,

    /// `==`
    EqEq,

    /// `!=`
    NotEq,

    /// A raw string value, assignment RHS or a command line
    RawValue(String),

    /// End of a logical line
    Newline,

    /// End of file
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
}

/// Lex the entire source into a flat token list.
pub fn lex(source: &str) -> Vec<Token> {
    let mut tokens = Vec::new();

    for (line_idx, raw_line) in source.lines().enumerate() {
        let line_no = line_idx + 1;
        let trimmed = raw_line.trim();

        // Skip blank lines and full-line comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        lex_line(trimmed, line_no, &mut tokens);
    }

    tokens.push(Token { kind: TokenKind::Eof, line: 0 });
    tokens
}

fn lex_line(line: &str, line_no: usize, out: &mut Vec<Token>) {
    if line == "{" {
        out.push(Token { kind: TokenKind::LBrace, line: line_no });
        out.push(Token { kind: TokenKind::Newline, line: line_no });
        return;
    }
    if line == "}" {
        out.push(Token { kind: TokenKind::RBrace, line: line_no });
        out.push(Token { kind: TokenKind::Newline, line: line_no });
        return;
    }
    // `} else if COND {`
    if line.starts_with("} else if ") && line.ends_with('{') {
        out.push(Token { kind: TokenKind::RBrace, line: line_no });
        out.push(Token { kind: TokenKind::Newline, line: line_no });
        let inner = line[2..line.len()-1].trim(); // "else if COND"
        lex_non_brace_line(inner, line_no, out);
        out.push(Token { kind: TokenKind::LBrace, line: line_no });
        out.push(Token { kind: TokenKind::Newline, line: line_no });
        return;
    }
    // `} else if COND` (brace on next line)
    if line.starts_with("} else if ") {
        out.push(Token { kind: TokenKind::RBrace, line: line_no });
        out.push(Token { kind: TokenKind::Newline, line: line_no });
        let inner = line[2..].trim(); // "else if COND"
        lex_non_brace_line(inner, line_no, out);
        out.push(Token { kind: TokenKind::Newline, line: line_no });
        return;
    }
    // `} else {`
    if line == "} else {" {
        out.push(Token { kind: TokenKind::RBrace, line: line_no });
        out.push(Token { kind: TokenKind::Newline, line: line_no });
        out.push(Token { kind: TokenKind::Ident("else".to_string()), line: line_no });
        out.push(Token { kind: TokenKind::LBrace, line: line_no });
        out.push(Token { kind: TokenKind::Newline, line: line_no });
        return;
    }
    // `} else` (brace on next line)
    if line == "} else" {
        out.push(Token { kind: TokenKind::RBrace, line: line_no });
        out.push(Token { kind: TokenKind::Newline, line: line_no });
        out.push(Token { kind: TokenKind::Ident("else".to_string()), line: line_no });
        out.push(Token { kind: TokenKind::Newline, line: line_no });
        return;
    }

    // Lines ending with `{` 
    if line.ends_with('{') {
        let prefix = line[..line.len() - 1].trim();
        lex_non_brace_line(prefix, line_no, out);
        out.push(Token { kind: TokenKind::LBrace, line: line_no });
        out.push(Token { kind: TokenKind::Newline, line: line_no });
        return;
    }

    lex_non_brace_line(line, line_no, out);
    out.push(Token { kind: TokenKind::Newline, line: line_no });
}

fn lex_non_brace_line(line: &str, line_no: usize, out: &mut Vec<Token>) {
    if line.starts_with('@') {
        let rest = &line[1..];
        let (kw, val) = split_first_word(rest);
        out.push(Token { kind: TokenKind::Ident(format!("@{}", kw)), line: line_no });
        let val = val.trim();
        if !val.is_empty() {
            out.push(Token { kind: TokenKind::RawValue(val.to_string()), line: line_no });
        }
        return;
    }

    if line.starts_with("else if ") {
        let rest = &line[8..]; // skip "else if "
        out.push(Token { kind: TokenKind::Ident("else".to_string()), line: line_no });
        out.push(Token { kind: TokenKind::Ident("if".to_string()), line: line_no });
        lex_condition_expr(rest.trim(), line_no, out);
        return;
    }

    if line == "else" {
        out.push(Token { kind: TokenKind::Ident("else".to_string()), line: line_no });
        return;
    }

    if line.starts_with("if ") || line == "if" {
        let rest = line[2..].trim();
        out.push(Token { kind: TokenKind::Ident("if".to_string()), line: line_no });
        lex_condition_expr(rest, line_no, out);
        return;
    }

    if let Some(eq_pos) = find_assignment_eq(line) {
        let name = line[..eq_pos].trim().to_string();
        let value = strip_inline_comment(line[eq_pos + 1..].trim()).trim().to_string();
        out.push(Token { kind: TokenKind::Ident(name), line: line_no });
        out.push(Token { kind: TokenKind::Equals, line: line_no });
        out.push(Token { kind: TokenKind::RawValue(value), line: line_no });
        return;
    }

    let (first, rest) = split_first_word(line);
    if rest.is_empty() && is_ident(first) {
        out.push(Token { kind: TokenKind::Ident(first.to_string()), line: line_no });
        return;
    }

    let cmd = strip_inline_comment(line);
    if !cmd.trim().is_empty() {
        out.push(Token { kind: TokenKind::RawValue(cmd.trim().to_string()), line: line_no });
    }
}

fn lex_condition_expr(expr: &str, line_no: usize, out: &mut Vec<Token>) {
    if let Some(idx) = expr.find("==") {
        // make sure it's not !=
        let lhs = expr[..idx].trim();
        let rhs = expr[idx + 2..].trim();
        out.push(Token { kind: TokenKind::RawValue(lhs.to_string()), line: line_no });
        out.push(Token { kind: TokenKind::EqEq, line: line_no });
        out.push(Token { kind: TokenKind::RawValue(rhs.to_string()), line: line_no });
    } else if let Some(idx) = expr.find("!=") {
        let lhs = expr[..idx].trim();
        let rhs = expr[idx + 2..].trim();
        out.push(Token { kind: TokenKind::RawValue(lhs.to_string()), line: line_no });
        out.push(Token { kind: TokenKind::NotEq, line: line_no });
        out.push(Token { kind: TokenKind::RawValue(rhs.to_string()), line: line_no });
    } else {
        out.push(Token { kind: TokenKind::RawValue(expr.to_string()), line: line_no });
    }
}

pub fn strip_inline_comment(s: &str) -> &str {
    let mut in_single = false;
    let mut in_double = false;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '\'' if !in_double => in_single = !in_single,
            '"'  if !in_single => in_double = !in_double,
            '#'  if !in_single && !in_double => {
                return s[..s.char_indices().nth(i).map(|(b,_)| b).unwrap_or(s.len())].trim_end();
            }
            _ => {}
        }
        i += 1;
    }
    s
}

/// Find the `=` of a top-level variable assignment.
/// LHS must be a valid identifier; `==` and `!=` are excluded.
fn find_assignment_eq(line: &str) -> Option<usize> {
    // Find first `=` that isn't part of `==` or `!=`
    let bytes = line.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'=' {
            // Exclude `==`
            if i + 1 < bytes.len() && bytes[i + 1] == b'=' {
                return None;
            }
            // Exclude `!=`
            if i > 0 && bytes[i - 1] == b'!' {
                return None;
            }
            let lhs = line[..i].trim();
            if !lhs.is_empty() && is_ident(lhs) {
                return Some(i);
            }
            return None;
        }
    }
    None
}

fn split_first_word(s: &str) -> (&str, &str) {
    match s.find(|c: char| c.is_whitespace()) {
        Some(i) => (&s[..i], &s[i..]),
        None => (s, ""),
    }
}

fn is_ident(s: &str) -> bool {
    !s.is_empty()
        && s.chars().next().map(|c| c.is_alphabetic() || c == '_').unwrap_or(false)
        && s.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-')
}
