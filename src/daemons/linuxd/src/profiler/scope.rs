// Copyright(c) Microsoft Corporation.
// Licensed under the MIT license.

//======================================================================================================================
// Imports
//======================================================================================================================

use crate::profiler::PROFILER;
use ::std::{
    cell::RefCell,
    fmt::{
        self,
        Debug,
    },
    io,
    rc::Rc,
    time::Instant,
};

//======================================================================================================================
// Structures
//======================================================================================================================

/// Internal representation of scopes as a tree. This tracks a single profiling block of code in relationship to other
/// profiled blocks.
pub struct Scope {
    /// Name of the scope.
    name: &'static str,

    /// Parent scope in the tree. Root scopes have no parent.
    pred: Option<Rc<RefCell<Scope>>>,

    /// Child scopes in the tree.
    succs: Vec<Rc<RefCell<Scope>>>,

    /// How often has this scope been visited?
    num_calls: usize,

    /// In total, how much time has been spent in this scope?
    duration_sum: u128,
}

/// A guard that is created when entering a scope and dropped when leaving it.
pub struct Guard {
    enter_time: Instant,
}

//======================================================================================================================
// Associated Functions
//======================================================================================================================

impl Scope {
    pub fn new(name: &'static str, pred: Option<Rc<RefCell<Scope>>>) -> Scope {
        Scope {
            name,
            pred,
            succs: Vec::new(),
            num_calls: 0,
            duration_sum: 0,
        }
    }

    pub fn get_name(&self) -> &'static str {
        self.name
    }

    pub fn get_pred(&self) -> &Option<Rc<RefCell<Scope>>> {
        &self.pred
    }

    pub fn get_succs(&self) -> &Vec<Rc<RefCell<Scope>>> {
        &self.succs
    }

    pub fn add_succ(&mut self, succ: Rc<RefCell<Scope>>) {
        self.succs.push(succ.clone())
    }

    pub fn get_duration_sum(&self) -> u128 {
        self.duration_sum
    }

    /// Enter this scope. Returns a `Guard` instance that should be dropped
    /// when leaving the scope.
    #[inline]
    pub fn enter(&mut self) -> Guard {
        Guard::enter()
    }

    /// Leave this scope. Called automatically by the `Guard` instance.
    #[inline]
    pub fn leave(&mut self, duration: u128) {
        self.num_calls += 1;

        self.duration_sum += duration;
    }

    /// Dump statistics.
    pub fn write_recursive<W: io::Write>(
        &self,
        out: &mut W,
        total_duration: u128,
        depth: usize,
        max_depth: Option<usize>,
    ) -> io::Result<()> {
        if let Some(d) = max_depth {
            if depth > d {
                return Ok(());
            }
        }

        let total_duration_secs = (total_duration) as f64;
        let duration_sum_secs = (self.duration_sum) as f64;
        let pred_sum_secs = self
            .pred
            .clone()
            .map_or(total_duration_secs, |pred| (pred.borrow().duration_sum) as f64);
        let percent_time = duration_sum_secs / pred_sum_secs * 100.0;

        // Write markers.
        let mut markers = String::from("+");
        for _ in 0..depth {
            markers.push('+');
        }
        writeln!(
            out,
            "{},{},{:.2},{:.2}",
            format_args!("{},{}", markers, self.name),
            self.num_calls,
            percent_time,
            duration_sum_secs / (self.num_calls as f64),
        )?;

        // Write children
        for succ in &self.succs {
            succ.borrow()
                .write_recursive(out, total_duration, depth + 1, max_depth)?;
        }

        Ok(())
    }
}

impl Guard {
    #[inline]
    pub fn enter() -> Self {
        Self {
            enter_time: Instant::now(),
        }
    }
}

//======================================================================================================================
// Trait Implementations
//======================================================================================================================

impl Debug for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Drop for Guard {
    #[inline]
    fn drop(&mut self) {
        let duration = self.enter_time.elapsed().as_micros();

        PROFILER.with(|p| p.borrow_mut().leave_scope(duration));
    }
}
