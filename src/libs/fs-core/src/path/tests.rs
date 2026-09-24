// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    walk,
    Adapter,
    Component::{
        self,
        Current,
        Name,
        Parent as ParentComponent,
    },
    EntryKind::{
        self,
        Directory,
        Other,
        Symlink,
    },
    FinalComponent::{
        self,
        FollowExisting,
        FollowOrMissing,
        LeaveUninspected,
    },
    WalkError::{
        self,
        Backend,
        NotDirectory,
        TooManySymlinks,
    },
};
use core::{
    array::IntoIter,
    cell::RefCell,
};
use Call::*;

#[derive(Debug, PartialEq, Eq)]
enum Fault {
    Missing,
    Denied,
    Storage,
    LinkRead,
}

#[derive(Debug)]
enum Call {
    Next(Option<Component<u8>>),
    NeedsDirectory(bool),
    Push(u8, Result<u8, Fault>),
    Parent(Result<(), Fault>),
    Inspect(Result<EntryKind, Fault>),
    Expand(u8, Result<(), Fault>),
}

// Only an expected call sequence: no parsing, path storage, lookup, or link expansion logic.
// Interior mutability lets the script also check requires_directory(), which takes &self.
struct Script<const N: usize> {
    calls: RefCell<IntoIter<Call, N>>,
}

impl<const N: usize> Script<N> {
    fn next_call(&self) -> Call {
        self.calls
            .borrow_mut()
            .next()
            .expect("unexpected call after script ended")
    }
}

impl<const N: usize> Adapter for Script<N> {
    type Name = u8;
    type Parent = u8;
    type Error = Fault;

    fn next_component(&mut self) -> Option<Component<u8>> {
        match self.next_call() {
            Next(component) => component,
            call => panic!("next_component: unexpected {call:?}"),
        }
    }

    fn requires_directory(&self) -> bool {
        match self.next_call() {
            NeedsDirectory(required) => required,
            call => panic!("requires_directory: unexpected {call:?}"),
        }
    }

    fn push(&mut self, name: u8) -> Result<u8, Fault> {
        match self.next_call() {
            Push(expected, result) => {
                assert_eq!(name, expected, "appended the wrong name token");
                result
            },
            call => panic!("push: unexpected {call:?}"),
        }
    }

    fn parent(&mut self) -> Result<(), Fault> {
        match self.next_call() {
            Parent(result) => result,
            call => panic!("parent: unexpected {call:?}"),
        }
    }

    fn inspect(&mut self) -> Result<EntryKind, Fault> {
        match self.next_call() {
            Inspect(result) => result,
            call => panic!("inspect: unexpected {call:?}"),
        }
    }

    fn expand_link(&mut self, parent: u8) -> Result<(), Fault> {
        match self.next_call() {
            Expand(expected, result) => {
                assert_eq!(parent, expected, "expanded against the wrong parent checkpoint");
                result
            },
            call => panic!("expand_link: unexpected {call:?}"),
        }
    }

    fn is_not_found(error: &Fault) -> bool {
        *error == Fault::Missing
    }
}

fn check<const N: usize>(
    calls: [Call; N],
    policy: FinalComponent,
    budget: usize,
    expected: Result<(), WalkError<Fault>>,
) {
    let mut adapter = Script {
        calls: RefCell::new(calls.into_iter()),
    };
    assert_eq!(walk(&mut adapter, policy, budget), expected);
    assert!(adapter.calls.into_inner().next().is_none(), "expected calls were not consumed");
}

#[test]
fn empty_dot_and_parent_need_no_lookup() {
    check([Next(None)], FollowExisting, 0, Ok(()));
    check(
        [
            Next(Some(Current)),
            Next(Some(ParentComponent)),
            Parent(Ok(())),
            Next(None),
        ],
        FollowExisting,
        0,
        Ok(()),
    );
}

#[test]
fn following_policies_inspect_an_existing_final_entry() {
    for policy in [FollowExisting, FollowOrMissing] {
        check(
            [
                Next(Some(Name(1))),
                Push(1, Ok(7)),
                NeedsDirectory(false),
                Inspect(Ok(Other)),
                Next(None),
            ],
            policy,
            0,
            Ok(()),
        );
    }
}

#[test]
fn missing_final_is_allowed_only_for_creation() {
    for (policy, expected) in [
        (FollowExisting, Err(Backend(Fault::Missing))),
        (FollowOrMissing, Ok(())),
    ] {
        check(
            [
                Next(Some(Name(1))),
                Push(1, Ok(7)),
                NeedsDirectory(false),
                Inspect(Err(Fault::Missing)),
            ],
            policy,
            0,
            expected,
        );
    }
}

#[test]
fn nofollow_skips_only_the_final_lookup() {
    // Expansion must precede the target's components and the original parent/final suffix.
    // Tokens 1, 2, and 3 stand for the link, its target, and the original final name.
    check(
        [
            Next(Some(Name(1))),
            Push(1, Ok(7)),
            NeedsDirectory(true),
            Inspect(Ok(Symlink)),
            Expand(7, Ok(())),
            Next(Some(Name(2))),
            Push(2, Ok(8)),
            NeedsDirectory(true),
            Inspect(Ok(Directory)),
            Next(Some(ParentComponent)),
            Parent(Ok(())),
            Next(Some(Name(3))),
            Push(3, Ok(9)),
            NeedsDirectory(false),
            Next(None),
        ],
        LeaveUninspected,
        1,
        Ok(()),
    );
}

#[test]
fn missing_ancestor_and_directory_requirement_are_not_bypassed() {
    for policy in [FollowExisting, FollowOrMissing, LeaveUninspected] {
        check(
            [
                Next(Some(Name(1))),
                Push(1, Ok(7)),
                NeedsDirectory(true),
                Inspect(Err(Fault::Missing)),
            ],
            policy,
            0,
            Err(Backend(Fault::Missing)),
        );
        // The adapter reports the same requirement for a suffix or a retained trailing separator.
        check(
            [
                Next(Some(Name(1))),
                Push(1, Ok(7)),
                NeedsDirectory(true),
                Inspect(Ok(Other)),
            ],
            policy,
            0,
            Err(NotDirectory),
        );
    }
}

#[test]
fn final_link_target_must_exist_even_in_creation_mode() {
    check(
        [
            Next(Some(Name(1))),
            Push(1, Ok(7)),
            NeedsDirectory(false),
            Inspect(Ok(Symlink)),
            Expand(7, Ok(())),
            Next(Some(Name(2))),
            Push(2, Ok(8)),
            NeedsDirectory(false),
            Inspect(Ok(Symlink)),
            Expand(8, Ok(())),
            Next(Some(Name(3))),
            Push(3, Ok(9)),
            NeedsDirectory(false),
            Inspect(Err(Fault::Missing)),
        ],
        FollowOrMissing,
        2,
        Err(Backend(Fault::Missing)),
    );
}

#[test]
fn intermediate_link_preserves_original_final_policy() {
    for (policy, expected) in [
        (FollowExisting, Err(Backend(Fault::Missing))),
        (FollowOrMissing, Ok(())),
    ] {
        check(
            [
                Next(Some(Name(1))),
                Push(1, Ok(7)),
                NeedsDirectory(true),
                Inspect(Ok(Symlink)),
                Expand(7, Ok(())),
                Next(Some(Name(2))),
                Push(2, Ok(8)),
                NeedsDirectory(true),
                Inspect(Ok(Directory)),
                Next(Some(Name(3))),
                Push(3, Ok(9)),
                NeedsDirectory(false),
                Inspect(Err(Fault::Missing)),
            ],
            policy,
            1,
            expected,
        );
    }
}

#[test]
fn intermediate_link_cannot_reenable_missing_target_allowance() {
    // Token 1 is the original final link; token 2 is an intermediate link inside its target.
    // Expanding token 2 must not undo the existence requirement imposed by following token 1.
    check(
        [
            Next(Some(Name(1))),
            Push(1, Ok(7)),
            NeedsDirectory(false),
            Inspect(Ok(Symlink)),
            Expand(7, Ok(())),
            Next(Some(Name(2))),
            Push(2, Ok(8)),
            NeedsDirectory(true),
            Inspect(Ok(Symlink)),
            Expand(8, Ok(())),
            Next(Some(Name(3))),
            Push(3, Ok(9)),
            NeedsDirectory(true),
            Inspect(Ok(Directory)),
            Next(Some(Name(4))),
            Push(4, Ok(10)),
            NeedsDirectory(false),
            Inspect(Err(Fault::Missing)),
        ],
        FollowOrMissing,
        2,
        Err(Backend(Fault::Missing)),
    );
}

#[test]
fn budget_is_checked_before_expansion_and_applies_to_the_whole_walk() {
    check(
        [
            Next(Some(Name(1))),
            Push(1, Ok(7)),
            NeedsDirectory(false),
            Inspect(Ok(Symlink)),
        ],
        FollowExisting,
        0,
        Err(TooManySymlinks),
    );
    check(
        [
            Next(Some(Name(1))),
            Push(1, Ok(7)),
            NeedsDirectory(false),
            Inspect(Ok(Symlink)),
            Expand(7, Ok(())),
            Next(Some(Name(2))),
            Push(2, Ok(8)),
            NeedsDirectory(false),
            Inspect(Ok(Other)),
            Next(None),
        ],
        FollowExisting,
        1,
        Ok(()),
    );
    check(
        [
            Next(Some(Name(1))),
            Push(1, Ok(7)),
            NeedsDirectory(true),
            Inspect(Ok(Symlink)),
            Expand(7, Ok(())),
            Next(Some(Name(2))),
            Push(2, Ok(8)),
            NeedsDirectory(true),
            Inspect(Ok(Directory)),
            Next(Some(ParentComponent)),
            Parent(Ok(())),
            Next(Some(Name(3))),
            Push(3, Ok(9)),
            NeedsDirectory(false),
            Inspect(Ok(Symlink)),
        ],
        FollowExisting,
        1,
        Err(TooManySymlinks),
    );
}

#[test]
fn backend_errors_keep_their_identity_and_stop_traversal() {
    check(
        [Next(Some(Name(1))), Push(1, Err(Fault::Storage))],
        FollowExisting,
        1,
        Err(Backend(Fault::Storage)),
    );
    check(
        [Next(Some(ParentComponent)), Parent(Err(Fault::Denied))],
        FollowExisting,
        1,
        Err(Backend(Fault::Denied)),
    );
    check(
        [
            Next(Some(Name(1))),
            Push(1, Ok(7)),
            NeedsDirectory(false),
            Inspect(Err(Fault::Denied)),
        ],
        FollowOrMissing,
        1,
        Err(Backend(Fault::Denied)),
    );
    check(
        [
            Next(Some(Name(1))),
            Push(1, Ok(7)),
            NeedsDirectory(false),
            Inspect(Ok(Symlink)),
            Expand(7, Err(Fault::LinkRead)),
        ],
        FollowOrMissing,
        1,
        Err(Backend(Fault::LinkRead)),
    );
}
