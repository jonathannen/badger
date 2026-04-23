//! Lexer for Badger source that preserves every byte as a token.

use crate::kinds::SyntaxKind;
use crate::ParseError;

#[derive(Debug, Clone)]
pub(crate) struct Token {
    pub(crate) kind: SyntaxKind,
    pub(crate) text: String,
    pub(crate) offset: usize,
}

pub(crate) fn tokenize(source: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let mut index = 0;

    while index < source.len() {
        let rest = &source[index..];
        let mut chars = rest.chars();
        let ch = chars.next().unwrap();

        if ch.is_whitespace() {
            let end = index
                + rest
                    .char_indices()
                    .find(|(_, c)| !c.is_whitespace())
                    .map(|(i, _)| i)
                    .unwrap_or(rest.len());
            tokens.push(Token {
                kind: SyntaxKind::Whitespace,
                text: source[index..end].to_string(),
                offset: index,
            });
            index = end;
            continue;
        }

        // Comments and whitespace remain in the token stream so serializing the
        // CST can reproduce the original source exactly.
        if rest.starts_with("///") {
            let end = index + rest.find('\n').unwrap_or(rest.len());
            tokens.push(Token {
                kind: SyntaxKind::DocComment,
                text: source[index..end].to_string(),
                offset: index,
            });
            index = end;
            continue;
        }

        if rest.starts_with("//") {
            let end = index + rest.find('\n').unwrap_or(rest.len());
            tokens.push(Token {
                kind: SyntaxKind::LineComment,
                text: source[index..end].to_string(),
                offset: index,
            });
            index = end;
            continue;
        }

        if rest.starts_with("@import") && !is_ident_continue(rest[7..].chars().next()) {
            tokens.push(Token {
                kind: SyntaxKind::BuiltinImport,
                text: "@import".to_string(),
                offset: index,
            });
            index += 7;
            continue;
        }

        if rest.starts_with("@recurse") && !is_ident_continue(rest[8..].chars().next()) {
            tokens.push(Token {
                kind: SyntaxKind::BuiltinRecurse,
                text: "@recurse".to_string(),
                offset: index,
            });
            index += 8;
            continue;
        }

        if rest.starts_with("!effect") && !is_ident_continue(rest[7..].chars().next()) {
            tokens.push(Token {
                kind: SyntaxKind::EffectMarker,
                text: "!effect".to_string(),
                offset: index,
            });
            index += 7;
            continue;
        }

        if rest.starts_with("!depend") && !is_ident_continue(rest[7..].chars().next()) {
            tokens.push(Token {
                kind: SyntaxKind::DependMarker,
                text: "!depend".to_string(),
                offset: index,
            });
            index += 7;
            continue;
        }

        if let Some((kind, text)) = lex_punctuation(rest) {
            tokens.push(Token {
                kind,
                text: text.to_string(),
                offset: index,
            });
            index += text.len();
            continue;
        }

        if ch == '"' {
            let end = lex_quoted(source, index, '"')?;
            tokens.push(Token {
                kind: SyntaxKind::String,
                text: source[index..end].to_string(),
                offset: index,
            });
            index = end;
            continue;
        }

        if ch == '\'' {
            let end = lex_quoted(source, index, '\'')?;
            tokens.push(Token {
                kind: SyntaxKind::Char,
                text: source[index..end].to_string(),
                offset: index,
            });
            index = end;
            continue;
        }

        if ch.is_ascii_digit() {
            let end = index
                + rest
                    .char_indices()
                    .find(|(_, c)| !c.is_ascii_digit())
                    .map(|(i, _)| i)
                    .unwrap_or(rest.len());
            tokens.push(Token {
                kind: SyntaxKind::Int,
                text: source[index..end].to_string(),
                offset: index,
            });
            index = end;
            continue;
        }

        if is_ident_start(ch) {
            let end = index
                + rest
                    .char_indices()
                    .find(|(_, c)| !is_ident_continue(Some(*c)))
                    .map(|(i, _)| i)
                    .unwrap_or(rest.len());
            let text = &source[index..end];
            tokens.push(Token {
                kind: keyword_kind(text).unwrap_or(SyntaxKind::Ident),
                text: text.to_string(),
                offset: index,
            });
            index = end;
            continue;
        }

        return Err(ParseError {
            offset: index,
            message: format!("unexpected character `{ch}`"),
        });
    }

    Ok(tokens)
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: Option<char>) -> bool {
    matches!(ch, Some('_')) || ch.is_some_and(|c| c.is_ascii_alphanumeric())
}

fn keyword_kind(text: &str) -> Option<SyntaxKind> {
    Some(match text {
        "pub" => SyntaxKind::PubKw,
        "fn" => SyntaxKind::FnKw,
        "struct" => SyntaxKind::StructKw,
        "enum" => SyntaxKind::EnumKw,
        "interface" => SyntaxKind::InterfaceKw,
        "implement" => SyntaxKind::ImplementKw,
        "as" => SyntaxKind::AsKw,
        "extends" => SyntaxKind::ExtendsKw,
        "match" => SyntaxKind::MatchKw,
        "type" => SyntaxKind::TypeKw,
        "self" => SyntaxKind::SelfKw,
        "Self" => SyntaxKind::SelfTypeKw,
        "true" => SyntaxKind::TrueKw,
        "false" => SyntaxKind::FalseKw,
        _ => return None,
    })
}

fn lex_punctuation(rest: &str) -> Option<(SyntaxKind, &'static str)> {
    [
        (SyntaxKind::Arrow, "->"),
        (SyntaxKind::FatArrow, "=>"),
        (SyntaxKind::Le, "<="),
        (SyntaxKind::PipeGt, "|>"),
        (SyntaxKind::LParen, "("),
        (SyntaxKind::RParen, ")"),
        (SyntaxKind::LBrace, "{"),
        (SyntaxKind::RBrace, "}"),
        (SyntaxKind::LBracket, "["),
        (SyntaxKind::RBracket, "]"),
        (SyntaxKind::Lt, "<"),
        (SyntaxKind::Gt, ">"),
        (SyntaxKind::Comma, ","),
        (SyntaxKind::Semi, ";"),
        (SyntaxKind::Colon, ":"),
        (SyntaxKind::Dot, "."),
        (SyntaxKind::Eq, "="),
        (SyntaxKind::Question, "?"),
        (SyntaxKind::Plus, "+"),
        (SyntaxKind::Star, "*"),
    ]
    .into_iter()
    .find(|(_, punct)| rest.starts_with(*punct))
}

fn lex_quoted(source: &str, start: usize, quote: char) -> Result<usize, ParseError> {
    let mut escaped = false;
    for (idx, ch) in source[start + quote.len_utf8()..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            return Ok(start + quote.len_utf8() + idx + ch.len_utf8());
        }
    }
    Err(ParseError {
        offset: start,
        message: format!("unterminated {quote} literal"),
    })
}

#[cfg(test)]
mod tests {
    use super::tokenize;
    use crate::ParseError;
    use std::fs;
    use std::path::{Path, PathBuf};

    #[test]
    fn lexer_roundtrips_corpus() -> Result<(), ParseError> {
        for path in corpus_files() {
            let source = fs::read_to_string(&path).unwrap();
            let roundtrip: String = tokenize(&source)?
                .into_iter()
                .map(|token| token.text)
                .collect();
            assert_eq!(
                roundtrip,
                source,
                "lexer round-trip failed for {}",
                path.display()
            );
        }
        Ok(())
    }

    fn corpus_files() -> Vec<PathBuf> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let mut files = Vec::new();
        gather_badger_files(&root.join("examples"), &mut files);
        gather_badger_files(&root.join("lib/std"), &mut files);
        files.sort();
        files
    }

    fn gather_badger_files(dir: &Path, files: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                gather_badger_files(&path, files);
            } else if path.extension().is_some_and(|ext| ext == "badger") {
                files.push(path);
            }
        }
    }
}
