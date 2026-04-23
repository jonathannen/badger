//! Syntax kinds and Rowan language glue for the Badger CST.

use rowan::Language;

/// Token and node kinds used by the lossless Badger CST.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum SyntaxKind {
    Root,
    Item,
    ImportBinding,
    ReExport,
    TypeAlias,
    StructDecl,
    EnumDecl,
    InterfaceDecl,
    Binding,
    GenericParamList,
    GenericArgList,
    ExtendsClause,
    ParamList,
    Param,
    Field,
    Variant,
    MethodSig,
    Type,
    FunctionType,
    EffectTail,
    Expr,
    FnExpr,
    Block,
    ArgList,
    Error,
    Whitespace,
    LineComment,
    DocComment,
    PubKw,
    FnKw,
    StructKw,
    EnumKw,
    InterfaceKw,
    ImplementKw,
    AsKw,
    ExtendsKw,
    MatchKw,
    TypeKw,
    SelfKw,
    SelfTypeKw,
    TrueKw,
    FalseKw,
    BuiltinImport,
    BuiltinRecurse,
    EffectMarker,
    DependMarker,
    Ident,
    Int,
    String,
    Char,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Lt,
    Gt,
    Comma,
    Semi,
    Colon,
    Dot,
    Arrow,
    FatArrow,
    Eq,
    Question,
    PipeGt,
    Plus,
    Star,
    Le,
}

impl SyntaxKind {
    pub(crate) fn from_raw(raw: u16) -> Self {
        match raw {
            0 => Self::Root,
            1 => Self::Item,
            2 => Self::ImportBinding,
            3 => Self::ReExport,
            4 => Self::TypeAlias,
            5 => Self::StructDecl,
            6 => Self::EnumDecl,
            7 => Self::InterfaceDecl,
            8 => Self::Binding,
            9 => Self::GenericParamList,
            10 => Self::GenericArgList,
            11 => Self::ExtendsClause,
            12 => Self::ParamList,
            13 => Self::Param,
            14 => Self::Field,
            15 => Self::Variant,
            16 => Self::MethodSig,
            17 => Self::Type,
            18 => Self::FunctionType,
            19 => Self::EffectTail,
            20 => Self::Expr,
            21 => Self::FnExpr,
            22 => Self::Block,
            23 => Self::ArgList,
            24 => Self::Error,
            25 => Self::Whitespace,
            26 => Self::LineComment,
            27 => Self::DocComment,
            28 => Self::PubKw,
            29 => Self::FnKw,
            30 => Self::StructKw,
            31 => Self::EnumKw,
            32 => Self::InterfaceKw,
            33 => Self::ImplementKw,
            34 => Self::AsKw,
            35 => Self::ExtendsKw,
            36 => Self::MatchKw,
            37 => Self::TypeKw,
            38 => Self::SelfKw,
            39 => Self::SelfTypeKw,
            40 => Self::TrueKw,
            41 => Self::FalseKw,
            42 => Self::BuiltinImport,
            43 => Self::BuiltinRecurse,
            44 => Self::EffectMarker,
            45 => Self::DependMarker,
            46 => Self::Ident,
            47 => Self::Int,
            48 => Self::String,
            49 => Self::Char,
            50 => Self::LParen,
            51 => Self::RParen,
            52 => Self::LBrace,
            53 => Self::RBrace,
            54 => Self::LBracket,
            55 => Self::RBracket,
            56 => Self::Lt,
            57 => Self::Gt,
            58 => Self::Comma,
            59 => Self::Semi,
            60 => Self::Colon,
            61 => Self::Dot,
            62 => Self::Arrow,
            63 => Self::FatArrow,
            64 => Self::Eq,
            65 => Self::Question,
            66 => Self::PipeGt,
            67 => Self::Plus,
            68 => Self::Star,
            69 => Self::Le,
            _ => Self::Error,
        }
    }

    pub(crate) fn is_trivia(self) -> bool {
        matches!(
            self,
            Self::Whitespace | Self::LineComment | Self::DocComment
        )
    }
}

impl From<SyntaxKind> for rowan::SyntaxKind {
    fn from(value: SyntaxKind) -> Self {
        rowan::SyntaxKind(value as u16)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BadgerLanguage {}

impl Language for BadgerLanguage {
    type Kind = SyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        SyntaxKind::from_raw(raw.0)
    }

    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        kind.into()
    }
}

/// The untyped red syntax node used by the public CST API.
pub type SyntaxNode = rowan::SyntaxNode<BadgerLanguage>;
