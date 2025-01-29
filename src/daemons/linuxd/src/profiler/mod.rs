// Copyright(c) Microsoft Corporation.
// Licensed under the MIT license.

//======================================================================================================================
// Exports
//======================================================================================================================

mod scope;

//======================================================================================================================
// Imports
//======================================================================================================================

use ::std::{
    self,
    cell::RefCell,
    io,
    rc::Rc,
};
use scope::{
    Guard,
    Scope,
};

//==================================================================================================
// Structures
//==================================================================================================

#[cfg(feature = "auto-calibrate")]
const SAMPLE_SIZE: usize = 10_000;

thread_local!(
    /// Global thread-local instance of the profiler.
    pub static PROFILER: RefCell<Profiler> = RefCell::new(Profiler::new())
);

/// A `Profiler` stores the scope tree and keeps track of the currently active
/// scope.
///
/// Note that there is a global thread-local instance of `Profiler` in
/// [`PROFILER`](constant.PROFILER.html), so it is not possible to manually
/// create an instance of `Profiler`.
pub struct Profiler {
    roots: Vec<Rc<RefCell<Scope>>>,
    current: Option<Rc<RefCell<Scope>>>,
    #[cfg(feature = "auto-calibrate")]
    clock_drift: u128,
}

//==================================================================================================
// Associated Functions
//==================================================================================================

impl Profiler {
    fn new() -> Profiler {
        Profiler {
            roots: Vec::new(),
            current: None,
            #[cfg(feature = "auto-calibrate")]
            clock_drift: Self::clock_drift(SAMPLE_SIZE),
        }
    }

    /// Create and enter a syncronous scope. Returns a [`Guard`](struct.Guard.html) that should be
    /// dropped upon leaving the scope.
    ///
    /// Usually, this method will be called by the
    /// [`profile`](macro.profile.html) macro, so it does not need to be used
    /// directly.
    #[inline]
    pub fn sync_scope(&mut self, name: &'static str) -> Guard {
        let scope = self.get_scope(name);
        self.enter_scope(scope)
    }

    /// Looks up the scope at the root level using the name, creating a new one if not found.
    fn get_root_scope(&mut self, name: &'static str) -> Rc<RefCell<Scope>> {
        //Check if `name` already is a root.
        let existing_root = self
            .roots
            .iter()
            .find(|root| root.borrow().get_name() == name)
            .cloned();

        existing_root.unwrap_or_else(|| {
            // Add a new root node.
            let new_scope: Scope = Scope::new(name, None);
            let succ = Rc::new(RefCell::new(new_scope));

            self.roots.push(succ.clone());

            succ
        })
    }

    /// Look up the scope using the name.
    pub fn get_scope(&mut self, name: &'static str) -> Rc<RefCell<Scope>> {
        // Check if we have already registered `name` at the current point in
        // the tree.
        if let Some(current) = self.current.as_ref() {
            // We are currently in some scope.
            let existing_succ = current
                .borrow()
                .get_succs()
                .iter()
                .find(|succ| succ.borrow().get_name() == name)
                .cloned();

            existing_succ.unwrap_or_else(|| {
                // Add new successor node to the current node.
                let new_scope: Scope = Scope::new(name, Some(current.clone()));
                let succ = Rc::new(RefCell::new(new_scope));

                current.borrow_mut().add_succ(succ.clone());

                succ
            })
        } else {
            // We are currently not within any scope.
            self.get_root_scope(name)
        }
    }

    /// Actually enter a scope.
    fn enter_scope(&mut self, scope: Rc<RefCell<Scope>>) -> Guard {
        let guard = scope.borrow_mut().enter();
        self.current = Some(scope);

        guard
    }

    /// Leave the current scope.
    #[inline]
    fn leave_scope(&mut self, duration: u128) {
        self.current = if let Some(current) = self.current.as_ref() {
            cfg_if::cfg_if! {
                if #[cfg(feature = "auto-calibrate")] {
                    let d = duration.checked_sub(self.clock_drift);
                    current.borrow_mut().leave(d.unwrap_or(duration));
                } else {
                    current.borrow_mut().leave(duration);
                }
            }

            // Set current scope back to the parent node (if any).
            current.borrow().get_pred().as_ref().cloned()
        } else {
            // This should not happen with proper usage.
            log::error!("Called perftools::profiler::leave() while not in any scope");

            None
        };
    }

    fn write<W: io::Write>(&self, out: &mut W, max_depth: Option<usize>) -> io::Result<()> {
        let total_duration: u128 = self
            .roots
            .iter()
            .map(|root| root.borrow().get_duration_sum())
            .sum();

        writeln!(out, "call_depth,function_name,num_calls,percent_time,microsecs_per_call")?;
        for root in self.roots.iter() {
            root.borrow()
                .write_recursive(out, total_duration, 0, max_depth)?;
        }

        out.flush()
    }

    #[cfg(feature = "auto-calibrate")]
    fn clock_drift(nsamples: usize) -> u128 {
        use std::time::Instant;

        let mut total = 0;

        for _ in 0..nsamples {
            let now = Instant::now();
            let duration: u128 = now.elapsed().as_micros();

            total += duration;
        }

        total / (nsamples as u128)
    }
}

//======================================================================================================================
// Trait Implementations
//======================================================================================================================

impl Drop for Profiler {
    fn drop(&mut self) {
        if let Err(e) = self.write(&mut std::io::stderr(), None) {
            log::error!("Failed to write profile data (error={})", e);
        }
    }
}
