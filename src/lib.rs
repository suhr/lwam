#![allow(dead_code)]

pub mod prolog;

// TODO: Vn denotes several instructions
pub enum Instruction {
    PutVariableX { n: usize, ai: usize },
    PutVariableY { n: usize, i: usize },
    PutValue { vn: usize, ai: usize },
    PutUnsafeValue { n: usize, ai: usize },
    PutStructure { f: (), ai: usize },
    PutList(usize),
    PutConstant { c: (), ai: usize },

    GetVariable { vn: usize, ai: usize },
    GetValue { vn: usize, ai: usize },
    GetStrucutre { f: (), ai: usize },
    GetList(usize),
    GetConstant { c: (), ai: usize },

    SetVariable(usize),
    SetValue(usize),
    SetLocalValue(usize),
    SetConstant(()),
    SetVoid(usize),

    UnifyVariable(usize),
    UnifyValue(usize),
    UnifyLocalValue(usize),
    UnifyConstant(()),
    UnifyVoid(usize),

    Allocate,
    Deallocate,
    Call { p: (), n: usize },
    Execute(()),
    Proceed,

    TryMeElse(()),
    RetryMeElse(()),
    TrustMe,
    Try(()),
    Retry(()),
    Trust(()),

    SwitchOnTerm { v: (), c: (), l: (), s: () },
    SwitchOnConstant { n: (), t: () },
    SwitchOnStructure { n: (), t: () },

    NeckCut,
    GetLevel(usize),
    Cut(usize),
}

pub struct Machine {}
