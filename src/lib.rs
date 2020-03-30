#![allow(dead_code)]

pub mod prolog;

pub struct Constant(usize);
pub struct Functor {
    name: usize,
    arity: u8,
}

pub struct Predicate(usize);

// TODO: Vn denotes several instructions
pub enum Instruction {
    PutVariable { is_temp: bool, vn: usize, ai: usize },
    PutValue { is_temp: bool, vn: usize, ai: usize },
    PutUnsafeValue { n: usize, ai: usize },
    PutStructure { f: Functor, ai: usize },
    PutList(usize),
    PutConstant { c: Constant, ai: usize },

    GetVariable { is_temp: bool, vn: usize, ai: usize },
    GetValue { is_temp: bool, vn: usize, ai: usize },
    GetStrucutre { f: Functor, ai: usize },
    GetList(usize),
    GetConstant { c: Constant, ai: usize },

    SetVariable(usize),
    SetValue(usize),
    SetLocalValue(usize),
    SetConstant(Constant),
    SetVoid(usize),

    UnifyVariable(usize),
    UnifyValue(usize),
    UnifyLocalValue(usize),
    UnifyConstant(Constant),
    UnifyVoid(usize),

    Allocate,
    Deallocate,
    Call { p: Predicate, n: usize },
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

enum Cell {
    Constant(Constant),
    List(usize),
    NamedStruct { name: usize, len: usize },
    HeapRef(usize),
    StackRef(usize, usize),
    Struct(usize)
}

pub struct Machine {
}
