use crate::error::{PyError, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    // Literals
    Int(i64),
    Float(f64),
    Str(String),
    True,
    False,
    None_,
    // Ident / keywords
    Name(String),
    If, Elif, Else,
    While, For, In,
    Def, Return,
    Pass, Break, Continue,
    And, Or, Not,
    // Operators
    Plus, Minus, Star, Slash, DSlash, Percent, DStar,
    Eq, Ne, Lt, Gt, Le, Ge,
    Assign,
    PlusEq, MinusEq, StarEq, SlashEq,
    // Delimiters
    LParen, RParen, LBracket, RBracket, LBrace, RBrace,
    Comma, Colon, Dot,
    // Structure
    Newline, Indent, Dedent, Eof,
}

pub fn tokenize(src: &str) -> Result<Vec<(Tok, usize)>> {
    let chars: Vec<char> = src.chars().collect();
    let mut pos = 0;
    let mut line = 1usize;
    let mut out: Vec<(Tok, usize)> = Vec::new();
    let mut indent_stack: Vec<usize> = vec![0];
    let mut paren_depth: usize = 0;
    let mut at_bol = true;

    while pos <= chars.len() {
        if pos == chars.len() {
            // EOF cleanup
            if !out.is_empty() && !matches!(out.last(), Some((Tok::Newline, _))) {
                out.push((Tok::Newline, line));
            }
            while indent_stack.len() > 1 {
                indent_stack.pop();
                out.push((Tok::Dedent, line));
            }
            out.push((Tok::Eof, line));
            break;
        }

        let ch = chars[pos];

        // ── beginning-of-line indent handling ──────────────────────────────
        if at_bol {
            at_bol = false;
            let mut spaces = 0usize;
            let mut p = pos;
            while p < chars.len() {
                match chars[p] {
                    ' ' => { spaces += 1; p += 1; }
                    '\t' => { spaces += 8; p += 1; }
                    _ => break,
                }
            }
            // blank line or comment line — skip indent, don't advance pos
            if p >= chars.len() || chars[p] == '\n' || chars[p] == '#' {
                // just continue normally without emitting indent tokens
            } else if paren_depth == 0 {
                pos = p; // consume the spaces
                let cur = *indent_stack.last().unwrap();
                if spaces > cur {
                    indent_stack.push(spaces);
                    out.push((Tok::Indent, line));
                } else if spaces < cur {
                    while *indent_stack.last().unwrap_or(&0) > spaces {
                        indent_stack.pop();
                        out.push((Tok::Dedent, line));
                    }
                    if *indent_stack.last().unwrap_or(&0) != spaces {
                        return Err(PyError::syntax("inconsistent indentation", line));
                    }
                }
                continue;
            } else {
                pos = p;
                continue;
            }
        }

        match ch {
            '\n' => {
                pos += 1;
                line += 1;
                if paren_depth == 0 && !out.is_empty()
                    && !matches!(out.last(), Some((Tok::Newline, _)))
                {
                    out.push((Tok::Newline, line - 1));
                }
                at_bol = true;
            }
            '\r' => { pos += 1; }
            ' ' | '\t' => { pos += 1; }
            '#' => { while pos < chars.len() && chars[pos] != '\n' { pos += 1; } }
            '\\' => {
                pos += 1;
                if chars.get(pos) == Some(&'\n') { pos += 1; line += 1; }
            }
            // ── string literals ──────────────────────────────────────────
            '"' | '\'' => {
                let q = ch;
                pos += 1;
                let triple = chars.get(pos) == Some(&q) && chars.get(pos + 1) == Some(&q);
                if triple { pos += 2; }
                let mut s = String::new();
                loop {
                    if pos >= chars.len() {
                        return Err(PyError::syntax("unterminated string", line));
                    }
                    if triple {
                        if chars.get(pos) == Some(&q)
                            && chars.get(pos + 1) == Some(&q)
                            && chars.get(pos + 2) == Some(&q)
                        {
                            pos += 3;
                            break;
                        }
                    } else {
                        if chars[pos] == q { pos += 1; break; }
                        if chars[pos] == '\n' {
                            return Err(PyError::syntax("unterminated string", line));
                        }
                    }
                    if chars[pos] == '\\' {
                        pos += 1;
                        if pos >= chars.len() { return Err(PyError::syntax("bad escape", line)); }
                        match chars[pos] {
                            'n' => { s.push('\n'); pos += 1; }
                            't' => { s.push('\t'); pos += 1; }
                            'r' => { s.push('\r'); pos += 1; }
                            '\\' => { s.push('\\'); pos += 1; }
                            '\'' => { s.push('\''); pos += 1; }
                            '"' => { s.push('"'); pos += 1; }
                            c => { s.push('\\'); s.push(c); pos += 1; }
                        }
                    } else {
                        if chars[pos] == '\n' { line += 1; }
                        s.push(chars[pos]);
                        pos += 1;
                    }
                }
                out.push((Tok::Str(s), line));
            }
            // ── numbers ──────────────────────────────────────────────────
            '0'..='9' => {
                let start = pos;
                let mut is_float = false;
                pos += 1;
                while pos < chars.len() && chars[pos].is_ascii_digit() { pos += 1; }
                if pos < chars.len() && chars[pos] == '.'
                    && chars.get(pos + 1).map(|c| c.is_ascii_digit()).unwrap_or(false)
                {
                    is_float = true;
                    pos += 1;
                    while pos < chars.len() && chars[pos].is_ascii_digit() { pos += 1; }
                }
                let s: String = chars[start..pos].iter().collect();
                if is_float {
                    out.push((Tok::Float(s.parse().unwrap()), line));
                } else {
                    out.push((Tok::Int(s.parse().map_err(|_| PyError::syntax("bad int", line))?), line));
                }
            }
            // ── identifiers / keywords ────────────────────────────────────
            'a'..='z' | 'A'..='Z' | '_' => {
                let start = pos;
                pos += 1;
                while pos < chars.len() && (chars[pos].is_alphanumeric() || chars[pos] == '_') {
                    pos += 1;
                }
                let id: String = chars[start..pos].iter().collect();
                let tok = match id.as_str() {
                    "if" => Tok::If, "elif" => Tok::Elif, "else" => Tok::Else,
                    "while" => Tok::While, "for" => Tok::For, "in" => Tok::In,
                    "def" => Tok::Def, "return" => Tok::Return,
                    "pass" => Tok::Pass, "break" => Tok::Break, "continue" => Tok::Continue,
                    "and" => Tok::And, "or" => Tok::Or, "not" => Tok::Not,
                    "True" => Tok::True, "False" => Tok::False, "None" => Tok::None_,
                    _ => Tok::Name(id),
                };
                out.push((tok, line));
            }
            // ── operators ─────────────────────────────────────────────────
            '+' => {
                pos += 1;
                if chars.get(pos) == Some(&'=') { pos += 1; out.push((Tok::PlusEq, line)); }
                else { out.push((Tok::Plus, line)); }
            }
            '-' => {
                pos += 1;
                if chars.get(pos) == Some(&'=') { pos += 1; out.push((Tok::MinusEq, line)); }
                else { out.push((Tok::Minus, line)); }
            }
            '*' => {
                pos += 1;
                if chars.get(pos) == Some(&'*') { pos += 1; out.push((Tok::DStar, line)); }
                else if chars.get(pos) == Some(&'=') { pos += 1; out.push((Tok::StarEq, line)); }
                else { out.push((Tok::Star, line)); }
            }
            '/' => {
                pos += 1;
                if chars.get(pos) == Some(&'/') { pos += 1; out.push((Tok::DSlash, line)); }
                else if chars.get(pos) == Some(&'=') { pos += 1; out.push((Tok::SlashEq, line)); }
                else { out.push((Tok::Slash, line)); }
            }
            '%' => { pos += 1; out.push((Tok::Percent, line)); }
            '=' => {
                pos += 1;
                if chars.get(pos) == Some(&'=') { pos += 1; out.push((Tok::Eq, line)); }
                else { out.push((Tok::Assign, line)); }
            }
            '!' => {
                pos += 1;
                if chars.get(pos) == Some(&'=') { pos += 1; out.push((Tok::Ne, line)); }
                else { return Err(PyError::syntax("unexpected '!'", line)); }
            }
            '<' => {
                pos += 1;
                if chars.get(pos) == Some(&'=') { pos += 1; out.push((Tok::Le, line)); }
                else { out.push((Tok::Lt, line)); }
            }
            '>' => {
                pos += 1;
                if chars.get(pos) == Some(&'=') { pos += 1; out.push((Tok::Ge, line)); }
                else { out.push((Tok::Gt, line)); }
            }
            '(' => { pos += 1; paren_depth += 1; out.push((Tok::LParen, line)); }
            ')' => { pos += 1; if paren_depth > 0 { paren_depth -= 1; } out.push((Tok::RParen, line)); }
            '[' => { pos += 1; paren_depth += 1; out.push((Tok::LBracket, line)); }
            ']' => { pos += 1; if paren_depth > 0 { paren_depth -= 1; } out.push((Tok::RBracket, line)); }
            '{' => { pos += 1; paren_depth += 1; out.push((Tok::LBrace, line)); }
            '}' => { pos += 1; if paren_depth > 0 { paren_depth -= 1; } out.push((Tok::RBrace, line)); }
            ',' => { pos += 1; out.push((Tok::Comma, line)); }
            ':' => { pos += 1; out.push((Tok::Colon, line)); }
            '.' => { pos += 1; out.push((Tok::Dot, line)); }
            c => {
                return Err(PyError::syntax(format!("unexpected character '{}'", c), line));
            }
        }
    }

    Ok(out)
}
