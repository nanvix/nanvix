---
name: coding-standards
description: Detailed Nanvix coding, documentation, and review standards across Rust, C/C++, Python, and Shell. Use this when asked to implement or review code quality conventions.
---

# Coding Standards and Review Guidelines

Use this skill when the user asks about style rules, documentation requirements, code review checks,
or language-specific conventions.

## General Standards

- Always follow the existing code style in the touched component.
- Keep changes minimal and focused.
- Use module/file-scope constants; avoid magic numbers in logic.
- Prefer typed errors and consistent OS error mapping over ad-hoc strings.
- Keep diagnostics concise and informative.
- Respect the repository line width limit (100 columns).
- Add/update docs when behavior or interfaces change.
- Avoid machine-specific paths and hardcoded environment assumptions.

## Documentation and Comments

- Public modules, types, functions, constants, and variables should have doc comments.
- Rust doc comments use `///`.
- For APIs, include sections as applicable: `# Description`, `# Parameters`, `# Returns`, `# Errors`.
- For unsafe APIs, include a `# Safety` section describing invariants.
- `TODO` and `FIXME` comments should reference an issue (for example: `TODO (#1234): ...`).
- In source code, restrict issue/PR/task references to `TODO`/`FIXME` comments only (e.g., `TODO (#1234): ...`).
- Avoid issue numbers in other code/doc comments and in commit messages.
- Prefer comments that explain *why*.
- End comments with a period.

## Rust-Specific Rules

- Avoid `panic!`, `unwrap()`, and `expect()` in production code; return `Result<T, E>`.
- In `#[cfg(test)]`, `expect()` is preferred over `unwrap()` for diagnostics.
- Prefer `#![deny(...)]` over `#![forbid(...)]` for unwrap/expect lints to allow scoped test
  exceptions.
- Minimize `unsafe`; keep scope narrow and document pre/post conditions.
- Use explicit type annotations for local variables and constants.
- Prefix imports with `::`.
- Log errors before returning from error paths.
- Keep logs single-line and machine-friendly; avoid multiline and pretty debug formatting.
- For C interop, prefer C ABI types (`c_size_t`, `c_ssize_t`, `c_int`, `c_uint`, `c_long`,
  `c_ulong`, `c_short`, `c_ushort`) over Rust primitive aliases.
- Keep struct fields private and expose accessors when API-facing.

## C/C++-Specific Rules

- Follow existing formatting and indentation patterns.
- Use proper header guards in headers.
- Use Linux-kernel-style C conventions where applicable.
- Include parameter names in function declarations in headers.
- Validate pointers before dereferencing and check bounds on buffers.
- Ensure robust memory management (no leaks, double-free, use-after-free).

## Python-Specific Rules

- Follow PEP 8.
- Use type hints for function signatures.
- Keep line length at or below 100 characters.
- Add docstrings for public functions and classes.
- Use context managers for resource management.

## Shell-Specific Rules

- Use `#!/bin/bash` shebang.
- Quote variable expansions unless word-splitting is intentional.
- Use `set -e` where appropriate.
- Keep scripts small and focused.
- Add comments for non-obvious operations.

## Review Checklist

- Ensure style and lint rules are satisfied.
- Verify docs/comments were updated where behavior changed.
- Check for arithmetic overflow risks.
- Check for resource leaks and deadlocks.
- Verify comprehensive error handling and error logging.
- Ensure relevant tests are added/updated.
- Verify no hardcoded machine-specific behavior was introduced.
