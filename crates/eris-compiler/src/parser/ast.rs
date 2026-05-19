use std::fmt;
use std::ops::Add;
use crate::lexer::{SourceLocation};

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>
}

#[derive(Debug, Clone, PartialEq)]
pub struct Stmt {
    pub kind: StmtKind,
    pub location: SourceLocation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StmtKind {
    ExprStmt(Expr),
    VarDecl {
        mutable: bool,
        typ: Option<Type>,
        ident: Identifier,
        initializer: Box<Expr>,
    },
    Assignment {
        target: Expr,
        value: Expr,
    },
    FuncDecl {
        name: Identifier,
        parameters: Vec<Param>,
        return_type: Option<Type>,
        body: Expr
    },
    ReturnStmt(Option<Expr>),
    WhileLoop {
        condition: Expr,
        body: Box<Expr>,
    },
    Break,
    Continue,
    ForIn {
        var: Identifier,
        iterable: Box<Expr>,
        body: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    pub kind: TypeKind,
    pub location: SourceLocation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Int,
    Float,
    Char,
    Bool,
    Str,
    Array {
        element: Box<TypeKind>,
        size: usize,
    },
    Ref {
        inner: Box<TypeKind>,
        mutable: bool,
    },
}

impl fmt::Display for TypeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeKind::Int => write!(f, "int"),
            TypeKind::Float => write!(f, "float"),
            TypeKind::Char => write!(f, "char"),
            TypeKind::Bool => write!(f, "bool"),
            TypeKind::Str => write!(f, "str"),
            TypeKind::Array { element, size } => {
                if *size == 0 {
                    write!(f, "arr[{}]", element)
                } else {
                    write!(f, "arr[{}; {}]", element, size)
                }
            }
            TypeKind::Ref { inner, mutable: false } => write!(f, "&{}", inner),
            TypeKind::Ref { inner, mutable: true } => write!(f, "&mut {}", inner),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub location: SourceLocation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    IntLit(i64),
    FloatLit(f64),
    CharLit(char),
    StringLit(String),
    ArrayLit(Vec<Expr>),
    BoolTrue,
    BoolFalse,
    IdentExpr(Identifier),

    UnaryExpr {
        op: UnaryOp,
        expression: Box<Expr>,
    },
    BinaryExpr {
        left: Box<Expr>,
        op: BinOp,
        right: Box<Expr>
    },
    ParenExpr(Box<Expr>),
    TakeRef {
        mutable: bool,
        place: Box<Expr>,
    },
    Deref(Box<Expr>),
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
    },
    ArrayIndex {
        array: Box<Expr>,
        index: Box<Expr>,
    },
    TupleExpr(Vec<Expr>),
    BlockExpr {
        statements: Vec<Stmt>,
        ret: Option<Box<Expr>>
    },
    IfExpr {
        condition: Box<Expr>,
        // then_branch: Box<Expr>,
        then_branch: Option<Box<Expr>>,
        else_branch: Option<Box<Expr>>,
    },
    FunctionCall {
        function: Box<Identifier>,
        arguments: Vec<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    LNot,
    BNot,
    Neg,
    // Deref,
    // Addr,
    // Sizeof,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    // Bitwise
    BAnd,
    BOr,
    BXor,
    Shl,
    Shr,
    LAnd,
    LOr,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Ord, Eq, PartialEq, PartialOrd, Debug)]
pub enum Precedence {
    Lowest = 0,
    LogicalOr,  // ||
    LogicalAnd, // &&
    Equality,   // == !=
    Comparison, // < <= > >=
    // Bitwise ops sit above comparisons (Rust-style), not below them (C-style),
    // so `a & b == c` parses as `(a & b) == c`. This avoids the C precedence footgun.
    BitOr,      // |
    BitXor,     // ^
    BitAnd,     // &
    Shift,      // << >>
    Sum,        // + -
    Product,    // * / %
    Group,      // ( )
    Prefix,     // -X !X ~X
}

impl Add<i32> for Precedence {
    type Output = Precedence;

    fn add(self, rhs: i32) -> Self::Output {
        let current_value = self as i32;
        let new_value = current_value + rhs;

        match new_value {
            0 => Precedence::Lowest,
            1 => Precedence::LogicalOr,
            2 => Precedence::LogicalAnd,
            3 => Precedence::Equality,
            4 => Precedence::Comparison,
            5 => Precedence::BitOr,
            6 => Precedence::BitXor,
            7 => Precedence::BitAnd,
            8 => Precedence::Shift,
            9 => Precedence::Sum,
            10 => Precedence::Product,
            11 => Precedence::Group,
            12 => Precedence::Prefix,
            _ => Precedence::Prefix,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub name: String,
    pub location: SourceLocation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub mutable: bool,
    pub ident: Identifier,
    pub typ: Type,
}
