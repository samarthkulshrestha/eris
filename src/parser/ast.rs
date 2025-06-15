use std::ops::Add;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Stmt>
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
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
    FuncCall {
        ident: String,
        args: Vec<FuncArgs>
    },
    ReturnStmt(Option<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncArgs {
    pub label: Option<String>,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FuncParam {
    ident: String,
    typ: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Char,
    Bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    IntLit(i64),
    FloatLit(f64),
    CharLit(char),
    BoolTrue,
    BoolFalse,
    Identifier(Identifier),

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
    TupleExpr(Vec<Expr>),
    BlockExpr {
        stmts: Vec<Stmt>,
        ret: Option<Box<Expr>>
    },
    IfExpr {
        condition: Box<Expr>,
        // then_branch: Box<Expr>,
        then_branch: Option<Box<Expr>>,
        else_branch: Option<Box<Expr>>,
    },
    FunctionCall {
        function: Box<Expr>,
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
            1 => Precedence::Equality,
            2 => Precedence::Comparison,
            3 => Precedence::Sum,
            4 => Precedence::Product,
            5 => Precedence::Group,
            6 => Precedence::Prefix,
            _ => Precedence::Prefix, // or panic, or return Result
        }
    }
}

// impl Precedence {
//     pub fn higher(self) -> Self {
//         match self {
//             Precedence::Lowest => Precedence::Equality,
//             Precedence::Equality => Precedence::Comparison,
//             Precedence::Comparison => Precedence::Sum,
//             Precedence::Sum => Precedence::Product,
//             Precedence::Product => Precedence::Group,
//             Precedence::Group => Precedence::Prefix,
//             Precedence::Prefix => Precedence::Prefix,
//         }
//     }
//
//     pub fn increment_by(self, levels: i32) -> Self {
//         let current = self as i32;
//         let new_level = (current + levels).clamp(0, 6);
//
//         match new_level {
//             0 => Precedence::Lowest,
//             1 => Precedence::Equality,
//             2 => Precedence::Comparison,
//             3 => Precedence::Sum,
//             4 => Precedence::Product,
//             5 => Precedence::Group,
//             6 => Precedence::Prefix,
//             _ => unreachable!(),
//         }
//     }
// }

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub name: String,
}
