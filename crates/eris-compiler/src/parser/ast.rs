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
        // params: Vec<FuncParam>,
        parameters: Vec<(Identifier, Type)>,
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
    // And,
    // Or,
    // Xor,
    LAnd,
    LOr,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    // Shl,
    // Shr,
    // Comma,
    // Assign,
}

#[derive(Ord, Eq, PartialEq, PartialOrd, Debug)]
pub enum Precedence {
    Lowest = 0,
    LogicalOr,  // ||
    LogicalAnd, // &&
    Equality,   // == or !=
    Comparison, // <, <=, >, >=
    Sum,        // + or -
    Product,    // * or /
    Group,      // ( )
    Prefix,     // -X or !X
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
            5 => Precedence::Sum,
            6 => Precedence::Product,
            7 => Precedence::Group,
            8 => Precedence::Prefix,
            _ => Precedence::Prefix,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub name: String,
    pub location: SourceLocation,
}
