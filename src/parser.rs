use crate::ast::*;
use crate::error::{PyError, Result};
use crate::lexer::Tok;

pub struct Parser {
    tokens: Vec<(Tok, usize)>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<(Tok, usize)>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn line(&self) -> usize {
        self.tokens.get(self.pos).map(|(_, l)| *l).unwrap_or(0)
    }

    fn peek(&self) -> &Tok {
        self.tokens.get(self.pos).map(|(t, _)| t).unwrap_or(&Tok::Eof)
    }

    #[allow(dead_code)]
    fn peek2(&self) -> &Tok {
        self.tokens.get(self.pos + 1).map(|(t, _)| t).unwrap_or(&Tok::Eof)
    }

    fn advance(&mut self) -> &Tok {
        let t = self.tokens.get(self.pos).map(|(t, _)| t).unwrap_or(&Tok::Eof);
        if self.pos < self.tokens.len() { self.pos += 1; }
        t
    }

    fn expect(&mut self, expected: &Tok) -> Result<()> {
        let line = self.line();
        let got = self.advance();
        if std::mem::discriminant(got) == std::mem::discriminant(expected) {
            Ok(())
        } else {
            Err(PyError::syntax(format!("expected {:?}, got {:?}", expected, got), line))
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Tok::Newline) { self.pos += 1; }
    }

    fn eat_newline(&mut self) {
        if matches!(self.peek(), Tok::Newline) { self.pos += 1; }
    }

    // ── Public entry point ─────────────────────────────────────────────────

    pub fn parse_module(&mut self) -> Result<Vec<Stmt>> {
        let mut stmts = Vec::new();
        self.skip_newlines();
        while !matches!(self.peek(), Tok::Eof) {
            stmts.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        Ok(stmts)
    }

    // ── Statements ─────────────────────────────────────────────────────────

    fn parse_stmt(&mut self) -> Result<Stmt> {
        self.skip_newlines();
        let line = self.line();
        match self.peek() {
            Tok::If => self.parse_if(),
            Tok::While => self.parse_while(),
            Tok::For => self.parse_for(),
            Tok::Def => self.parse_def(),
            Tok::Return => {
                self.advance();
                let val = if matches!(self.peek(), Tok::Newline | Tok::Eof) {
                    None
                } else {
                    Some(self.parse_or()?)
                };
                self.eat_newline();
                Ok(Stmt::Return(val))
            }
            Tok::Pass => { self.advance(); self.eat_newline(); Ok(Stmt::Pass) }
            Tok::Break => { self.advance(); self.eat_newline(); Ok(Stmt::Break) }
            Tok::Continue => { self.advance(); self.eat_newline(); Ok(Stmt::Continue) }
            _ => self.parse_expr_stmt(line),
        }
    }

    fn parse_if(&mut self) -> Result<Stmt> {
        self.advance(); // 'if'
        let cond = self.parse_or()?;
        let body = self.parse_block()?;
        self.skip_newlines();

        let orelse = if matches!(self.peek(), Tok::Elif) {
            vec![self.parse_if_from_elif()?]
        } else if matches!(self.peek(), Tok::Else) {
            self.advance();
            self.parse_block()?
        } else {
            Vec::new()
        };

        Ok(Stmt::If(cond, body, orelse))
    }

    fn parse_if_from_elif(&mut self) -> Result<Stmt> {
        self.advance(); // 'elif'
        let cond = self.parse_or()?;
        let body = self.parse_block()?;
        self.skip_newlines();

        let orelse = if matches!(self.peek(), Tok::Elif) {
            vec![self.parse_if_from_elif()?]
        } else if matches!(self.peek(), Tok::Else) {
            self.advance();
            self.parse_block()?
        } else {
            Vec::new()
        };

        Ok(Stmt::If(cond, body, orelse))
    }

    fn parse_while(&mut self) -> Result<Stmt> {
        self.advance(); // 'while'
        let cond = self.parse_or()?;
        let body = self.parse_block()?;
        Ok(Stmt::While(cond, body))
    }

    fn parse_for(&mut self) -> Result<Stmt> {
        let line = self.line();
        self.advance(); // 'for'
        let name = match self.advance().clone() {
            Tok::Name(n) => n,
            t => return Err(PyError::syntax(format!("expected variable name, got {:?}", t), line)),
        };
        self.expect(&Tok::In)?;
        let iter = self.parse_or()?;
        let body = self.parse_block()?;
        Ok(Stmt::For(name, iter, body))
    }

    fn parse_def(&mut self) -> Result<Stmt> {
        let line = self.line();
        self.advance(); // 'def'
        let name = match self.advance().clone() {
            Tok::Name(n) => n,
            t => return Err(PyError::syntax(format!("expected function name, got {:?}", t), line)),
        };
        self.expect(&Tok::LParen)?;
        let mut args = Vec::new();
        while !matches!(self.peek(), Tok::RParen | Tok::Eof) {
            match self.advance().clone() {
                Tok::Name(n) => args.push(n),
                t => return Err(PyError::syntax(format!("expected arg name, got {:?}", t), line)),
            }
            if matches!(self.peek(), Tok::Comma) { self.advance(); }
        }
        self.expect(&Tok::RParen)?;
        let body = self.parse_block()?;
        Ok(Stmt::FuncDef(name, args, body))
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>> {
        self.expect(&Tok::Colon)?;
        self.skip_newlines();
        self.expect(&Tok::Indent)?;
        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Tok::Dedent | Tok::Eof) { break; }
            stmts.push(self.parse_stmt()?);
        }
        if matches!(self.peek(), Tok::Dedent) { self.advance(); }
        Ok(stmts)
    }

    fn parse_expr_stmt(&mut self, line: usize) -> Result<Stmt> {
        let expr = self.parse_or()?;

        match self.peek() {
            Tok::Assign => {
                self.advance();
                let val = self.parse_or()?;
                self.eat_newline();
                match expr {
                    Expr::Name(n) => Ok(Stmt::Assign(n, val)),
                    Expr::Subscript(obj, idx) => Ok(Stmt::SubAssign(obj, idx, val)),
                    _ => Err(PyError::syntax("invalid assignment target", line)),
                }
            }
            Tok::PlusEq => {
                self.advance();
                let rhs = self.parse_or()?;
                self.eat_newline();
                match expr {
                    Expr::Name(n) => Ok(Stmt::AugAssign(n, BinOp::Add, rhs)),
                    _ => Err(PyError::syntax("invalid augmented assignment", line)),
                }
            }
            Tok::MinusEq => {
                self.advance();
                let rhs = self.parse_or()?;
                self.eat_newline();
                match expr {
                    Expr::Name(n) => Ok(Stmt::AugAssign(n, BinOp::Sub, rhs)),
                    _ => Err(PyError::syntax("invalid augmented assignment", line)),
                }
            }
            Tok::StarEq => {
                self.advance();
                let rhs = self.parse_or()?;
                self.eat_newline();
                match expr {
                    Expr::Name(n) => Ok(Stmt::AugAssign(n, BinOp::Mul, rhs)),
                    _ => Err(PyError::syntax("invalid augmented assignment", line)),
                }
            }
            Tok::SlashEq => {
                self.advance();
                let rhs = self.parse_or()?;
                self.eat_newline();
                match expr {
                    Expr::Name(n) => Ok(Stmt::AugAssign(n, BinOp::Div, rhs)),
                    _ => Err(PyError::syntax("invalid augmented assignment", line)),
                }
            }
            _ => {
                self.eat_newline();
                Ok(Stmt::Expr(expr))
            }
        }
    }

    // ── Expressions (precedence climbing) ──────────────────────────────────

    // or
    fn parse_or(&mut self) -> Result<Expr> {
        let mut left = self.parse_and()?;
        while matches!(self.peek(), Tok::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    // and
    fn parse_and(&mut self) -> Result<Expr> {
        let mut left = self.parse_not()?;
        while matches!(self.peek(), Tok::And) {
            self.advance();
            let right = self.parse_not()?;
            left = Expr::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    // not
    fn parse_not(&mut self) -> Result<Expr> {
        if matches!(self.peek(), Tok::Not) {
            self.advance();
            Ok(Expr::Not(Box::new(self.parse_not()?)))
        } else {
            self.parse_compare()
        }
    }

    // comparisons
    fn parse_compare(&mut self) -> Result<Expr> {
        let left = self.parse_add()?;
        let op = match self.peek() {
            Tok::Eq => CmpOp::Eq,
            Tok::Ne => CmpOp::Ne,
            Tok::Lt => CmpOp::Lt,
            Tok::Gt => CmpOp::Gt,
            Tok::Le => CmpOp::Le,
            Tok::Ge => CmpOp::Ge,
            _ => return Ok(left),
        };
        self.advance();
        let right = self.parse_add()?;
        Ok(Expr::Compare(Box::new(left), op, Box::new(right)))
    }

    // + -
    fn parse_add(&mut self) -> Result<Expr> {
        let mut left = self.parse_mul()?;
        loop {
            let op = match self.peek() {
                Tok::Plus => BinOp::Add,
                Tok::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_mul()?;
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    // * / // %
    fn parse_mul(&mut self) -> Result<Expr> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Tok::Star => BinOp::Mul,
                Tok::Slash => BinOp::Div,
                Tok::DSlash => BinOp::FloorDiv,
                Tok::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_unary()?;
            left = Expr::BinOp(Box::new(left), op, Box::new(right));
        }
        Ok(left)
    }

    // unary -
    fn parse_unary(&mut self) -> Result<Expr> {
        if matches!(self.peek(), Tok::Minus) {
            self.advance();
            Ok(Expr::UnaryNeg(Box::new(self.parse_unary()?)))
        } else if matches!(self.peek(), Tok::Plus) {
            self.advance();
            self.parse_unary()
        } else {
            self.parse_power()
        }
    }

    // **  (right-assoc)
    fn parse_power(&mut self) -> Result<Expr> {
        let base = self.parse_postfix()?;
        if matches!(self.peek(), Tok::DStar) {
            self.advance();
            let exp = self.parse_unary()?; // right-assoc
            Ok(Expr::BinOp(Box::new(base), BinOp::Pow, Box::new(exp)))
        } else {
            Ok(base)
        }
    }

    // call / subscript / attribute
    fn parse_postfix(&mut self) -> Result<Expr> {
        let mut expr = self.parse_atom()?;
        loop {
            match self.peek() {
                Tok::LParen => {
                    self.advance();
                    let mut args = Vec::new();
                    while !matches!(self.peek(), Tok::RParen | Tok::Eof) {
                        args.push(self.parse_or()?);
                        if matches!(self.peek(), Tok::Comma) { self.advance(); }
                    }
                    self.expect(&Tok::RParen)?;
                    expr = Expr::Call(Box::new(expr), args);
                }
                Tok::LBracket => {
                    self.advance();
                    let idx = self.parse_or()?;
                    self.expect(&Tok::RBracket)?;
                    expr = Expr::Subscript(Box::new(expr), Box::new(idx));
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    // atoms: literals, names, parens, list
    fn parse_atom(&mut self) -> Result<Expr> {
        let line = self.line();
        match self.peek().clone() {
            Tok::Int(n) => { self.advance(); Ok(Expr::Int(n)) }
            Tok::Float(f) => { self.advance(); Ok(Expr::Float(f)) }
            Tok::Str(s) => { self.advance(); Ok(Expr::Str(s)) }
            Tok::True => { self.advance(); Ok(Expr::Bool(true)) }
            Tok::False => { self.advance(); Ok(Expr::Bool(false)) }
            Tok::None_ => { self.advance(); Ok(Expr::None) }
            Tok::Name(n) => { self.advance(); Ok(Expr::Name(n)) }
            Tok::LParen => {
                self.advance();
                let e = self.parse_or()?;
                self.expect(&Tok::RParen)?;
                Ok(e)
            }
            Tok::LBracket => {
                self.advance();
                let mut items = Vec::new();
                while !matches!(self.peek(), Tok::RBracket | Tok::Eof) {
                    items.push(self.parse_or()?);
                    if matches!(self.peek(), Tok::Comma) { self.advance(); }
                }
                self.expect(&Tok::RBracket)?;
                Ok(Expr::List(items))
            }
            t => Err(PyError::syntax(format!("unexpected token {:?}", t), line)),
        }
    }
}

pub fn parse(tokens: Vec<(Tok, usize)>) -> Result<Vec<Stmt>> {
    Parser::new(tokens).parse_module()
}
