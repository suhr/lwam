use l0::ast::{Addr, Atom, CompiledFact, FactInstruction, QueryInstruction};

use std::vec::{Vec};

#[derive(Clone)]
enum HeapCell {
    NamedStr(usize, Atom),
    Ref(usize),
    Str(usize),
}

#[derive(Clone, Copy)]
enum MachineMode {
    Read,
    Write
}

type Heap = Vec<HeapCell>;

type Registers = Vec<HeapCell>;

pub struct Machine {
    h : usize,
    s : usize,
    pub fail : bool,
    heap : Heap,
    mode : MachineMode,
    pub program : Option<CompiledFact>,
    registers : Registers
}

impl Machine {
    pub fn new() -> Machine {
        Machine { h : 0,
                  s : 0,
                  fail : false,
                  heap : Vec::with_capacity(256),
                  mode : MachineMode::Write,
                  program : None,
                  registers : vec![HeapCell::Ref(0); 33] }
    }
    
    fn lookup(&self, a: Addr) -> &HeapCell {
        match a {
            Addr::HeapCell(hc) => &self.heap[hc],
            Addr::RegNum(reg)  => &self.registers[reg]
        }
    }

    fn deref(&self, a: Addr) -> Addr {
        let mut a = a;

        loop {
            if let &HeapCell::Ref(value) = self.lookup(a) {
                if let Addr::HeapCell(av) = a {
                    if value != av {
                        a = Addr::HeapCell(value);
                        continue;
                    }
                }
            }

            return a;
        };
    }

    fn is_unbound(hc: &HeapCell, index: usize) -> bool {
        match hc {
            &HeapCell::Ref(r) => r == index,
            _ => false
        }
    }
    
    //TODO: try to compress this function. currently it is dog shit.
    fn bind(&mut self, a: Addr, val: usize) {
        let mut a = a;
        
        loop {
            match a {
                Addr::RegNum(reg) => {
                    if let HeapCell::Ref(hc) = self.registers[reg] {
                        a = Addr::HeapCell(hc);
                    } else if Machine::is_unbound(&self.heap[val], val) {
                        self.heap[val] = self.registers[reg].clone();
                        break;
                    } else {
                        self.fail = true;
                        break;
                    }                        
                },
                Addr::HeapCell(hc) if Machine::is_unbound(&self.heap[hc], hc) => {
                    self.heap[hc] = HeapCell::Ref(val);
                    break;
                },
                Addr::HeapCell(hc) if Machine::is_unbound(&self.heap[val], val) => {                
                    self.heap[val] = HeapCell::Ref(hc);
                    break;
                },
                _ => {
                    self.fail = true;                    
                    break;
                }
            };
        }
    }

    fn unify(&mut self, a1: Addr, a2: Addr) {
        let mut pdl : Vec<Addr> = vec![a1, a2];

        self.fail = false;

        while !(pdl.is_empty() || self.fail) {
            let d1 = self.deref(pdl.pop().unwrap());
            let d2 = self.deref(pdl.pop().unwrap());

            if d1 != d2 {
                match (self.lookup(d1), self.lookup(d2)) {
                    (&HeapCell::Ref(hc), _) =>
                        self.bind(d2, hc),
                    (_, &HeapCell::Ref(hc)) =>
                        self.bind(d1, hc),
                    (&HeapCell::Str(a1), &HeapCell::Str(a2)) => {
                        let r1 = &self.heap[a1];
                        let r2 = &self.heap[a2];

                        if let &HeapCell::NamedStr(n1, ref f1) = r1 {
                            if let &HeapCell::NamedStr(n2, ref f2) = r2 {
                                if n1 == n2 && *f1 == *f2 {
                                    for i in 1 .. n1 {
                                        pdl.push(Addr::HeapCell(a1 + i));
                                        pdl.push(Addr::HeapCell(a2 + i));
                                    }

                                    continue;
                                }
                            }
                        }

                        self.fail = true;
                    },
                    _ => self.fail = true,
                };
            }
        }
    }    

    pub fn execute_query_instr<'a, 'b : 'a>(&'a mut self, instr: &'b QueryInstruction) {
        match instr {
            &QueryInstruction::PutStructure(ref name, arity, reg) => {
                self.heap.push(HeapCell::Str(self.h + 1));
                self.heap.push(HeapCell::NamedStr(arity, name.clone()));

                self.registers[reg] = self.heap[self.h].clone();

                self.h += 2;
            },
            &QueryInstruction::SetVariable(reg) => {
                self.heap.push(HeapCell::Ref(self.h));
                self.registers[reg] = self.heap[self.h].clone();

                self.h += 1;
            },
            &QueryInstruction::SetValue(reg) => {
                self.heap.push(self.registers[reg].clone());
                self.h += 1;
            },
        }
    }
    
    pub fn execute_fact_instr<'a, 'b : 'a>(&'a mut self, instr: &'b FactInstruction) {
        match instr {
            &FactInstruction::GetStructure(ref name, arity, reg) => {
                let addr = self.deref(Addr::RegNum(reg));

                match self.lookup(addr) {
                    &HeapCell::Str(a) => {
                        let result = &self.heap[a];

                        if let &HeapCell::NamedStr(named_arity, ref named_str) = result {
                            if arity == named_arity && *name == *named_str {
                                self.s = a + 1;
                                self.mode = MachineMode::Read;
                            } else {
                                self.fail = true;
                            }
                        }
                    },
                    &HeapCell::Ref(r) => {
                        self.heap.push(HeapCell::Str(self.h + 1));
                        self.heap.push(HeapCell::NamedStr(arity, name.clone()));

                        let h = self.h;

                        self.bind(Addr::HeapCell(r), h);

                        self.h += 2;
                        self.mode = MachineMode::Write;
                    },
                    _ => {
                        self.fail = true;
                    }
                };
            },
            &FactInstruction::UnifyVariable(reg) => {
                match self.mode {
                    MachineMode::Read  => self.registers[reg] = self.heap[self.s].clone(),
                    MachineMode::Write => {
                        self.heap.push(HeapCell::Ref(self.h));
                        self.registers[reg] = self.heap[self.h].clone();
                        self.h += 1;
                    }
                };

                self.s += 1;
            },
            &FactInstruction::UnifyValue(reg) => {
                let s = self.s;

                match self.mode {
                    MachineMode::Read  => self.unify(Addr::RegNum(reg), Addr::HeapCell(s)),
                    MachineMode::Write => {
                        self.heap.push(self.registers[reg].clone());
                        self.h += 1;
                    }
                };
                
                self.s += 1;
            }
        }
    }
    
    pub fn reset_heap(&mut self) {
        let program = self.program.take();

        *self = Machine::new();
        self.program = program;
    }    
}
