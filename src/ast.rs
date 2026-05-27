#[derive(Debug, Clone)]
pub enum BinOp {
    Add, Sub, Mul, Div, FloorDiv, Mod, Pow,
}

#[derive(Debug, Clone)]
pub enum CmpOp {
    Eq, Ne, Lt, Gt, Le, Ge,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    None,
    Name(String),
    BinOp(Box<Expr>, BinOp, Box<Expr>),
    UnaryNeg(Box<Expr>),
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Compare(Box<Expr>, CmpOp, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    List(Vec<Expr>),
    Subscript(Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Assign(String, Expr),
    AugAssign(String, BinOp, Expr),
    SubAssign(Box<Expr>, Box<Expr>, Expr), // obj[idx] = val
    Expr(Expr),
    If(Expr, Vec<Stmt>, Vec<Stmt>),
    While(Expr, Vec<Stmt>),
    For(String, Expr, Vec<Stmt>),
    FuncDef(String, Vec<String>, Vec<Stmt>),
    Return(Option<Expr>),
    Pass,
    Break,
    Continue,
}
