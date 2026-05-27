use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::bytecode::{CodeObject, Const, Op};
use crate::error::{PyError, Result};

// ── Value ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    None,
    List(Rc<RefCell<Vec<Value>>>),
    Iter(Rc<RefCell<IterState>>),
    Function { name: String, code: Rc<CodeObject> },
    Builtin(&'static str),
}

#[derive(Debug, Clone)]
pub struct IterState {
    pub items: Vec<Value>,
    pub pos: usize,
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Int(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::None => false,
            Value::List(v) => !v.borrow().is_empty(),
            _ => true,
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "str",
            Value::Bool(_) => "bool",
            Value::None => "NoneType",
            Value::List(_) => "list",
            Value::Iter(_) => "iterator",
            Value::Function { .. } => "function",
            Value::Builtin(_) => "builtin_function",
        }
    }

    fn display(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Float(f) => {
                let s = format!("{}", f);
                if s.contains('.') { s } else { format!("{}.0", s) }
            }
            Value::Str(s) => s.clone(),
            Value::Bool(b) => if *b { "True".into() } else { "False".into() },
            Value::None => "None".into(),
            Value::List(v) => {
                let items: Vec<String> = v.borrow().iter().map(Value::repr).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Iter(_) => "<iterator>".into(),
            Value::Function { name, .. } => format!("<function {}>", name),
            Value::Builtin(n) => format!("<built-in function {}>", n),
        }
    }

    fn repr(&self) -> String {
        match self {
            Value::Str(s) => format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'")),
            _ => self.display(),
        }
    }
}

fn const_to_value(c: &Const) -> Value {
    match c {
        Const::Int(n) => Value::Int(*n),
        Const::Float(f) => Value::Float(*f),
        Const::Str(s) => Value::Str(s.clone()),
        Const::Bool(b) => Value::Bool(*b),
        Const::None => Value::None,
        Const::Code(co) => Value::Function { name: co.name.clone(), code: co.clone() },
    }
}

// ── Frame ──────────────────────────────────────────────────────────────────

struct Frame {
    code: Rc<CodeObject>,
    locals: HashMap<String, Value>,
    stack: Vec<Value>,
    ip: usize,
}

impl Frame {
    fn new(code: Rc<CodeObject>, locals: HashMap<String, Value>) -> Self {
        Frame { code, locals, stack: Vec::new(), ip: 0 }
    }

    fn push(&mut self, v: Value) { self.stack.push(v); }

    fn pop(&mut self) -> Result<Value> {
        self.stack.pop().ok_or_else(|| PyError::runtime("stack underflow"))
    }

    fn peek(&self) -> Result<&Value> {
        self.stack.last().ok_or_else(|| PyError::runtime("stack empty"))
    }
}

// ── VM ─────────────────────────────────────────────────────────────────────

pub struct VM {
    globals: HashMap<String, Value>,
}

impl VM {
    pub fn new() -> Self {
        let mut globals = HashMap::new();
        for name in &["print", "input", "range", "len", "str", "int", "float", "bool", "abs", "max", "min", "type"] {
            globals.insert(name.to_string(), Value::Builtin(name));
        }
        VM { globals }
    }

    pub fn run(&mut self, code: Rc<CodeObject>) -> Result<()> {
        self.exec_frame(code, HashMap::new())?;
        Ok(())
    }

    fn exec_frame(&mut self, code: Rc<CodeObject>, locals: HashMap<String, Value>) -> Result<Value> {
        let mut frame = Frame::new(code, locals);

        loop {
            if frame.ip >= frame.code.code.len() {
                return Ok(Value::None);
            }
            let op = frame.code.code[frame.ip].clone();
            frame.ip += 1;

            match op {
                Op::Halt => return Ok(Value::None),

                Op::LoadConst(i) => {
                    let v = const_to_value(&frame.code.consts[i]);
                    frame.push(v);
                }

                Op::LoadName(i) => {
                    let name = &frame.code.names[i];
                    let v = frame.locals.get(name)
                        .or_else(|| self.globals.get(name))
                        .cloned()
                        .ok_or_else(|| PyError::runtime(format!("name '{}' is not defined", name)))?;
                    frame.push(v);
                }

                Op::StoreName(i) => {
                    let v = frame.pop()?;
                    let name = frame.code.names[i].clone();
                    if frame.locals.contains_key(&name) || frame.code.args.contains(&name) {
                        frame.locals.insert(name, v);
                    } else {
                        // module scope or global
                        self.globals.insert(name, v);
                    }
                }

                Op::BinaryAdd => {
                    let b = frame.pop()?;
                    let a = frame.pop()?;
                    frame.push(self.add(a, b)?);
                }
                Op::BinarySub => {
                    let b = frame.pop()?;
                    let a = frame.pop()?;
                    frame.push(self.arith(a, b, '-')?);
                }
                Op::BinaryMul => {
                    let b = frame.pop()?;
                    let a = frame.pop()?;
                    frame.push(self.arith(a, b, '*')?);
                }
                Op::BinaryDiv => {
                    let b = frame.pop()?;
                    let a = frame.pop()?;
                    frame.push(self.arith(a, b, '/')?);
                }
                Op::BinaryFloorDiv => {
                    let b = frame.pop()?;
                    let a = frame.pop()?;
                    frame.push(self.arith(a, b, 'd')?);
                }
                Op::BinaryMod => {
                    let b = frame.pop()?;
                    let a = frame.pop()?;
                    frame.push(self.arith(a, b, '%')?);
                }
                Op::BinaryPow => {
                    let b = frame.pop()?;
                    let a = frame.pop()?;
                    frame.push(self.arith(a, b, '^')?);
                }
                Op::UnaryNeg => {
                    let v = frame.pop()?;
                    frame.push(match v {
                        Value::Int(n) => Value::Int(-n),
                        Value::Float(f) => Value::Float(-f),
                        v => return Err(PyError::runtime(format!("bad operand for unary -: '{}'", v.type_name()))),
                    });
                }

                Op::CmpEq => { let (a, b) = (frame.pop()?, frame.pop()?); frame.push(Value::Bool(self.equal(&b, &a))); }
                Op::CmpNe => { let (a, b) = (frame.pop()?, frame.pop()?); frame.push(Value::Bool(!self.equal(&b, &a))); }
                Op::CmpLt => { let b = frame.pop()?; let a = frame.pop()?; frame.push(Value::Bool(self.cmp_lt(&a, &b)?)); }
                Op::CmpGt => { let b = frame.pop()?; let a = frame.pop()?; frame.push(Value::Bool(self.cmp_lt(&b, &a)?)); }
                Op::CmpLe => { let b = frame.pop()?; let a = frame.pop()?; frame.push(Value::Bool(!self.cmp_lt(&b, &a)?)); }
                Op::CmpGe => { let b = frame.pop()?; let a = frame.pop()?; frame.push(Value::Bool(!self.cmp_lt(&a, &b)?)); }

                Op::UnaryNot => {
                    let v = frame.pop()?;
                    frame.push(Value::Bool(!v.is_truthy()));
                }

                Op::JumpIfFalseOrPop(target) => {
                    if !frame.peek()?.is_truthy() {
                        frame.ip = target;
                    } else {
                        frame.pop()?;
                    }
                }
                Op::JumpIfTrueOrPop(target) => {
                    if frame.peek()?.is_truthy() {
                        frame.ip = target;
                    } else {
                        frame.pop()?;
                    }
                }
                Op::Jump(target) => { frame.ip = target; }
                Op::JumpIfFalse(target) => {
                    let v = frame.pop()?;
                    if !v.is_truthy() { frame.ip = target; }
                }
                Op::JumpIfTrue(target) => {
                    let v = frame.pop()?;
                    if v.is_truthy() { frame.ip = target; }
                }

                Op::MakeFunction => {
                    // TOS is already a Function value from LoadConst(Code(...))
                    // nothing to do — MakeFunction is a no-op here
                    // (could attach defaults in future)
                }

                Op::CallFunction(argc) => {
                    let mut args: Vec<Value> = (0..argc).map(|_| frame.pop()).collect::<Result<Vec<_>>>()?;
                    args.reverse();
                    let func = frame.pop()?;
                    let result = self.call(func, args, &frame)?;
                    frame.push(result);
                }

                Op::ReturnValue => {
                    return frame.pop();
                }

                Op::GetIter => {
                    let v = frame.pop()?;
                    let items = match v {
                        Value::List(l) => l.borrow().clone(),
                        Value::Iter(it) => it.borrow().items[it.borrow().pos..].to_vec(),
                        v => return Err(PyError::runtime(format!("'{}' is not iterable", v.type_name()))),
                    };
                    frame.push(Value::Iter(Rc::new(RefCell::new(IterState { items, pos: 0 }))));
                }

                Op::ForIter(end) => {
                    let iter = match frame.peek()? {
                        Value::Iter(it) => it.clone(),
                        v => return Err(PyError::runtime(format!("expected iterator, got '{}'", v.type_name()))),
                    };
                    let mut state = iter.borrow_mut();
                    if state.pos >= state.items.len() {
                        drop(state);
                        frame.pop()?; // pop the iterator
                        frame.ip = end;
                    } else {
                        let item = state.items[state.pos].clone();
                        state.pos += 1;
                        drop(state);
                        frame.push(item);
                    }
                }

                Op::BuildList(n) => {
                    let mut items: Vec<Value> = (0..n).map(|_| frame.pop()).collect::<Result<Vec<_>>>()?;
                    items.reverse();
                    frame.push(Value::List(Rc::new(RefCell::new(items))));
                }

                Op::BinarySubscr => {
                    let idx = frame.pop()?;
                    let obj = frame.pop()?;
                    frame.push(self.subscript(obj, idx)?);
                }

                Op::StoreSubscr => {
                    let val = frame.pop()?;
                    let obj = frame.pop()?;
                    let idx = frame.pop()?;
                    match (obj, idx) {
                        (Value::List(l), Value::Int(i)) => {
                            let mut v = l.borrow_mut();
                            let len = v.len() as i64;
                            let i = if i < 0 { (len + i) as usize } else { i as usize };
                            if i >= v.len() {
                                return Err(PyError::runtime("list index out of range"));
                            }
                            v[i] = val;
                        }
                        _ => return Err(PyError::runtime("invalid subscript assignment")),
                    }
                }

                Op::Pop => { frame.pop()?; }
                Op::Dup => {
                    let v = frame.peek()?.clone();
                    frame.push(v);
                }
            }
        }
    }

    fn call(&mut self, func: Value, args: Vec<Value>, _frame: &Frame) -> Result<Value> {
        match func {
            Value::Function { name: _, code } => {
                if args.len() != code.args.len() {
                    return Err(PyError::runtime(format!(
                        "{}() takes {} arg(s), got {}",
                        code.name, code.args.len(), args.len()
                    )));
                }
                let mut locals = HashMap::new();
                for (name, val) in code.args.iter().zip(args) {
                    locals.insert(name.clone(), val);
                }
                self.exec_frame(code, locals)
            }
            Value::Builtin(name) => self.call_builtin(name, args),
            v => Err(PyError::runtime(format!("'{}' is not callable", v.type_name()))),
        }
    }

    fn call_builtin(&mut self, name: &str, args: Vec<Value>) -> Result<Value> {
        match name {
            "print" => {
                let parts: Vec<String> = args.iter().map(|v| v.display()).collect();
                println!("{}", parts.join(" "));
                Ok(Value::None)
            }
            "input" => {
                use std::io::{self, Write};
                if let Some(prompt) = args.first() {
                    print!("{}", prompt.display());
                    io::stdout().flush().ok();
                }
                let mut line = String::new();
                io::stdin().read_line(&mut line)
                    .map_err(|e| PyError::runtime(format!("input() failed: {}", e)))?;
                Ok(Value::Str(line.trim_end_matches('\n').trim_end_matches('\r').to_string()))
            }
            "range" => {
                let (start, stop, step) = match args.len() {
                    1 => (0i64, int_val(&args[0])?, 1i64),
                    2 => (int_val(&args[0])?, int_val(&args[1])?, 1i64),
                    3 => (int_val(&args[0])?, int_val(&args[1])?, int_val(&args[2])?),
                    n => return Err(PyError::runtime(format!("range() takes 1-3 args, got {}", n))),
                };
                let mut items = Vec::new();
                let mut i = start;
                if step == 0 { return Err(PyError::runtime("range() step cannot be zero")); }
                while (step > 0 && i < stop) || (step < 0 && i > stop) {
                    items.push(Value::Int(i));
                    i += step;
                }
                Ok(Value::List(Rc::new(RefCell::new(items))))
            }
            "len" => {
                if args.len() != 1 { return Err(PyError::runtime("len() takes exactly 1 arg")); }
                match &args[0] {
                    Value::Str(s) => Ok(Value::Int(s.len() as i64)),
                    Value::List(l) => Ok(Value::Int(l.borrow().len() as i64)),
                    v => Err(PyError::runtime(format!("object of type '{}' has no len()", v.type_name()))),
                }
            }
            "str" => {
                if args.len() != 1 { return Err(PyError::runtime("str() takes 1 arg")); }
                Ok(Value::Str(args[0].display()))
            }
            "int" => {
                if args.len() != 1 { return Err(PyError::runtime("int() takes 1 arg")); }
                match &args[0] {
                    Value::Int(n) => Ok(Value::Int(*n)),
                    Value::Float(f) => Ok(Value::Int(*f as i64)),
                    Value::Bool(b) => Ok(Value::Int(*b as i64)),
                    Value::Str(s) => s.trim().parse::<i64>()
                        .map(Value::Int)
                        .map_err(|_| PyError::runtime(format!("invalid literal for int(): '{}'", s))),
                    v => Err(PyError::runtime(format!("int() can't convert '{}'", v.type_name()))),
                }
            }
            "float" => {
                if args.len() != 1 { return Err(PyError::runtime("float() takes 1 arg")); }
                match &args[0] {
                    Value::Float(f) => Ok(Value::Float(*f)),
                    Value::Int(n) => Ok(Value::Float(*n as f64)),
                    Value::Bool(b) => Ok(Value::Float(*b as i64 as f64)),
                    Value::Str(s) => s.trim().parse::<f64>()
                        .map(Value::Float)
                        .map_err(|_| PyError::runtime(format!("invalid literal for float(): '{}'", s))),
                    v => Err(PyError::runtime(format!("float() can't convert '{}'", v.type_name()))),
                }
            }
            "bool" => {
                if args.len() != 1 { return Err(PyError::runtime("bool() takes 1 arg")); }
                Ok(Value::Bool(args[0].is_truthy()))
            }
            "abs" => {
                if args.len() != 1 { return Err(PyError::runtime("abs() takes 1 arg")); }
                match &args[0] {
                    Value::Int(n) => Ok(Value::Int(n.abs())),
                    Value::Float(f) => Ok(Value::Float(f.abs())),
                    v => Err(PyError::runtime(format!("abs() not supported for '{}'", v.type_name()))),
                }
            }
            "max" => {
                if args.is_empty() { return Err(PyError::runtime("max() requires at least one arg")); }
                let items = if args.len() == 1 {
                    match &args[0] {
                        Value::List(l) => l.borrow().clone(),
                        v => return Err(PyError::runtime(format!("'{}' is not iterable", v.type_name()))),
                    }
                } else { args };
                items.into_iter().try_fold(None::<Value>, |acc, v| {
                    Ok(Some(match acc {
                        None => v.clone(),
                        Some(a) => if self.cmp_lt(&a, &v)? { v } else { a },
                    }))
                })?.ok_or_else(|| PyError::runtime("max() arg is empty sequence"))
            }
            "min" => {
                if args.is_empty() { return Err(PyError::runtime("min() requires at least one arg")); }
                let items = if args.len() == 1 {
                    match &args[0] {
                        Value::List(l) => l.borrow().clone(),
                        v => return Err(PyError::runtime(format!("'{}' is not iterable", v.type_name()))),
                    }
                } else { args };
                items.into_iter().try_fold(None::<Value>, |acc, v| {
                    Ok(Some(match acc {
                        None => v.clone(),
                        Some(a) => if self.cmp_lt(&v, &a)? { v } else { a },
                    }))
                })?.ok_or_else(|| PyError::runtime("min() arg is empty sequence"))
            }
            "type" => {
                if args.len() != 1 { return Err(PyError::runtime("type() takes 1 arg")); }
                Ok(Value::Str(format!("<class '{}'>", args[0].type_name())))
            }
            n => Err(PyError::runtime(format!("unknown builtin '{}'", n))),
        }
    }

    fn add(&self, a: Value, b: Value) -> Result<Value> {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x + y)),
            (Value::Float(x), Value::Float(y)) => Ok(Value::Float(x + y)),
            (Value::Int(x), Value::Float(y)) => Ok(Value::Float(x as f64 + y)),
            (Value::Float(x), Value::Int(y)) => Ok(Value::Float(x + y as f64)),
            (Value::Str(x), Value::Str(y)) => Ok(Value::Str(x + &y)),
            (a, b) => Err(PyError::runtime(format!(
                "unsupported operand types for +: '{}' and '{}'", a.type_name(), b.type_name()
            ))),
        }
    }

    fn arith(&self, a: Value, b: Value, op: char) -> Result<Value> {
        let (x, y, is_float) = match (&a, &b) {
            (Value::Int(x), Value::Int(y)) => (*x as f64, *y as f64, false),
            (Value::Float(x), Value::Float(y)) => (*x, *y, true),
            (Value::Int(x), Value::Float(y)) => (*x as f64, *y, true),
            (Value::Float(x), Value::Int(y)) => (*x, *y as f64, true),
            _ => return Err(PyError::runtime(format!(
                "unsupported operand types for '{}' and '{}'", a.type_name(), b.type_name()
            ))),
        };

        if (op == '/' || op == 'd' || op == '%') && y == 0.0 {
            return Err(PyError::runtime("division by zero"));
        }

        let result = match op {
            '-' => x - y,
            '*' => x * y,
            '/' => x / y,
            'd' => (x / y).floor(),
            '%' => x - (x / y).floor() * y,
            '^' => x.powf(y),
            _ => unreachable!(),
        };

        if !is_float && op != '/' {
            Ok(Value::Int(result as i64))
        } else {
            Ok(Value::Float(result))
        }
    }

    fn equal(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => x == y,
            (Value::Float(x), Value::Float(y)) => x == y,
            (Value::Int(x), Value::Float(y)) => *x as f64 == *y,
            (Value::Float(x), Value::Int(y)) => *x == *y as f64,
            (Value::Str(x), Value::Str(y)) => x == y,
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::None, Value::None) => true,
            _ => false,
        }
    }

    fn cmp_lt(&self, a: &Value, b: &Value) -> Result<bool> {
        match (a, b) {
            (Value::Int(x), Value::Int(y)) => Ok(x < y),
            (Value::Float(x), Value::Float(y)) => Ok(x < y),
            (Value::Int(x), Value::Float(y)) => Ok((*x as f64) < *y),
            (Value::Float(x), Value::Int(y)) => Ok(*x < (*y as f64)),
            (Value::Str(x), Value::Str(y)) => Ok(x < y),
            (a, b) => Err(PyError::runtime(format!(
                "'<' not supported between '{}' and '{}'", a.type_name(), b.type_name()
            ))),
        }
    }

    fn subscript(&self, obj: Value, idx: Value) -> Result<Value> {
        match (obj, idx) {
            (Value::List(l), Value::Int(i)) => {
                let v = l.borrow();
                let len = v.len() as i64;
                let i = if i < 0 { len + i } else { i };
                if i < 0 || i >= len {
                    return Err(PyError::runtime("list index out of range"));
                }
                Ok(v[i as usize].clone())
            }
            (Value::Str(s), Value::Int(i)) => {
                let chars: Vec<char> = s.chars().collect();
                let len = chars.len() as i64;
                let i = if i < 0 { len + i } else { i };
                if i < 0 || i >= len {
                    return Err(PyError::runtime("string index out of range"));
                }
                Ok(Value::Str(chars[i as usize].to_string()))
            }
            (obj, idx) => Err(PyError::runtime(format!(
                "'{}' indices must be integers, not '{}'", obj.type_name(), idx.type_name()
            ))),
        }
    }
}

fn int_val(v: &Value) -> Result<i64> {
    match v {
        Value::Int(n) => Ok(*n),
        Value::Bool(b) => Ok(*b as i64),
        v => Err(PyError::runtime(format!("expected int, got '{}'", v.type_name()))),
    }
}
