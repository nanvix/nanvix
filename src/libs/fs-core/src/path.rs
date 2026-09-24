// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Ordered, symlink-aware traversal without prescribing path representation or filesystem access.

/// The three component actions distinguished by POSIX pathname resolution.
///
/// [POSIX.1-2024 §4.16] gives `.` and `..` special meanings; other filenames require a lookup.
/// Separators and root selection are parser/adapter concerns, not additional component actions.
/// Names need not be UTF-8 or borrow from pending input.
///
/// [POSIX.1-2024 §4.16]: https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/V1_chap04.html#tag_04_16
#[derive(Debug, PartialEq, Eq)]
pub enum Component<N> {
    /// Remain at the current directory.
    Current,
    /// Move to the parent after resolving preceding links.
    Parent,
    /// An ordinary filesystem name.
    Name(N),
}

/// The entry distinctions needed for traversal, not a complete POSIX file-type enumeration.
///
/// [POSIX pathname resolution] requires expanding links and requiring directories before subsequent
/// components. All remaining file types take the same traversal branch, so `Other` includes regular
/// files, devices, FIFOs, and sockets. The POSIX adapter obtains this information with [`lstat()`],
/// which reports the link itself instead of following it.
///
/// [POSIX pathname resolution]: https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/V1_chap04.html#tag_04_16
/// [`lstat()`]: https://pubs.opengroup.org/onlinepubs/9799919799/functions/lstat.html
#[derive(Debug, PartialEq, Eq)]
pub enum EntryKind {
    /// A symbolic link whose target still needs walking.
    Symlink,
    /// An ordinary directory.
    Directory,
    /// A non-directory, non-symlink entry.
    Other,
}

/// Final-entry policies needed by libc resolution and hostfs preflight, not POSIX flag values.
///
/// [POSIX.1-2024 §4.16] distinguishes resolving an existing entry from resolving a parent before
/// creating an entry, and makes final-link following depend on the operation. These policies cover
/// the consumers' current combinations; they do not implement each operation's existence, type,
/// permission, or atomicity checks. Those checks remain with the final filesystem operation.
///
/// [POSIX.1-2024 §4.16]: https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/V1_chap04.html#tag_04_16
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinalComponent {
    /// Follow the final link and require the result to exist, as required by POSIX [`realpath()`].
    /// Its `ENOENT` condition includes an absent final entry or link target.
    ///
    /// [`realpath()`]: https://pubs.opengroup.org/onlinepubs/9799919799/functions/realpath.html
    FollowExisting,
    /// Permit an absent ordinary final entry, but require a followed link's target to exist.
    /// This preserves Windows hostfs preflight, not full POSIX [`open()`] `O_CREAT` semantics:
    /// creating through a dangling final link is not supported by this policy. The caller still
    /// performs the actual open/create and its flag checks.
    ///
    /// [`open()`]: https://pubs.opengroup.org/onlinepubs/9799919799/functions/open.html
    FollowOrMissing,
    /// Resolve the parent and return the final name without inspecting it, whether or not it exists.
    /// This supports operations acting on the link itself (such as POSIX [`lstat()`]) or creating
    /// a directory entry. The operation, not the walker, checks final existence/type. This is not
    /// `O_NOFOLLOW`, which requires `open()` to reject a final symlink. Retained trailing separators
    /// still require directory traversal, as specified in POSIX pathname resolution.
    ///
    /// [`lstat()`]: https://pubs.opengroup.org/onlinepubs/9799919799/functions/lstat.html
    LeaveUninspected,
}

/// Algorithm-detected failures plus the adapter's unchanged errors.
///
/// [POSIX pathname resolution] specifies `ELOOP` for excessive link following and requires a
/// directory for further traversal (`ENOTDIR` in [`realpath()`]). Those are the two failures the
/// walker can detect itself. Lookup/storage failures such as `ENOENT`, `EACCES`, and `ENAMETOOLONG`
/// remain `Backend` errors so guest errno or native host error details are not lost.
///
/// [POSIX pathname resolution]: https://pubs.opengroup.org/onlinepubs/9799919799/basedefs/V1_chap04.html#tag_04_16
/// [`realpath()`]: https://pubs.opengroup.org/onlinepubs/9799919799/functions/realpath.html
#[derive(Debug, PartialEq, Eq)]
pub enum WalkError<E> {
    /// Path storage, anchoring, or filesystem access failed.
    Backend(E),
    /// Another link would exceed the operation-wide expansion budget.
    TooManySymlinks,
    /// A non-directory entry was followed by more path syntax.
    NotDirectory,
}

/// Storage and filesystem primitives used by [`walk`].
///
/// # Description
///
/// Initialize the current path and pending input before walking. The adapter owns all path storage;
/// the walker neither allocates nor interprets strings. Operations may parse/copy components but
/// must not implement another traversal loop. Root, cwd, native prefixes, and containment remain
/// the caller's responsibility. This interface does not prevent canonicalize-then-use races.
pub trait Adapter {
    /// An owned name or an index into pending storage, valid until consumed by `push`.
    type Name;
    /// A checkpoint identifying the parent of the last appended name.
    type Parent;
    /// The consumer's native error type.
    type Error;

    /// Returns the next component, or `None` after input is exhausted.
    /// Parsing must not remove a parent component or resolve a link.
    fn next_component(&mut self) -> Option<Component<Self::Name>>;

    /// Whether unconsumed components or retained trailing separators require a directory.
    /// Called immediately after consuming a name. Parsing policy is consumer-specific.
    fn requires_directory(&self) -> bool;

    /// Appends one name and returns its parent checkpoint.
    ///
    /// # Errors
    /// Returns the adapter's error if the output cannot hold the name.
    fn push(&mut self, name: Self::Name) -> Result<Self::Parent, Self::Error>;

    /// Applies one parent step, retaining the consumer's root behavior.
    ///
    /// # Errors
    /// Returns the adapter's error if the parent step is invalid.
    fn parent(&mut self) -> Result<(), Self::Error>;

    /// Inspects the current entry without following a final symlink.
    /// Native canonicalization of ordinary entries may update the current path here.
    ///
    /// # Errors
    /// Preserves filesystem lookup errors, including missing entries.
    fn inspect(&mut self) -> Result<EntryKind, Self::Error>;

    /// Reads the current link, restores its parent (or target root), and prepends the target
    /// to the unconsumed input. It must not walk that target or discard the original suffix.
    ///
    /// # Errors
    /// Preserves link-read, target-validation, anchoring, and storage errors.
    fn expand_link(&mut self, parent: Self::Parent) -> Result<(), Self::Error>;

    /// Whether an inspection error means the current entry is absent, not another failure.
    fn is_not_found(error: &Self::Error) -> bool;
}

/// Walks an adapter's pending path in filesystem order.
///
/// # Parameters
///
/// - `adapter`: Initialized path storage and filesystem operations.
/// - `final_component`: Policy for the original final entry, not intermediate link targets.
/// - `max_symlinks`: Maximum expansions across the entire operation, including chained links.
///
/// # Returns
///
/// On success, the adapter holds the resulting path. An empty input leaves its initial path intact;
/// the caller decides whether an empty request is valid and how to finish the output.
///
/// # Errors
///
/// Returns [`WalkError::Backend`] for adapter failures, [`WalkError::NotDirectory`] when remaining
/// syntax requires a directory, or [`WalkError::TooManySymlinks`] when expansion would exceed the
/// budget. On failure the adapter's partially traversed path must not be used as a successful result.
pub fn walk<A: Adapter>(
    adapter: &mut A,
    final_component: FinalComponent,
    max_symlinks: usize,
) -> Result<(), WalkError<A::Error>> {
    let mut links_left = max_symlinks;
    let mut allow_missing = final_component == FinalComponent::FollowOrMissing;
    while let Some(component) = adapter.next_component() {
        let name = match component {
            Component::Current => continue,
            Component::Parent => {
                adapter.parent().map_err(WalkError::Backend)?;
                continue;
            },
            Component::Name(name) => name,
        };
        let parent = adapter.push(name).map_err(WalkError::Backend)?;
        let needs_directory = adapter.requires_directory();
        if !needs_directory && final_component == FinalComponent::LeaveUninspected {
            continue;
        }
        let kind = match adapter.inspect() {
            Ok(kind) => kind,
            Err(error) if !needs_directory && allow_missing && A::is_not_found(&error) => {
                return Ok(());
            },
            Err(error) => return Err(WalkError::Backend(error)),
        };
        if kind == EntryKind::Symlink {
            links_left = links_left
                .checked_sub(1)
                .ok_or(WalkError::TooManySymlinks)?;
            // A final link's target must exist, even when its original name could have been absent.
            if !needs_directory {
                allow_missing = false;
            }
            adapter.expand_link(parent).map_err(WalkError::Backend)?;
        } else if needs_directory && kind != EntryKind::Directory {
            return Err(WalkError::NotDirectory);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
