//! Lossless concrete syntax tree support for Badger source files.
//!
//! The primary entry point is [`parse`], which produces a CST that can be
//! serialized back to the original source byte-for-byte.

mod kinds;
mod lexer;
mod parser;

use rowan::GreenNode;

pub use kinds::{BadgerLanguage, SyntaxKind, SyntaxNode};

/// A strict parse failure.
///
/// Parsing stops at the first offending token; no partial tree is returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// Byte offset of the offending token in the source.
    pub offset: usize,
    /// Human-readable parse failure message.
    pub message: String,
}

/// A successfully parsed Badger concrete syntax tree.
pub struct Parse {
    pub(crate) green: GreenNode,
}

impl Parse {
    /// Returns the root red node for the parsed syntax tree.
    pub fn syntax(&self) -> SyntaxNode {
        SyntaxNode::new_root(self.green.clone())
    }

    /// Serializes the CST back to source text without normalization.
    pub fn serialize(&self) -> String {
        self.syntax()
            .descendants_with_tokens()
            .filter_map(|element| element.into_token())
            .map(|token| token.text().to_string())
            .collect()
    }
}

/// Parses Badger source into a lossless concrete syntax tree.
pub fn parse(source: &str) -> Result<Parse, ParseError> {
    parser::parse(source)
}
