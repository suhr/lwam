use crate::prolog::ast::*;

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::collections::btree_map::{IntoIter, IterMut, Values};
use std::mem::swap;
use std::vec::Vec;

pub type OccurrenceSet = BTreeSet<(GenContext, usize)>;

pub type HeapVarDict<'a>  = HashMap<&'a Var, Addr>;
pub type AllocVarDict<'a> = HashMap<&'a Var, VarData>;

pub struct TempVarData {
    last_term_arity: usize,
    pub use_set: OccurrenceSet,
    pub no_use_set: BTreeSet<usize>,
    pub conflict_set: BTreeSet<usize>
}

// labeled with chunk numbers.
pub enum VarStatus {
    Perm(usize), Temp(usize, TempVarData) // Perm(chunk_num) | Temp(chunk_num, _)
}

// Perm: 0 initially, a stack register once processed.
// Temp: labeled with chunk_num and temp offset (unassigned if 0).
pub enum VarData {
    Perm(usize), Temp(usize, usize, TempVarData)
}

impl VarData {
    pub fn as_reg_type(&self) -> RegType {
        match self {
            &VarData::Temp(_, r, _) => RegType::Temp(r),
            &VarData::Perm(r) => RegType::Perm(r)
        }
    }
}

impl TempVarData {
    fn new(last_term_arity: usize) -> Self {
        TempVarData {
            last_term_arity: last_term_arity,
            use_set: BTreeSet::new(),
            no_use_set: BTreeSet::new(),
            conflict_set: BTreeSet::new()
        }
    }

    fn uses_reg(&self, reg: usize) -> bool {
        for &(_, nreg) in self.use_set.iter() {
            if reg == nreg {
                return true;
            }
        }

        return false;
    }

    fn populate_conflict_set(&mut self) {
        if self.last_term_arity > 0 {
            let arity = self.last_term_arity;            
            let mut conflict_set : BTreeSet<usize> = (1..arity).collect();

            for &(_, reg) in self.use_set.iter() {
                conflict_set.remove(&reg);
            }

            self.conflict_set = conflict_set;
        }
    }
}

type VariableFixture<'a>  = (VarStatus, Vec<&'a Cell<VarReg>>);
pub struct VariableFixtures<'a>(BTreeMap<&'a Var, VariableFixture<'a>>);

impl<'a> VariableFixtures<'a>
{
    pub fn new() -> Self {
        VariableFixtures(BTreeMap::new())
    }

    pub fn insert(&mut self, var: &'a Var, vs: VariableFixture<'a>) {
        self.0.insert(var, vs);
    }

    // computes no_use and conflict sets for all temp vars.
    pub fn populate_restricting_sets(&mut self)
    {
        // three stages:
        // 1. move the use sets of each variable to a local HashMap, use_set
        // (iterate mutably, swap mutable refs).
        // 2. drain use_set. For each use set of U, add into the
        // no-use sets of appropriate variables T /= U.
        // 3. Move the use sets back to their original locations in the fixture.
        // Compute the conflict set of u.

        // 1.
        let mut use_sets : HashMap<&'a Var, OccurrenceSet> = HashMap::new();

        for (ref var, &mut (ref mut var_status, _)) in self.iter_mut() {
            if let &mut VarStatus::Temp(_, ref mut var_data) = var_status {
                let mut use_set = OccurrenceSet::new();

                swap(&mut var_data.use_set, &mut use_set);
                use_sets.insert(var, use_set);
            }
        }

        for (u, use_set) in use_sets.drain() {
            // 2.
            for &(term_loc, reg) in use_set.iter() {
                if let GenContext::Last(cn_u) = term_loc {
                    for (ref t, &mut (ref mut var_status, _)) in self.iter_mut() {
                        if let &mut VarStatus::Temp(cn_t, ref mut t_data) = var_status {
                            if cn_u == cn_t && *u != ***t {
                                if !t_data.uses_reg(reg) {
                                    t_data.no_use_set.insert(reg);
                                }
                            }
                        }
                    }
                }
            }

            // 3.
            match self.get_mut(u).unwrap() {
                &mut (VarStatus::Temp(_, ref mut u_data), _) => {
                    u_data.use_set = use_set;
                    u_data.populate_conflict_set();
                },
                _ => {}
            };
        }
    }

    fn get_mut(&mut self, u: &'a Var) -> Option<&mut VariableFixture<'a>> {
        self.0.get_mut(u)
    }

    fn iter_mut(&mut self) -> IterMut<&'a Var, VariableFixture<'a>> {
        self.0.iter_mut()
    }

    fn record_temp_info(&mut self,
                        tvd: &mut TempVarData,
                        arg_c: usize,
                        term_loc: GenContext)
    {
        match term_loc {
            GenContext::Head | GenContext::Last(_) => {
                tvd.use_set.insert((term_loc, arg_c));
            },
            _ => {}
        };
    }

    pub fn vars_above_threshold(&self, index: usize) -> usize
    {
        let mut var_count = 0;

        for &(ref var_status, _) in self.values() {
            if let &VarStatus::Perm(i) = var_status {
                if i > index {
                    var_count += 1;
                }
            }
        }

        var_count
    }

    pub fn mark_vars_in_chunk(&mut self,
                              term: &'a Term,
                              last_term_arity: usize,
                              chunk_num: usize,
                              term_loc: GenContext)
    {
        let mut arg_c = 1;

        for term_ref in term.breadth_first_iter() {
            if let TermRef::Var(lvl, cell, var) = term_ref {
                let mut status = self.0.remove(var)
                    .unwrap_or((VarStatus::Temp(chunk_num, TempVarData::new(last_term_arity)),
                                Vec::new()));

                status.1.push(cell);

                match status.0 {
                    VarStatus::Temp(cn, ref mut tvd) if cn == chunk_num => {
                        if let Level::Shallow = lvl {
                            self.record_temp_info(tvd, arg_c, term_loc);
                        }
                    },
                    _ => status.0 = VarStatus::Perm(chunk_num)
                };

                self.0.insert(var, status);
            }

            if let Level::Shallow = term_ref.level() {
                arg_c += 1;
            }
        }
    }

    pub fn into_iter(self) -> IntoIter<&'a Var, VariableFixture<'a>> {
        self.0.into_iter()
    }

    fn values(&self) -> Values<&'a Var, VariableFixture<'a>> {
        self.0.values()
    }

    pub fn set_perm_vals(&self, has_deep_cuts: bool)
    {
        let mut values_vec : Vec<_> = self.values()
            .filter_map(|ref v| {
                match &v.0 {
                    &VarStatus::Perm(i) => Some((i, &v.1)),
                    _ => None
                }
            })
            .collect();

        values_vec.sort_by_key(|ref v| v.0);

        let offset = has_deep_cuts as usize;

        for (i, (_, cells)) in values_vec.into_iter().rev().enumerate() {
            for cell in cells {
                cell.set(VarReg::Norm(RegType::Perm(i + 1 + offset)));
            }
        }
    }

    fn mark_unsafe_vars(&self, unsafe_vars: &mut HashMap<RegType, bool>, query: &mut CompiledQuery)
    {
        for query_instr in query.iter_mut() {
            match query_instr {
                &mut QueryInstruction::PutValue(RegType::Perm(i), arg) =>
                    if let Some(found) = unsafe_vars.get_mut(&RegType::Perm(i)) {
                        if !*found {
                            *found = true;
                            *query_instr = QueryInstruction::PutUnsafeValue(i, arg);
                        }
                    },
                &mut QueryInstruction::SetVariable(reg)
              | &mut QueryInstruction::PutVariable(reg, _) =>
                    if let Some(found) = unsafe_vars.get_mut(&reg) {
                        *found = true;
                    },
                &mut QueryInstruction::SetValue(reg) =>
                    if let Some(found) = unsafe_vars.get_mut(&reg) {
                        if !*found {
                            *found = true;
                            *query_instr = QueryInstruction::SetLocalValue(reg);
                        }
                    },
                _ => {}
            };
        }
    }

    fn record_unsafe_vars(&self, unsafe_vars: &mut HashMap<RegType, bool>) {        
        for &(_, ref cb) in self.values() {
            match cb.first() {
                Some(index) => {
                    unsafe_vars.insert(index.get().norm(), false);
                },
                None => {}
            };
        }
    }
    
    fn mark_head_vars_as_safe(&self, head: &Term, unsafe_vars: &mut HashMap<RegType, bool>)
    {                
        for term_ref in head.breadth_first_iter() {
            match term_ref {
                TermRef::Var(_, cell, _) => {
                    unsafe_vars.remove(&cell.get().norm());
                },
                _ => {}
            };
        }
    }

    pub fn mark_unsafe_vars_in_query(&self, query: &mut CompiledQuery) {
        let mut unsafe_vars = HashMap::new();

        self.record_unsafe_vars(&mut unsafe_vars);
        self.mark_unsafe_vars(&mut unsafe_vars, query);
    }
    
    pub fn mark_unsafe_vars_in_rule(&self, head: &Term, query: &mut CompiledQuery)
    {
        let mut unsafe_vars = HashMap::new();

        self.record_unsafe_vars(&mut unsafe_vars);        
        self.mark_head_vars_as_safe(head, &mut unsafe_vars);
        self.mark_unsafe_vars(&mut unsafe_vars, query);
    }
}
