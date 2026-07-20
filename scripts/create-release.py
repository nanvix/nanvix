#!/usr/bin/env python3

# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""
Utility for creating a release of Nanvix.

Run 'python create-release.py --help' for usage information.
"""

# ======================================================================
# Imports
# ======================================================================

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys

# ======================================================================
# Constants
# ======================================================================

_RED = "\033[31m"
_GREEN = "\033[32m"
_CYAN = "\033[36m"
_RESET = "\033[0m"

# ======================================================================
# Logging
# ======================================================================


def _supports_color() -> bool:
    """Check if the terminal supports ANSI color codes."""
    if os.environ.get("NO_COLOR"):
        return False
    return hasattr(sys.stdout, "isatty") and sys.stdout.isatty()


_COLOR = _supports_color()


def _c(code: str, text: str) -> str:
    return f"{code}{text}{_RESET}" if _COLOR else text


def print_error(msg: str) -> None:
    """Print error message to stderr."""
    print(f"{_c(_RED, '[ERROR]')} {msg}", file=sys.stderr)


def print_success(msg: str) -> None:
    """Print success message to stdout."""
    print(f"{_c(_GREEN, '[OK]')}    {msg}")


def print_info(msg: str) -> None:
    """Print info message to stdout."""
    print(f"{_c(_CYAN, '[INFO]')}  {msg}")


# ======================================================================
# Version Helpers
# ======================================================================

# Matches the `version = "X.Y.Z"` line inside the [workspace.package] section.
_VERSION_RE = re.compile(
    r"(\[workspace\.package\]\s*\n(?:(?!\n\[)[^\n]*\n)*?)"
    r'(version\s*=\s*")(\d+\.\d+\.\d+)(")',
    re.MULTILINE,
)


def get_cargo_toml_version(cargo_toml: str) -> str:
    """Read the version string from a Cargo.toml file."""
    if not os.path.isfile(cargo_toml):
        print_error(f"{cargo_toml} does not exist.")
        sys.exit(1)
    with open(cargo_toml, encoding="utf-8") as fh:
        content = fh.read()
    match = _VERSION_RE.search(content)
    if not match:
        print_error(f"Could not extract version from {cargo_toml}.")
        sys.exit(1)
    return match.group(3)


def increment_version(version: str, bump: str) -> str:
    """Increment a version component of a MAJOR.MINOR.PATCH version string.

    Args:
        version: Current version string in MAJOR.MINOR.PATCH format.
        bump: One of 'patch', 'minor', or 'major'.
    """
    parts = version.split(".")
    if len(parts) != 3 or not all(p.isdigit() for p in parts):
        print_error(f"Invalid version format '{version}'")
        sys.exit(1)
    major, minor, patch = int(parts[0]), int(parts[1]), int(parts[2])
    if bump == "major":
        major += 1
        minor = 0
        patch = 0
    elif bump == "minor":
        minor += 1
        patch = 0
    else:
        patch += 1
    return f"{major}.{minor}.{patch}"


def update_cargo_toml(cargo_toml: str, new_version: str) -> None:
    """Update the version field in a Cargo.toml file."""
    with open(cargo_toml, encoding="utf-8") as fh:
        content = fh.read()

    new_content, count = _VERSION_RE.subn(
        rf"\g<1>\g<2>{new_version}\g<4>", content, count=1
    )
    if count == 0:
        print_error("Failed to update version in Cargo.toml")
        sys.exit(1)

    with open(cargo_toml, "w", encoding="utf-8", newline="\n") as fh:
        fh.write(new_content)

    # Verify the write.
    if get_cargo_toml_version(cargo_toml) != new_version:
        # Restore original content.
        with open(cargo_toml, "w", encoding="utf-8", newline="\n") as fh:
            fh.write(content)
        print_error("Failed to update version in Cargo.toml")
        sys.exit(1)

    print_success(f"Updated Cargo.toml with version: {new_version}")


# ======================================================================
# Cargo Helpers
# ======================================================================


def update_cargo_lock(repo_root: str) -> None:
    """Regenerate Cargo.lock to reflect changes in Cargo.toml."""
    print_info("Updating workspace crate versions in Cargo.lock...")
    result = subprocess.run(
        [
            "cargo",
            "update",
            "--workspace",
            "--manifest-path",
            os.path.join(repo_root, "Cargo.toml"),
        ],
        check=False,
    )
    if result.returncode != 0:
        print_error("Failed to update Cargo.lock")
        sys.exit(1)
    print_success("Updated Cargo.lock")


# ======================================================================
# Git Helpers
# ======================================================================


def _git(*args: str, capture: bool = False) -> subprocess.CompletedProcess[str]:
    """Run a git command and return the CompletedProcess."""
    return subprocess.run(
        ["git", *args],
        capture_output=capture,
        text=True,
        check=False,
    )


def git_is_configured() -> bool:
    """Check if git user.name and user.email are configured."""
    name = _git("config", "--get", "user.name", capture=True)
    email = _git("config", "--get", "user.email", capture=True)
    return bool(name.stdout.strip() and email.stdout.strip())


def is_merge_commit(commit: str) -> bool:
    """Check if the given commit is a merge commit."""
    result = _git("rev-parse", "-q", "--verify", f"{commit}^2", capture=True)
    return result.returncode == 0


def get_default_branch() -> str:
    """Get the default branch name from the remote 'origin'."""
    # Verify origin exists.
    remotes = _git("remote", capture=True)
    if "origin" not in remotes.stdout.split():
        print_error("Remote 'origin' does not exist.")
        sys.exit(1)

    result = _git("remote", "show", "origin", capture=True)
    if result.returncode != 0:
        print_error(f"Failed to query remote 'origin': {result.stderr.strip()}")
        sys.exit(1)
    for line in result.stdout.splitlines():
        if "HEAD branch" in line:
            return line.split()[-1]

    print_error("Could not determine default branch from remote 'origin'.")
    sys.exit(1)


def is_default_branch() -> bool:
    """Check if the current branch is the default branch."""
    current = _git("rev-parse", "--abbrev-ref", "HEAD", capture=True).stdout.strip()
    print_info(f"Current branch: {current}")
    default = get_default_branch()
    print_info(f"Default branch: {default}")
    return current == default


def git_commit(
    cargo_toml: str, cargo_lock: str, new_version: str, skip_ci: bool
) -> None:
    """Commit the version bump if there are changes."""
    toml_changed = _git("diff", "--quiet", cargo_toml).returncode != 0
    lock_changed = _git("diff", "--quiet", cargo_lock).returncode != 0

    if toml_changed or lock_changed:
        print_info("Committing new version...")
        result = _git("add", cargo_toml, cargo_lock)
        if result.returncode != 0:
            print_error("Failed to stage files for commit.")
            sys.exit(1)
        commit_message = f"Nanvix {new_version}"
        if skip_ci:
            commit_message += " [skip ci]"
        result = _git("commit", "--no-verify", "-m", commit_message)
        if result.returncode != 0:
            print_error("Failed to commit version bump.")
            sys.exit(1)
        print_success(f"Successfully created release: {new_version}")
    else:
        print_info("No changes to commit")


def git_push() -> None:
    """Push committed changes to the remote repository on the current branch."""
    print_info("Pushing changes to remote...")
    if not git_is_configured():
        print_error("Git is not configured. Please set user.name and user.email.")
        sys.exit(1)

    current_branch = _git(
        "rev-parse", "--abbrev-ref", "HEAD", capture=True
    ).stdout.strip()
    print_info(f"Pushing to branch: {current_branch}")
    result = _git("push", "--no-verify", "-u", "origin", current_branch)
    if result.returncode != 0:
        print_error("Failed to push changes to remote.")
        sys.exit(1)


# ======================================================================
# Main
# ======================================================================


def parse_args() -> argparse.Namespace:
    """Parse command-line arguments."""
    parser = argparse.ArgumentParser(
        description="Utility for creating a release of Nanvix.",
    )
    bump_group = parser.add_mutually_exclusive_group(required=True)
    bump_group.add_argument(
        "--patch",
        action="store_const",
        const="patch",
        dest="bump",
        help="Bump patch version: X.Y.Z -> X.Y.(Z+1)",
    )
    bump_group.add_argument(
        "--minor",
        action="store_const",
        const="minor",
        dest="bump",
        help="Bump minor version: X.Y.Z -> X.(Y+1).0",
    )
    bump_group.add_argument(
        "--major",
        action="store_const",
        const="major",
        dest="bump",
        help="Bump major version: X.Y.Z -> (X+1).0.0",
    )
    parser.add_argument(
        "--push",
        action="store_true",
        help="Push the release commit to the remote origin.",
    )
    parser.add_argument(
        "--skip-merge-check",
        action="store_true",
        help="Skip the merge commit requirement (for manual releases).",
    )
    parser.add_argument(
        "--skip-ci",
        action="store_true",
        help="Add [skip ci] to the release commit message.",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()

    # Must be on the default branch.
    if not is_default_branch():
        print_error("This script must be run on the default branch.")
        sys.exit(1)

    # HEAD must be a merge commit (unless skipped for manual releases).
    if not args.skip_merge_check:
        head = _git("rev-parse", "HEAD", capture=True).stdout.strip()
        if not is_merge_commit(head):
            print_error("The HEAD commit is not a merge commit.")
            sys.exit(1)

    repo_root = _git("rev-parse", "--show-toplevel", capture=True).stdout.strip()
    cargo_toml = os.path.join(repo_root, "Cargo.toml")
    cargo_lock = os.path.join(repo_root, "Cargo.lock")

    print_info(f"Creating {args.bump} release...")

    current_version = get_cargo_toml_version(cargo_toml)
    print_info(f"Current version: {current_version}")

    new_version = increment_version(current_version, args.bump)
    print_info(f"Incrementing version: {current_version} -> {new_version}")

    update_cargo_toml(cargo_toml, new_version)
    update_cargo_lock(repo_root)
    git_commit(cargo_toml, cargo_lock, new_version, args.skip_ci)

    if args.push:
        git_push()
    else:
        print_info("Changes committed locally.")


if __name__ == "__main__":
    main()
