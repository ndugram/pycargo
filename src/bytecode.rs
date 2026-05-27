use std::rc::Rc;

/// Compile-time constant pool values.
#[derive(Debug, Clone)]
pub enum Const {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    None,
    Code(Rc<CodeObject>),
}

/// A compiled function or module.
#[derive(Debug, Clone)]
pub struct CodeObject {
    pub name: String,
    pub args: Vec<String>,
    pub consts: Vec<Const>,  // constant pool
    pub names: Vec<String>,  // variable name pool
    pub code: Vec<Op>,
}

impl CodeObject {
    pub fn new(name: impl Into<String>, args: Vec<String>) -> Self {
        CodeObject {
            name: name.into(),
            args,
            consts: Vec::new(),
            names: Vec::new(),
            code: Vec::new(),
        }
    }

    pub fn add_const(&mut self, c: Const) -> usize {
        // Deduplicate simple constants
        for (i, existing) in self.consts.iter().enumerate() {
            match (existing, &c) {
                (Const::Int(a), Const::Int(b)) if a == b => return i,
                (Const::Float(a), Const::Float(b)) if a.to_bits() == b.to_bits() => return i,
                (Const::Str(a), Const::Str(b)) if a == b => return i,
                (Const::Bool(a), Const::Bool(b)) if a == b => return i,
                (Const::None, Const::None) => return i,
                _ => {}
            }
        }
        let i = self.consts.len();
        self.consts.push(c);
        i
    }

    pub fn add_name(&mut self, name: &str) -> usize {
        if let Some(i) = self.names.iter().position(|n| n == name) {
            return i;
        }
        let i = self.names.len();
        self.names.push(name.to_string());
        i
    }

    pub fn emit(&mut self, op: Op) -> usize {
        let i = self.code.len();
        self.code.push(op);
        i
    }

    /// Patch a jump instruction's target.
    pub fn patch(&mut self, at: usize, target: usize) {
        match &mut self.code[at] {
            Op::Jump(t) | Op::JumpIfFalse(t) | Op::JumpIfTrue(t)
            | Op::JumpIfFalseOrPop(t) | Op::JumpIfTrueOrPop(t)
            | Op::ForIter(t) => *t = target,
            _ => panic!("patch on non-jump at {}", at),
        }
    }

    pub fn next_ip(&self) -> usize {
        self.code.len()
    }
}

#[derive(Debug, Clone)]
pub enum Op {
    // Constants & names
    LoadConst(usize),
    LoadName(usize),
    StoreName(usize),

    // Arithmetic
    BinaryAdd,
    BinarySub,
    BinaryMul,
    BinaryDiv,
    BinaryFloorDiv,
    BinaryMod,
    BinaryPow,
    UnaryNeg,

    // Comparison
    CmpEq, CmpNe, CmpLt, CmpGt, CmpLe, CmpGe,

    // Boolean
    UnaryNot,
    JumpIfFalseOrPop(usize), // 'and': if TOS false → keep + jump, else pop + continue
    JumpIfTrueOrPop(usize),  // 'or':  if TOS true  → keep + jump, else pop + continue

    // Jumps
    Jump(usize),
    JumpIfFalse(usize), // pop, jump if false
    JumpIfTrue(usize),  // pop, jump if true

    // Functions
    MakeFunction,        // pop code object → push Function value
    CallFunction(usize), // pop func + argc args → push result
    ReturnValue,

    // Iteration
    GetIter,             // pop iterable → push iterator
    ForIter(usize),      // advance iter; if done → jump; else push next item

    // Collections
    BuildList(usize),
    BinarySubscr,
    StoreSubscr,

    // Stack
    Pop,
    Dup,

    Halt,
}

// ── Bytecode serialization (for .pyc) ──────────────────────────────────────

pub fn serialize(co: &CodeObject) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"PYCO");
    out.extend_from_slice(&1u32.to_le_bytes());
    write_code(&mut out, co);
    out
}

pub fn deserialize(data: &[u8]) -> Option<CodeObject> {
    if data.len() < 8 { return None; }
    if &data[..4] != b"PYCO" { return None; }
    let _ver = u32::from_le_bytes(data[4..8].try_into().ok()?);
    let mut pos = 8;
    read_code(data, &mut pos)
}

fn write_str(out: &mut Vec<u8>, s: &str) {
    let b = s.as_bytes();
    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
    out.extend_from_slice(b);
}

fn write_u32(out: &mut Vec<u8>, n: usize) {
    out.extend_from_slice(&(n as u32).to_le_bytes());
}

fn write_code(out: &mut Vec<u8>, co: &CodeObject) {
    write_str(out, &co.name);
    write_u32(out, co.args.len());
    for a in &co.args { write_str(out, a); }
    write_u32(out, co.names.len());
    for n in &co.names { write_str(out, n); }
    write_u32(out, co.consts.len());
    for c in &co.consts { write_const(out, c); }
    write_u32(out, co.code.len());
    for op in &co.code { write_op(out, op); }
}

fn write_const(out: &mut Vec<u8>, c: &Const) {
    match c {
        Const::Int(n) => { out.push(0); out.extend_from_slice(&n.to_le_bytes()); }
        Const::Float(f) => { out.push(1); out.extend_from_slice(&f.to_bits().to_le_bytes()); }
        Const::Str(s) => { out.push(2); write_str(out, s); }
        Const::Bool(b) => { out.push(3); out.push(*b as u8); }
        Const::None => { out.push(4); }
        Const::Code(co) => { out.push(5); write_code(out, co); }
    }
}

fn write_op(out: &mut Vec<u8>, op: &Op) {
    macro_rules! op1 { ($tag:expr, $n:expr) => {{ out.push($tag); write_u32(out, *$n); }} }
    match op {
        Op::LoadConst(n) => op1!(0, n),
        Op::LoadName(n) => op1!(1, n),
        Op::StoreName(n) => op1!(2, n),
        Op::BinaryAdd => out.push(10),
        Op::BinarySub => out.push(11),
        Op::BinaryMul => out.push(12),
        Op::BinaryDiv => out.push(13),
        Op::BinaryFloorDiv => out.push(14),
        Op::BinaryMod => out.push(15),
        Op::BinaryPow => out.push(16),
        Op::UnaryNeg => out.push(17),
        Op::CmpEq => out.push(20),
        Op::CmpNe => out.push(21),
        Op::CmpLt => out.push(22),
        Op::CmpGt => out.push(23),
        Op::CmpLe => out.push(24),
        Op::CmpGe => out.push(25),
        Op::UnaryNot => out.push(26),
        Op::JumpIfFalseOrPop(n) => op1!(30, n),
        Op::JumpIfTrueOrPop(n) => op1!(31, n),
        Op::Jump(n) => op1!(32, n),
        Op::JumpIfFalse(n) => op1!(33, n),
        Op::JumpIfTrue(n) => op1!(34, n),
        Op::MakeFunction => out.push(40),
        Op::CallFunction(n) => op1!(41, n),
        Op::ReturnValue => out.push(42),
        Op::GetIter => out.push(50),
        Op::ForIter(n) => op1!(51, n),
        Op::BuildList(n) => op1!(60, n),
        Op::BinarySubscr => out.push(61),
        Op::StoreSubscr => out.push(62),
        Op::Pop => out.push(70),
        Op::Dup => out.push(71),
        Op::Halt => out.push(255),
    }
}

fn read_u32(data: &[u8], pos: &mut usize) -> Option<usize> {
    let v = u32::from_le_bytes(data.get(*pos..*pos + 4)?.try_into().ok()?);
    *pos += 4;
    Some(v as usize)
}

fn read_str(data: &[u8], pos: &mut usize) -> Option<String> {
    let len = read_u32(data, pos)?;
    let s = std::str::from_utf8(data.get(*pos..*pos + len)?).ok()?.to_string();
    *pos += len;
    Some(s)
}

fn read_code(data: &[u8], pos: &mut usize) -> Option<CodeObject> {
    let name = read_str(data, pos)?;
    let argc = read_u32(data, pos)?;
    let mut args = Vec::new();
    for _ in 0..argc { args.push(read_str(data, pos)?); }
    let nc = read_u32(data, pos)?;
    let mut names = Vec::new();
    for _ in 0..nc { names.push(read_str(data, pos)?); }
    let cc = read_u32(data, pos)?;
    let mut consts = Vec::new();
    for _ in 0..cc { consts.push(read_const(data, pos)?); }
    let oc = read_u32(data, pos)?;
    let mut code = Vec::new();
    for _ in 0..oc { code.push(read_op(data, pos)?); }
    Some(CodeObject { name, args, consts, names, code })
}

fn read_const(data: &[u8], pos: &mut usize) -> Option<Const> {
    let tag = *data.get(*pos)?; *pos += 1;
    match tag {
        0 => { let n = i64::from_le_bytes(data.get(*pos..*pos+8)?.try_into().ok()?); *pos += 8; Some(Const::Int(n)) }
        1 => { let bits = u64::from_le_bytes(data.get(*pos..*pos+8)?.try_into().ok()?); *pos += 8; Some(Const::Float(f64::from_bits(bits))) }
        2 => Some(Const::Str(read_str(data, pos)?)),
        3 => { let b = *data.get(*pos)? != 0; *pos += 1; Some(Const::Bool(b)) }
        4 => Some(Const::None),
        5 => Some(Const::Code(std::rc::Rc::new(read_code(data, pos)?))),
        _ => None,
    }
}

fn read_op(data: &[u8], pos: &mut usize) -> Option<Op> {
    let tag = *data.get(*pos)?; *pos += 1;
    match tag {
        0 => Some(Op::LoadConst(read_u32(data, pos)?)),
        1 => Some(Op::LoadName(read_u32(data, pos)?)),
        2 => Some(Op::StoreName(read_u32(data, pos)?)),
        10 => Some(Op::BinaryAdd),
        11 => Some(Op::BinarySub),
        12 => Some(Op::BinaryMul),
        13 => Some(Op::BinaryDiv),
        14 => Some(Op::BinaryFloorDiv),
        15 => Some(Op::BinaryMod),
        16 => Some(Op::BinaryPow),
        17 => Some(Op::UnaryNeg),
        20 => Some(Op::CmpEq),
        21 => Some(Op::CmpNe),
        22 => Some(Op::CmpLt),
        23 => Some(Op::CmpGt),
        24 => Some(Op::CmpLe),
        25 => Some(Op::CmpGe),
        26 => Some(Op::UnaryNot),
        30 => Some(Op::JumpIfFalseOrPop(read_u32(data, pos)?)),
        31 => Some(Op::JumpIfTrueOrPop(read_u32(data, pos)?)),
        32 => Some(Op::Jump(read_u32(data, pos)?)),
        33 => Some(Op::JumpIfFalse(read_u32(data, pos)?)),
        34 => Some(Op::JumpIfTrue(read_u32(data, pos)?)),
        40 => Some(Op::MakeFunction),
        41 => Some(Op::CallFunction(read_u32(data, pos)?)),
        42 => Some(Op::ReturnValue),
        50 => Some(Op::GetIter),
        51 => Some(Op::ForIter(read_u32(data, pos)?)),
        60 => Some(Op::BuildList(read_u32(data, pos)?)),
        61 => Some(Op::BinarySubscr),
        62 => Some(Op::StoreSubscr),
        70 => Some(Op::Pop),
        71 => Some(Op::Dup),
        255 => Some(Op::Halt),
        _ => None,
    }
}
