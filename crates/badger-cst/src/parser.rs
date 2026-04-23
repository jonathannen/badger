//! Recursive-descent parser for the current Badger corpus.

use rowan::GreenNodeBuilder;

use crate::kinds::SyntaxKind;
use crate::lexer::{tokenize, Token};
use crate::{Parse, ParseError};

pub(crate) fn parse(source: &str) -> Result<Parse, ParseError> {
    Parser::new(tokenize(source)?).parse()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    builder: GreenNodeBuilder<'static>,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            pos: 0,
            builder: GreenNodeBuilder::new(),
        }
    }

    fn parse(mut self) -> Result<Parse, ParseError> {
        self.start_node(SyntaxKind::Root);
        while !self.at_eof() {
            if self.current_kind().is_some_and(SyntaxKind::is_trivia) {
                self.bump();
                continue;
            }
            self.parse_item()?;
        }
        self.finish_node();
        Ok(Parse {
            green: self.builder.finish(),
        })
    }

    fn parse_item(&mut self) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::Item);
        let result = match self.nth_nontrivia_kind(0) {
            Some(SyntaxKind::LBrace) => self.parse_import_binding(),
            Some(SyntaxKind::PubKw)
                if self.nth_nontrivia_kind(1) == Some(SyntaxKind::BuiltinImport) =>
            {
                self.parse_reexport()
            }
            Some(SyntaxKind::PubKw) if self.nth_nontrivia_kind(1) == Some(SyntaxKind::TypeKw) => {
                self.parse_type_alias()
            }
            Some(SyntaxKind::PubKw) if self.nth_nontrivia_kind(1) == Some(SyntaxKind::StructKw) => {
                self.parse_struct_decl()
            }
            Some(SyntaxKind::PubKw) if self.nth_nontrivia_kind(1) == Some(SyntaxKind::EnumKw) => {
                self.parse_enum_decl()
            }
            Some(SyntaxKind::PubKw)
                if self.nth_nontrivia_kind(1) == Some(SyntaxKind::InterfaceKw) =>
            {
                self.parse_interface_decl()
            }
            Some(SyntaxKind::PubKw) => self.parse_binding(),
            Some(kind) => {
                Err(self.error_at_next(format!("unexpected top-level token `{:?}`", kind)))
            }
            None => Ok(()),
        };
        self.finish_node();
        result
    }

    fn parse_import_binding(&mut self) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::ImportBinding);
        self.expect(SyntaxKind::LBrace, "expected `{`")?;
        loop {
            self.expect_ident("expected imported name")?;
            if !self.eat(SyntaxKind::Comma) {
                break;
            }
        }
        self.expect(SyntaxKind::RBrace, "expected `}`")?;
        self.expect(SyntaxKind::Eq, "expected `=`")?;
        self.parse_expr()?;
        self.expect(SyntaxKind::Semi, "expected `;`")?;
        self.finish_node();
        Ok(())
    }

    fn parse_reexport(&mut self) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::ReExport);
        self.expect(SyntaxKind::PubKw, "expected `pub`")?;
        self.parse_expr()?;
        self.expect(SyntaxKind::Semi, "expected `;`")?;
        self.finish_node();
        Ok(())
    }

    fn parse_type_alias(&mut self) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::TypeAlias);
        self.expect(SyntaxKind::PubKw, "expected `pub`")?;
        self.expect(SyntaxKind::TypeKw, "expected `type`")?;
        self.expect_ident("expected type alias name")?;
        if self.at_nontrivia(SyntaxKind::Lt) {
            self.parse_generic_params(GenericParamMode::BareNames)?;
        }
        self.expect(SyntaxKind::Eq, "expected `=`")?;
        self.parse_type()?;
        self.expect(SyntaxKind::Semi, "expected `;`")?;
        self.finish_node();
        Ok(())
    }

    fn parse_struct_decl(&mut self) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::StructDecl);
        self.expect(SyntaxKind::PubKw, "expected `pub`")?;
        self.expect(SyntaxKind::StructKw, "expected `struct`")?;
        self.expect_ident("expected struct name")?;
        if self.at_nontrivia(SyntaxKind::Lt) {
            self.parse_generic_params(GenericParamMode::BareNames)?;
        }
        self.expect(SyntaxKind::LBrace, "expected `{`")?;
        while !self.at_nontrivia(SyntaxKind::RBrace) {
            if self.at_eof() {
                return Err(self.error_at_next("expected `}`".to_string()));
            }
            self.parse_field()?;
        }
        self.expect(SyntaxKind::RBrace, "expected `}`")?;
        self.eat(SyntaxKind::Semi);
        self.finish_node();
        Ok(())
    }

    fn parse_field(&mut self) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::Field);
        self.expect_ident("expected field name")?;
        self.expect(SyntaxKind::Colon, "expected `:`")?;
        self.parse_type()?;
        self.expect(SyntaxKind::Semi, "expected `;`")?;
        self.finish_node();
        Ok(())
    }

    fn parse_enum_decl(&mut self) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::EnumDecl);
        self.expect(SyntaxKind::PubKw, "expected `pub`")?;
        self.expect(SyntaxKind::EnumKw, "expected `enum`")?;
        self.expect_ident("expected enum name")?;
        if self.at_nontrivia(SyntaxKind::Lt) {
            self.parse_generic_params(GenericParamMode::BareNames)?;
        }
        self.expect(SyntaxKind::LBrace, "expected `{`")?;
        while !self.at_nontrivia(SyntaxKind::RBrace) {
            if self.at_eof() {
                return Err(self.error_at_next("expected `}`".to_string()));
            }
            self.parse_variant()?;
        }
        self.expect(SyntaxKind::RBrace, "expected `}`")?;
        self.eat(SyntaxKind::Semi);
        self.finish_node();
        Ok(())
    }

    fn parse_variant(&mut self) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::Variant);
        self.expect_ident("expected variant name")?;
        if self.eat(SyntaxKind::LParen) {
            if !self.at_nontrivia(SyntaxKind::RParen) {
                loop {
                    self.parse_type()?;
                    if !self.eat(SyntaxKind::Comma) {
                        break;
                    }
                }
            }
            self.expect(SyntaxKind::RParen, "expected `)`")?;
        }
        self.expect(SyntaxKind::Semi, "expected `;`")?;
        self.finish_node();
        Ok(())
    }

    fn parse_interface_decl(&mut self) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::InterfaceDecl);
        self.expect(SyntaxKind::PubKw, "expected `pub`")?;
        self.expect(SyntaxKind::InterfaceKw, "expected `interface`")?;
        self.expect_ident("expected interface name")?;
        if self.at_nontrivia(SyntaxKind::Lt) {
            self.parse_generic_params(GenericParamMode::ShapeOrName)?;
        }
        if self.at_nontrivia(SyntaxKind::ExtendsKw) {
            self.parse_extends_clause()?;
        }
        self.expect(SyntaxKind::LBrace, "expected `{`")?;
        while !self.at_nontrivia(SyntaxKind::RBrace) {
            if self.at_eof() {
                return Err(self.error_at_next("expected `}`".to_string()));
            }
            self.parse_method_sig()?;
        }
        self.expect(SyntaxKind::RBrace, "expected `}`")?;
        self.eat(SyntaxKind::Semi);
        self.finish_node();
        Ok(())
    }

    fn parse_extends_clause(&mut self) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::ExtendsClause);
        self.expect(SyntaxKind::ExtendsKw, "expected `extends`")?;
        loop {
            self.parse_type()?;
            if !self.eat(SyntaxKind::Comma) {
                break;
            }
        }
        self.finish_node();
        Ok(())
    }

    fn parse_method_sig(&mut self) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::MethodSig);
        self.expect_ident("expected method name")?;
        if self.at_nontrivia(SyntaxKind::Lt) {
            self.parse_generic_params(GenericParamMode::BareNames)?;
        }
        self.parse_param_list(ParamMode::Typed)?;
        self.parse_return_and_effect_tail()?;
        if self.eat(SyntaxKind::Semi) {
            self.finish_node();
            return Ok(());
        }
        self.parse_block()?;
        self.eat(SyntaxKind::Semi);
        self.finish_node();
        Ok(())
    }

    fn parse_binding(&mut self) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::Binding);
        self.expect(SyntaxKind::PubKw, "expected `pub`")?;
        self.expect_ident("expected binding name")?;
        if self.eat(SyntaxKind::Colon) {
            self.parse_type()?;
        }
        self.expect(SyntaxKind::Eq, "expected `=`")?;
        self.parse_expr()?;
        self.eat(SyntaxKind::Semi);
        self.finish_node();
        Ok(())
    }

    fn parse_generic_params(&mut self, mode: GenericParamMode) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::GenericParamList);
        self.expect(SyntaxKind::Lt, "expected `<`")?;
        if !self.at_nontrivia(SyntaxKind::Gt) {
            loop {
                match mode {
                    GenericParamMode::BareNames => {
                        self.expect_ident("expected generic parameter")?;
                    }
                    GenericParamMode::ShapeOrName => {
                        self.parse_generic_slot()?;
                    }
                }
                if !self.eat(SyntaxKind::Comma) {
                    break;
                }
            }
        }
        self.expect(SyntaxKind::Gt, "expected `>`")?;
        self.finish_node();
        Ok(())
    }

    fn parse_generic_slot(&mut self) -> Result<(), ParseError> {
        // Interface generic slots accept either a bare name (`<T>`) or a shape
        // pattern such as `<[]Type>` / `<Iterator<Type>>`.
        match self.nth_nontrivia_kind(0) {
            Some(SyntaxKind::LBracket) => self.parse_type(),
            Some(SyntaxKind::Ident) | Some(SyntaxKind::SelfTypeKw) => {
                if self.nth_nontrivia_kind(1) == Some(SyntaxKind::Lt) {
                    self.parse_type()
                } else {
                    self.expect_name_like("expected generic slot")
                }
            }
            _ => Err(self.error_at_next("expected generic slot".to_string())),
        }
    }

    fn parse_type(&mut self) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::Type);
        let result = match self.nth_nontrivia_kind(0) {
            Some(SyntaxKind::FnKw) => self.parse_function_type_inner(),
            Some(SyntaxKind::LBracket) => {
                self.expect(SyntaxKind::LBracket, "expected `[`")?;
                if self.at_nontrivia(SyntaxKind::RBracket) {
                    self.expect(SyntaxKind::RBracket, "expected `]`")?;
                    self.parse_type()
                } else {
                    if self.at_nontrivia(SyntaxKind::Int) {
                        self.expect(SyntaxKind::Int, "expected array length")?;
                    }
                    self.expect(SyntaxKind::RBracket, "expected `]`")?;
                    self.parse_type()
                }
            }
            Some(SyntaxKind::LParen) => {
                self.expect(SyntaxKind::LParen, "expected `(`")?;
                if !self.at_nontrivia(SyntaxKind::RParen) {
                    loop {
                        self.parse_type()?;
                        if !self.eat(SyntaxKind::Comma) {
                            break;
                        }
                    }
                }
                self.expect(SyntaxKind::RParen, "expected `)`")?;
                Ok(())
            }
            Some(SyntaxKind::Ident) | Some(SyntaxKind::SelfTypeKw) => {
                self.expect_name_like("expected type")?;
                while self.eat(SyntaxKind::Dot) {
                    self.expect_ident("expected path segment")?;
                }
                if self.at_nontrivia(SyntaxKind::Lt) {
                    self.parse_generic_args()?;
                }
                Ok(())
            }
            _ => Err(self.error_at_next("expected type".to_string())),
        };
        self.finish_node();
        result
    }

    fn parse_function_type_inner(&mut self) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::FunctionType);
        self.expect(SyntaxKind::FnKw, "expected `fn`")?;
        self.parse_param_list(ParamMode::Typed)?;
        self.parse_return_and_effect_tail()?;
        self.finish_node();
        Ok(())
    }

    fn parse_generic_args(&mut self) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::GenericArgList);
        self.expect(SyntaxKind::Lt, "expected `<`")?;
        if !self.at_nontrivia(SyntaxKind::Gt) {
            loop {
                self.parse_type()?;
                if !self.eat(SyntaxKind::Comma) {
                    break;
                }
            }
        }
        self.expect(SyntaxKind::Gt, "expected `>`")?;
        self.finish_node();
        Ok(())
    }

    fn parse_param_list(&mut self, mode: ParamMode) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::ParamList);
        self.expect(SyntaxKind::LParen, "expected `(`")?;
        if !self.at_nontrivia(SyntaxKind::RParen) {
            loop {
                self.parse_param(mode)?;
                if !self.eat(SyntaxKind::Comma) {
                    break;
                }
            }
        }
        self.expect(SyntaxKind::RParen, "expected `)`")?;
        self.finish_node();
        Ok(())
    }

    fn parse_param(&mut self, mode: ParamMode) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::Param);
        self.expect_param_name("expected parameter name")?;
        if self.eat(SyntaxKind::Colon) {
            self.parse_type()?;
        } else if matches!(mode, ParamMode::Typed) {
            return Err(self.error_at_next("expected `:`".to_string()));
        }
        self.finish_node();
        Ok(())
    }

    fn parse_return_and_effect_tail(&mut self) -> Result<(), ParseError> {
        // `!effect(...)` is parsed as a sibling of the return type rather than
        // folded into the type node itself.
        if self.eat(SyntaxKind::Arrow) {
            self.parse_type()?;
            if self.at_nontrivia(SyntaxKind::EffectMarker) {
                self.parse_effect_tail()?;
            }
        }
        Ok(())
    }

    fn parse_effect_tail(&mut self) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::EffectTail);
        self.expect(SyntaxKind::EffectMarker, "expected `!effect`")?;
        self.expect(SyntaxKind::LParen, "expected `(`")?;
        self.expect_ident("expected effect name")?;
        self.expect(SyntaxKind::RParen, "expected `)`")?;
        self.finish_node();
        Ok(())
    }

    fn parse_expr(&mut self) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::Expr);
        let result = self.parse_expr_inner();
        self.finish_node();
        result
    }

    fn parse_expr_inner(&mut self) -> Result<(), ParseError> {
        match self.nth_nontrivia_kind(0) {
            Some(SyntaxKind::FnKw) => self.parse_fn_expr(),
            Some(SyntaxKind::LBrace) => self.parse_block(),
            Some(SyntaxKind::BuiltinImport) => self.parse_import_expr(),
            Some(SyntaxKind::Ident)
            | Some(SyntaxKind::SelfKw)
            | Some(SyntaxKind::TrueKw)
            | Some(SyntaxKind::FalseKw)
            | Some(SyntaxKind::Int)
            | Some(SyntaxKind::String)
            | Some(SyntaxKind::Char) => self.bump_nontrivia(),
            Some(kind) => {
                return Err(self.error_at_next(format!("unexpected expression token `{:?}`", kind)))
            }
            None => return Err(self.error_at_eof("expected expression".to_string())),
        }?;

        loop {
            if self.at_nontrivia(SyntaxKind::Dot) {
                self.expect(SyntaxKind::Dot, "expected `.`")?;
                self.expect_ident("expected field or method name")?;
                continue;
            }
            if self.at_nontrivia(SyntaxKind::LParen) {
                self.parse_arg_list()?;
                continue;
            }
            break;
        }

        Ok(())
    }

    fn parse_import_expr(&mut self) -> Result<(), ParseError> {
        self.expect(SyntaxKind::BuiltinImport, "expected `@import`")?;
        self.parse_arg_list()?;
        while self.at_nontrivia(SyntaxKind::Dot) {
            self.expect(SyntaxKind::Dot, "expected `.`")?;
            self.expect_ident("expected path segment")?;
        }
        Ok(())
    }

    fn parse_fn_expr(&mut self) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::FnExpr);
        self.expect(SyntaxKind::FnKw, "expected `fn`")?;
        self.parse_param_list(ParamMode::Inferable)?;
        self.parse_return_and_effect_tail()?;
        if self.eat(SyntaxKind::FatArrow) {
            self.parse_expr()?;
        } else {
            self.parse_block()?;
        }
        self.finish_node();
        Ok(())
    }

    fn parse_arg_list(&mut self) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::ArgList);
        self.expect(SyntaxKind::LParen, "expected `(`")?;
        if !self.at_nontrivia(SyntaxKind::RParen) {
            loop {
                self.parse_expr()?;
                if !self.eat(SyntaxKind::Comma) {
                    break;
                }
            }
        }
        self.expect(SyntaxKind::RParen, "expected `)`")?;
        self.finish_node();
        Ok(())
    }

    fn parse_block(&mut self) -> Result<(), ParseError> {
        self.start_node(SyntaxKind::Block);
        self.expect(SyntaxKind::LBrace, "expected `{`")?;
        while !self.at_nontrivia(SyntaxKind::RBrace) {
            if self.at_eof() {
                return Err(self.error_at_next("expected `}`".to_string()));
            }
            self.parse_expr()?;
            if !self.eat(SyntaxKind::Semi) {
                break;
            }
        }
        self.expect(SyntaxKind::RBrace, "expected `}`")?;
        self.finish_node();
        Ok(())
    }

    fn start_node(&mut self, kind: SyntaxKind) {
        self.builder.start_node(kind.into());
    }

    fn finish_node(&mut self) {
        self.builder.finish_node();
    }

    fn bump(&mut self) {
        let token = &self.tokens[self.pos];
        self.builder.token(token.kind.into(), &token.text);
        self.pos += 1;
    }

    fn bump_nontrivia(&mut self) -> Result<(), ParseError> {
        self.consume_trivia();
        if self.at_eof() {
            return Err(self.error_at_eof("unexpected end of file".to_string()));
        }
        self.bump();
        Ok(())
    }

    fn consume_trivia(&mut self) {
        while self.current_kind().is_some_and(SyntaxKind::is_trivia) {
            self.bump();
        }
    }

    fn expect(&mut self, kind: SyntaxKind, message: &str) -> Result<(), ParseError> {
        self.consume_trivia();
        if self.current_kind() == Some(kind) {
            self.bump();
            Ok(())
        } else {
            Err(self.error_at_next(message.to_string()))
        }
    }

    fn eat(&mut self, kind: SyntaxKind) -> bool {
        self.consume_trivia();
        if self.current_kind() == Some(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect_ident(&mut self, message: &str) -> Result<(), ParseError> {
        self.expect(SyntaxKind::Ident, message)
    }

    fn expect_name_like(&mut self, message: &str) -> Result<(), ParseError> {
        self.consume_trivia();
        match self.current_kind() {
            Some(SyntaxKind::Ident) | Some(SyntaxKind::SelfTypeKw) => {
                self.bump();
                Ok(())
            }
            _ => Err(self.error_at_next(message.to_string())),
        }
    }

    fn expect_param_name(&mut self, message: &str) -> Result<(), ParseError> {
        self.consume_trivia();
        match self.current_kind() {
            Some(SyntaxKind::Ident) | Some(SyntaxKind::SelfKw) => {
                self.bump();
                Ok(())
            }
            _ => Err(self.error_at_next(message.to_string())),
        }
    }

    fn at_nontrivia(&self, kind: SyntaxKind) -> bool {
        self.nth_nontrivia_kind(0) == Some(kind)
    }

    fn current_kind(&self) -> Option<SyntaxKind> {
        self.tokens.get(self.pos).map(|token| token.kind)
    }

    fn nth_nontrivia_kind(&self, mut n: usize) -> Option<SyntaxKind> {
        for token in &self.tokens[self.pos..] {
            if token.kind.is_trivia() {
                continue;
            }
            if n == 0 {
                return Some(token.kind);
            }
            n -= 1;
        }
        None
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn error_at_next(&self, message: String) -> ParseError {
        if let Some(token) = self.tokens[self.pos..]
            .iter()
            .find(|token| !token.kind.is_trivia())
        {
            ParseError {
                offset: token.offset,
                message,
            }
        } else {
            self.error_at_eof(message)
        }
    }

    fn error_at_eof(&self, message: String) -> ParseError {
        ParseError {
            offset: self
                .tokens
                .last()
                .map(|token| token.offset + token.text.len())
                .unwrap_or(0),
            message,
        }
    }
}

#[derive(Clone, Copy)]
enum GenericParamMode {
    BareNames,
    ShapeOrName,
}

#[derive(Clone, Copy)]
enum ParamMode {
    Typed,
    Inferable,
}
