# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Read-only git access for the PR flagger.

The flagger runs on untrusted pull requests, so it only ever reads blobs and trees: it never checks
out, builds or executes anything from the pull request.
"""

from __future__ import annotations

import subprocess
from dataclasses import dataclass
from pathlib import Path
from types import TracebackType

# Rename detection gives up beyond this many candidate files; matching by content still works.
_RENAME_LIMIT = "4000"


class GitError(Exception):
    """Raised when a git command fails."""


@dataclass(frozen=True)
class Change:
    """One file changed by a pull request."""

    # `A` (added), `D` (deleted), `M` (modified) or `R` (renamed, possibly modified).
    status: str
    # Path at the head revision (at the base revision for deleted files).
    path: str
    # Path at the base revision (equal to `path` unless the file was renamed).
    old_path: str
    similarity: int = 100


def _git(root: Path, *args: str) -> bytes:
    try:
        proc = subprocess.run(
            ["git", *args], cwd=root, capture_output=True, check=False
        )
    except OSError as err:
        raise GitError(f"cannot run git: {err}") from err
    if proc.returncode != 0:
        message = proc.stderr.decode("utf-8", "replace").strip()
        raise GitError(f"git {' '.join(args)} failed: {message}")
    return proc.stdout


def resolve(root: Path, rev: str) -> str:
    """Resolves a revision to a commit hash."""
    out = _git(root, "rev-parse", "--verify", "--end-of-options", f"{rev}^{{commit}}")
    return out.decode("ascii").strip()


def merge_base(root: Path, base: str, head: str) -> str:
    """Returns the best common ancestor of two commits."""
    try:
        out = _git(root, "merge-base", base, head)
    except GitError as err:
        raise GitError(
            f"{err}. Fetch enough history for both revisions to share an ancestor."
        ) from err
    return out.decode("ascii").strip()


def changed_files(root: Path, base: str, head: str) -> list[Change]:
    """Lists the files that differ between two commits, detecting renames."""
    out = _git(
        root,
        "diff",
        "--name-status",
        "-z",
        "-M",
        f"-l{_RENAME_LIMIT}",
        "--no-ext-diff",
        base,
        head,
    )
    fields = out.decode("utf-8", "surrogateescape").split("\0")
    changes: list[Change] = []
    i = 0
    while i < len(fields) and fields[i]:
        status = fields[i]
        letter = status[0]
        if letter in ("R", "C"):
            old, new = fields[i + 1], fields[i + 2]
            i += 3
            if letter == "R":
                similarity = int(status[1:] or 100)
                changes.append(Change("R", new, old, similarity))
            else:
                changes.append(Change("A", new, new))
            continue
        path = fields[i + 1]
        i += 2
        if letter in ("A", "D"):
            changes.append(Change(letter, path, path))
        else:
            changes.append(Change("M", path, path))
    return changes


class BlobReader:
    """Reads file contents at given revisions through one long-lived `git cat-file` process."""

    def __init__(self, root: Path) -> None:
        try:
            self._proc = subprocess.Popen(
                ["git", "cat-file", "--batch"],
                cwd=root,
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
            )
        except OSError as err:
            raise GitError(f"cannot run git: {err}") from err

    def __enter__(self) -> BlobReader:
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        tb: TracebackType | None,
    ) -> None:
        self.close()

    def close(self) -> None:
        """Stops the underlying process."""
        if self._proc.stdin is not None:
            self._proc.stdin.close()
        if self._proc.stdout is not None:
            self._proc.stdout.close()
        self._proc.wait()

    def read(self, rev: str, path: str) -> bytes | None:
        """Returns the contents of `path` at `rev`, or `None` when it does not exist there."""
        if "\n" in path:
            return None
        stdin, stdout = self._proc.stdin, self._proc.stdout
        assert stdin is not None and stdout is not None
        stdin.write(f"{rev}:{path}\n".encode("utf-8", "surrogateescape"))
        stdin.flush()
        header = stdout.readline().split()
        if len(header) != 3:
            if header and header[-1] in (b"missing", b"ambiguous"):
                return None
            raise GitError(f"unexpected `git cat-file` reply for {rev}:{path}")
        size = int(header[2])
        data = stdout.read(size)
        stdout.read(1)
        return data if header[1] == b"blob" else None
