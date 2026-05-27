use std::collections::HashSet;
use std::rc::Rc;

use crate::ast::*;
use crate::bytecode::{CodeObject, Const, Op};
use crate::error::Result;

struct LoopCtx {
    start: usize,        // instruction to jump to for `continue`
    breaks: Vec<usize>,  // positions of Break jumps to patch
}

struct Compiler {
    code: CodeObject,
    _locals: Option<HashSet<String>>, // None = module scope
    loop_stack: Vec<LoopCtx>,
}

impl Compiler {
    fn new(name: impl Into<String>, args: Vec<String>, locals: Option<HashSet<String>>) -> Self {
        Compiler {
            code: CodeObject::new(name, args),
            _locals: locals,
            loop_stack: Vec::new(),
        }
    }

    fn compile_stmts(&mut self, stmts: &[Stmt]) -> Result<()> {
        for s in stmts { self.compile_stmt(s)?; }
        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match stmt {
            Stmt::Assign(name, expr) => {
                self.compile_expr(expr)?;
                let idx = self.code.add_name(name);
                self.code.emit(Op::StoreName(idx));
            }
            Stmt::AugAssign(name, op, rhs) => {
                let idx = self.code.add_name(name);
                self.code.emit(Op::LoadName(idx));
                self.compile_expr(rhs)?;
                self.code.emit(match op {
                    BinOp::Add => Op::BinaryAdd,
                    BinOp::Sub => Op::BinarySub,
                    BinOp::Mul => Op::BinaryMul,
                    BinOp::Div => Op::BinaryDiv,
                    BinOp::FloorDiv => Op::BinaryFloorDiv,
                    BinOp::Mod => Op::BinaryMod,
                    BinOp::Pow => Op::BinaryPow,
                });
                self.code.emit(Op::StoreName(idx));
            }
            Stmt::SubAssign(obj, idx_expr, val) => {
                self.compile_expr(val)?;
                self.compile_expr(obj)?;
                self.compile_expr(idx_expr)?;
                self.code.emit(Op::StoreSubscr);
            }
            Stmt::Expr(expr) => {
                self.compile_expr(expr)?;
                self.code.emit(Op::Pop);
            }
            Stmt::If(cond, body, orelse) => {
                self.compile_expr(cond)?;
                let jf = self.code.emit(Op::JumpIfFalse(0));
                self.compile_stmts(body)?;
                if orelse.is_empty() {
                    let target = self.code.next_ip();
                    self.code.patch(jf, target);
                } else {
                    let jmp = self.code.emit(Op::Jump(0));
                    let else_start = self.code.next_ip();
                    self.code.patch(jf, else_start);
                    self.compile_stmts(orelse)?;
                    let end = self.code.next_ip();
                    self.code.patch(jmp, end);
                }
            }
            Stmt::While(cond, body) => {
                let loop_start = self.code.next_ip();
                self.compile_expr(cond)?;
                let jf = self.code.emit(Op::JumpIfFalse(0));

                self.loop_stack.push(LoopCtx { start: loop_start, breaks: Vec::new() });
                self.compile_stmts(body)?;
                let ctx = self.loop_stack.pop().unwrap();

                self.code.emit(Op::Jump(loop_start));
                let end = self.code.next_ip();
                self.code.patch(jf, end);
                for b in ctx.breaks { self.code.patch(b, end); }
            }
            Stmt::For(target, iter_expr, body) => {
                self.compile_expr(iter_expr)?;
                self.code.emit(Op::GetIter);
                let loop_start = self.code.next_ip();
                let for_iter_pos = self.code.emit(Op::ForIter(0));

                let target_idx = self.code.add_name(target);
                self.code.emit(Op::StoreName(target_idx));

                self.loop_stack.push(LoopCtx { start: loop_start, breaks: Vec::new() });
                self.compile_stmts(body)?;
                let ctx = self.loop_stack.pop().unwrap();

                self.code.emit(Op::Jump(loop_start));
                let end = self.code.next_ip();
                self.code.patch(for_iter_pos, end);
                for b in ctx.breaks { self.code.patch(b, end); }
            }
            Stmt::FuncDef(name, args, body) => {
                // Collect locals for the function body
                let fn_locals = collect_locals(body);
                let mut fn_compiler = Compiler::new(name.clone(), args.clone(), Some(fn_locals));
                fn_compiler.compile_stmts(body)?;
                // Ensure function always has a return
                let ci = fn_compiler.code.add_const(Const::None);
                fn_compiler.code.emit(Op::LoadConst(ci));
                fn_compiler.code.emit(Op::ReturnValue);

                let co = Rc::new(fn_compiler.code);
                let ci = self.code.add_const(Const::Code(co));
                self.code.emit(Op::LoadConst(ci));
                self.code.emit(Op::MakeFunction);
                let ni = self.code.add_name(name);
                self.code.emit(Op::StoreName(ni));
            }
            Stmt::Return(val) => {
                match val {
                    Some(e) => self.compile_expr(e)?,
                    None => {
                        let ci = self.code.add_const(Const::None);
                        self.code.emit(Op::LoadConst(ci));
                    }
                }
                self.code.emit(Op::ReturnValue);
            }
            Stmt::Pass => {}
            Stmt::Break => {
                let pos = self.code.emit(Op::Jump(0));
                if let Some(ctx) = self.loop_stack.last_mut() {
                    ctx.breaks.push(pos);
                }
            }
            Stmt::Continue => {
                if let Some(ctx) = self.loop_stack.last() {
                    let target = ctx.start;
                    self.code.emit(Op::Jump(target));
                }
            }
        }
        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<()> {
        match expr {
            Expr::Int(n) => {
                let i = self.code.add_const(Const::Int(*n));
                self.code.emit(Op::LoadConst(i));
            }
            Expr::Float(f) => {
                let i = self.code.add_const(Const::Float(*f));
                self.code.emit(Op::LoadConst(i));
            }
            Expr::Str(s) => {
                let i = self.code.add_const(Const::Str(s.clone()));
                self.code.emit(Op::LoadConst(i));
            }
            Expr::Bool(b) => {
                let i = self.code.add_const(Const::Bool(*b));
                self.code.emit(Op::LoadConst(i));
            }
            Expr::None => {
                let i = self.code.add_const(Const::None);
                self.code.emit(Op::LoadConst(i));
            }
            Expr::Name(n) => {
                let i = self.code.add_name(n);
                self.code.emit(Op::LoadName(i));
            }
            Expr::BinOp(l, op, r) => {
                self.compile_expr(l)?;
                self.compile_expr(r)?;
                self.code.emit(match op {
                    BinOp::Add => Op::BinaryAdd,
                    BinOp::Sub => Op::BinarySub,
                    BinOp::Mul => Op::BinaryMul,
                    BinOp::Div => Op::BinaryDiv,
                    BinOp::FloorDiv => Op::BinaryFloorDiv,
                    BinOp::Mod => Op::BinaryMod,
                    BinOp::Pow => Op::BinaryPow,
                });
            }
            Expr::UnaryNeg(e) => {
                self.compile_expr(e)?;
                self.code.emit(Op::UnaryNeg);
            }
            Expr::Not(e) => {
                self.compile_expr(e)?;
                self.code.emit(Op::UnaryNot);
            }
            Expr::And(l, r) => {
                self.compile_expr(l)?;
                let jmp = self.code.emit(Op::JumpIfFalseOrPop(0));
                self.compile_expr(r)?;
                let end = self.code.next_ip();
                self.code.patch(jmp, end);
            }
            Expr::Or(l, r) => {
                self.compile_expr(l)?;
                let jmp = self.code.emit(Op::JumpIfTrueOrPop(0));
                self.compile_expr(r)?;
                let end = self.code.next_ip();
                self.code.patch(jmp, end);
            }
            Expr::Compare(l, op, r) => {
                self.compile_expr(l)?;
                self.compile_expr(r)?;
                self.code.emit(match op {
                    CmpOp::Eq => Op::CmpEq,
                    CmpOp::Ne => Op::CmpNe,
                    CmpOp::Lt => Op::CmpLt,
                    CmpOp::Gt => Op::CmpGt,
                    CmpOp::Le => Op::CmpLe,
                    CmpOp::Ge => Op::CmpGe,
                });
            }
            Expr::Call(func, args) => {
                self.compile_expr(func)?;
                let argc = args.len();
                for a in args { self.compile_expr(a)?; }
                self.code.emit(Op::CallFunction(argc));
            }
            Expr::List(items) => {
                let n = items.len();
                for item in items { self.compile_expr(item)?; }
                self.code.emit(Op::BuildList(n));
            }
            Expr::Subscript(obj, idx) => {
                self.compile_expr(obj)?;
                self.compile_expr(idx)?;
                self.code.emit(Op::BinarySubscr);
            }
        }
        Ok(())
    }
}

fn collect_locals(stmts: &[Stmt]) -> HashSet<String> {
    let mut set = HashSet::new();
    for s in stmts {
        match s {
            Stmt::Assign(n, _) | Stmt::AugAssign(n, _, _) => { set.insert(n.clone()); }
            Stmt::For(n, _, body) => {
                set.insert(n.clone());
                set.extend(collect_locals(body));
            }
            Stmt::If(_, a, b) => {
                set.extend(collect_locals(a));
                set.extend(collect_locals(b));
            }
            Stmt::While(_, body) => { set.extend(collect_locals(body)); }
            // Don't recurse into nested FuncDef — those get their own scope
            _ => {}
        }
    }
    set
}

pub fn compile_module(stmts: &[Stmt]) -> Result<CodeObject> {
    let mut c = Compiler::new("<module>", vec![], None);
    c.compile_stmts(stmts)?;
    c.code.emit(Op::Halt);
    Ok(c.code)
}
