# Copyright(c) The Maintainers of Nanvix.
# Licensed under the MIT License.

"""Path classification shared by the test-layout checker and the PR flagger."""

from __future__ import annotations

import os
import posixpath
import re
import subprocess
from pathlib import Path

# Repository-relative location of the allowlist.
ALLOWLIST_FILE = "scripts/test-layout-allowlist.txt"

TEST_DIR_NAMES = frozenset({"tests", "test"})

C_EXTENSIONS = (".c", ".cc", ".cpp", ".cxx", ".h", ".hh", ".hpp", ".hxx")

# Directories never scanned when the file list is built without git.
_SKIPPED_DIRS = frozenset(
    {
        ".git",
        ".venv",
        "target",
        "toolchain",
        "node_modules",
        "__pycache__",
        "sysroot-debug",
    }
)


def to_posix(path: str) -> str:
    """Normalizes a repository-relative path to forward slashes."""
    return path.replace("\\", "/")


def _stem(name: str) -> str:
    return name.rsplit(".", 1)[0] if "." in name else name


def is_test_stem(stem: str) -> bool:
    """Tells whether a file stem names a test file (`tests`, `foo_test`, `test_foo`, ...)."""
    return (
        stem in ("test", "tests")
        or stem.endswith(("_test", "_tests"))
        or stem.startswith(("test_", "test-"))
    )


def has_test_component(path: str) -> bool:
    """Tells whether a directory component of `path` is named `test` or `tests`."""
    return any(part in TEST_DIR_NAMES for part in to_posix(path).split("/")[:-1])


def is_rust_test_file(path: str) -> bool:
    """Tells whether a `.rs` file is a test file (it may contain test code)."""
    path = to_posix(path)
    if path.startswith("src/tests/") or has_test_component(path):
        return True
    stem = _stem(posixpath.basename(path))
    return (
        stem in ("test", "tests")
        or stem.endswith(("_test", "_tests"))
        or stem.startswith("test_")
    )


def is_python_test_file(path: str) -> bool:
    """Tells whether a `.py` file is a test file."""
    path = to_posix(path)
    name = posixpath.basename(path)
    return (
        path.startswith("tests/") or name == "conftest.py" or is_test_stem(_stem(name))
    )


def is_c_test_file(path: str) -> bool:
    """Tells whether a C/C++ file is a test file."""
    path = to_posix(path)
    return (
        path.startswith("src/tests/")
        or has_test_component(path)
        or is_test_stem(_stem(posixpath.basename(path)))
    )


def is_e2e_manifest(path: str) -> bool:
    """Tells whether a path is an end-to-end test manifest under `test/`."""
    path = to_posix(path)
    return path.startswith("test/") and path.endswith(".toml")


def looks_like_e2e_manifest(path: str) -> bool:
    """Tells whether a TOML file is named like an end-to-end test manifest."""
    name = posixpath.basename(to_posix(path))
    return name.endswith(".toml") and name != "Cargo.toml" and is_test_stem(_stem(name))


_PYTHON_TEST_CONTENT = re.compile(
    r"^(?:class\s+\w+\([^)]*\bTestCase\b[^)]*\)|def\s+test_\w+\("
    r"|import\s+pytest\b|from\s+pytest\b)",
    re.MULTILINE,
)


def python_looks_like_test(text: str) -> bool:
    """Tells whether Python source defines unit tests."""
    return _PYTHON_TEST_CONTENT.search(text) is not None


def list_repo_files(root: Path) -> list[str]:
    """Lists repository files (tracked, plus untracked and not ignored) relative to `root`."""
    try:
        out = subprocess.run(
            ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
            cwd=root,
            check=True,
            capture_output=True,
        ).stdout.decode("utf-8")
        names = [n for n in out.split("\0") if n]
    except (OSError, subprocess.CalledProcessError):
        names = []
        for current, dirs, files in os.walk(root):
            dirs[:] = [d for d in dirs if d not in _SKIPPED_DIRS]
            for name in files:
                names.append(to_posix(str(Path(current, name).relative_to(root))))
    return sorted({to_posix(n) for n in names if (root / n).is_file()})


_MANIFEST_PATH = re.compile(r'^\s*path\s*=\s*"([^"]+\.rs)"', re.MULTILINE)


def crate_roots(root: Path, files: list[str]) -> set[str]:
    """Returns the Rust files that are crate roots (their child modules live next to them)."""
    known = set(files)
    roots: set[str] = set()
    for manifest in files:
        if posixpath.basename(manifest) != "Cargo.toml":
            continue
        base = posixpath.dirname(manifest)
        for rel in ("src/lib.rs", "src/main.rs", "build.rs"):
            candidate = posixpath.normpath(posixpath.join(base, rel))
            if candidate in known:
                roots.add(candidate)
        prefix = posixpath.normpath(posixpath.join(base, "src/bin")) + "/"
        roots.update(f for f in files if f.startswith(prefix) and f.endswith(".rs"))
        try:
            text = (root / manifest).read_text(encoding="utf-8")
        except OSError:
            continue
        for rel in _MANIFEST_PATH.findall(text):
            roots.add(posixpath.normpath(posixpath.join(base, rel)))
    return roots


def module_directory(file: str, roots: set[str]) -> str:
    """Returns the directory in which child modules of `file` are declared."""
    file = to_posix(file)
    name = posixpath.basename(file)
    parent = posixpath.dirname(file)
    if name in ("lib.rs", "main.rs", "mod.rs") or file in roots:
        return parent
    return posixpath.join(parent, _stem(name))


def module_candidates(
    file: str,
    name: str,
    roots: set[str],
    path_attr: str | None = None,
    inline_chain: tuple[str, ...] = (),
) -> list[str]:
    """Returns the files the compiler would try for `mod name;` declared in `file`."""
    file = to_posix(file)
    if path_attr is not None:
        base = posixpath.dirname(file)
        if inline_chain:
            base = posixpath.join(base, *inline_chain)
        return [posixpath.normpath(posixpath.join(base, path_attr))]
    base = module_directory(file, roots)
    if inline_chain:
        base = posixpath.join(base, *inline_chain)
    return [posixpath.join(base, f"{name}.rs"), posixpath.join(base, name, "mod.rs")]
