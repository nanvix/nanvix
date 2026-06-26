// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::prog::{
    Inst,
    Prog,
};
use alloc::vec::Vec;

//==================================================================================================
// Structures
//==================================================================================================

/// A live NFA thread: a program counter plus its submatch slots.
struct Thread {
    pc: usize,
    slots: Vec<i32>,
    cost: i32,
}

/// A successful whole match plus its lazy-repetition priority cost.
struct Match {
    slots: Vec<i32>,
    cost: i32,
}

/// Pike VM execution state shared across the recursive thread expansion.
struct Vm<'a> {
    prog: &'a Prog,
    s: &'a [u8],
    notbol: bool,
    noteol: bool,
    /// Per-instruction generation markers used to deduplicate threads within a step.
    onlist: Vec<i32>,
}

impl Vm<'_> {
    /// Adds the thread at `pc` to `list`, following epsilon transitions (`Jmp`/`Split`/`Save`) and
    /// resolving anchors (`Bol`/`Eol`). Consuming instructions are appended to `list`.
    fn add_thread(
        &mut self,
        list: &mut Vec<Thread>,
        pc: usize,
        slots: &mut [i32],
        sp: usize,
        gen: i32,
        cost: i32,
    ) {
        if self.onlist.get(pc) == Some(&gen) {
            return;
        }
        if let Some(slot) = self.onlist.get_mut(pc) {
            *slot = gen;
        }
        let inst: &Inst = match self.prog.insts.get(pc) {
            Some(inst) => inst,
            None => return,
        };
        match *inst {
            Inst::Jmp(x) => self.add_thread(list, x, slots, sp, gen, cost),
            Inst::Split(x, y, penalize_y) => {
                let mut copy: Vec<i32> = slots.to_vec();
                self.add_thread(list, x, slots, sp, gen, cost);
                let y_cost: i32 = if penalize_y {
                    cost.saturating_add(1)
                } else {
                    cost
                };
                self.add_thread(list, y, &mut copy, sp, gen, y_cost);
            },
            Inst::Save(slot_idx) => {
                let saved: i32 = slots.get(slot_idx).copied().unwrap_or(-1);
                if let Some(cell) = slots.get_mut(slot_idx) {
                    *cell = i32::try_from(sp).unwrap_or(-1);
                }
                self.add_thread(list, pc + 1, slots, sp, gen, cost);
                if let Some(cell) = slots.get_mut(slot_idx) {
                    *cell = saved;
                }
            },
            Inst::Bol => {
                if sp == 0 {
                    if !self.notbol {
                        self.add_thread(list, pc + 1, slots, sp, gen, cost);
                    }
                } else if self.prog.newline && self.s.get(sp - 1) == Some(&b'\n') {
                    self.add_thread(list, pc + 1, slots, sp, gen, cost);
                }
            },
            Inst::Eol => {
                if sp == self.s.len() {
                    if !self.noteol {
                        self.add_thread(list, pc + 1, slots, sp, gen, cost);
                    }
                } else if self.prog.newline && self.s.get(sp) == Some(&b'\n') {
                    self.add_thread(list, pc + 1, slots, sp, gen, cost);
                }
            },
            // Consuming instruction (Char/Any/AnyByte/Set) or Match: park the thread for this step.
            _ => list.push(Thread {
                pc,
                slots: slots.to_vec(),
                cost,
            }),
        }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Returns `true` if `candidate` is a better whole match than `current`.
fn better_match(candidate: &Thread, current: Option<&Match>) -> bool {
    let candidate_start: i32 = candidate.slots.first().copied().unwrap_or(-1);
    let candidate_end: i32 = candidate.slots.get(1).copied().unwrap_or(-1);
    if candidate_start < 0 || candidate_end < candidate_start {
        return false;
    }

    let current: &Match = match current {
        Some(current) => current,
        None => return true,
    };
    let current_start: i32 = current.slots.first().copied().unwrap_or(-1);
    let current_end: i32 = current.slots.get(1).copied().unwrap_or(-1);

    if candidate_start != current_start {
        return current_start < 0 || candidate_start < current_start;
    }
    if candidate.cost != current.cost {
        return candidate.cost < current.cost;
    }
    candidate_end > current_end
}

/// Runs the compiled program against `s`, returning the winning submatch slots on success.
///
/// The result is a vector of `2 * (nsub + 1)` byte offsets (`-1` when unset); slot `0`/`1` hold the
/// whole match. Returns `None` if the pattern does not match.
pub(crate) fn exec(prog: &Prog, s: &[u8], notbol: bool, noteol: bool) -> Option<Vec<i32>> {
    let nslots: usize = 2 * (usize::try_from(prog.nsub).unwrap_or(0) + 1);
    let mut vm: Vm = Vm {
        prog,
        s,
        notbol,
        noteol,
        onlist: alloc::vec![-1i32; prog.insts.len()],
    };

    let mut clist: Vec<Thread> = Vec::new();
    let mut nlist: Vec<Thread> = Vec::new();
    let mut matched: Option<Match> = None;
    let mut gen: i32 = 0;

    {
        let mut init: Vec<i32> = alloc::vec![-1i32; nslots];
        vm.add_thread(&mut clist, 0, &mut init, 0, gen, 0);
    }

    let slen: usize = s.len();
    let mut sp: usize = 0;
    loop {
        nlist.clear();
        gen += 1;
        let ch: u8 = if sp < slen { s[sp] } else { 0 };
        let fch: u8 = if prog.icase {
            ch.to_ascii_lowercase()
        } else {
            ch
        };

        let mut t: usize = 0;
        while t < clist.len() {
            let pc: usize = clist[t].pc;
            let consume: bool = match prog.insts.get(pc) {
                Some(Inst::Char(c)) => sp < slen && fch == *c,
                Some(Inst::Any) => sp < slen && !(prog.newline && ch == b'\n'),
                Some(Inst::AnyByte) => sp < slen,
                Some(Inst::Set(set)) => {
                    sp < slen && (set[usize::from(ch >> 3)] & (1u8 << (ch & 7)) != 0)
                },
                Some(Inst::Match) => {
                    if better_match(&clist[t], matched.as_ref()) {
                        matched = Some(Match {
                            slots: clist[t].slots.clone(),
                            cost: clist[t].cost,
                        });
                    }
                    false
                },
                _ => false,
            };
            if consume {
                let mut sl: Vec<i32> = clist[t].slots.clone();
                vm.add_thread(&mut nlist, pc + 1, &mut sl, sp + 1, gen, clist[t].cost);
            }
            t += 1;
        }

        core::mem::swap(&mut clist, &mut nlist);
        if sp >= slen {
            break;
        }
        sp += 1;
    }

    matched.map(|matched| matched.slots)
}
