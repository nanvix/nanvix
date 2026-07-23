# Contributing to Nanvix

Welcome to the Nanvix community! Whether you are fixing a bug, adding a feature, improving
documentation, or just asking questions — all contributions are valued. This guide covers the
essentials to help you get involved.

## Ways to Contribute

- **Report bugs** — Open an issue describing the problem, steps to reproduce, and expected behavior.
- **Suggest features** — Open an issue to discuss your idea before writing code.
- **Submit code** — Fix a bug, implement a feature, or improve existing code.
- **Improve documentation** — Clarify instructions, fix typos, or add missing guides.
- **Review pull requests** — Provide constructive feedback on open PRs.
- **Ask and answer questions** — Join the [Nanvix Slack](https://join.slack.com/t/nanvix/shared_invite/zt-1yu30bs28-nsNmw8IwCyh6MBBV~B~X7w)
  to discuss the project.

## Getting Started

See [doc/setup.md](doc/setup.md) for full environment setup. The quickest path:

```bash
git clone https://github.com/nanvix/nanvix.git && cd nanvix
./z setup
./z build -- all
```

On Windows, run `.\z.ps1 setup` to install the repository Git hooks from `.githooks`.

Further reading: [Building](doc/build.md) | [Running](doc/run.md) | [Testing](doc/test.md) |
[Benchmarking](doc/benchmark.md) | [Troubleshooting](doc/troubleshooting.md)

## Submitting Changes

### Guidelines

- Keep changes minimal, focused, and consistent with existing style (see [Coding Standards](#coding-standards)).
- Fix root causes — do not add superficial patches.
- Do not modify unrelated code paths or tests.
- Update documentation when behavior or interfaces change.
- Each commit must build successfully on its own.

### Workflow

1. Create a feature branch from `dev`.
2. Work on changes.
3. Commit changes following the [commit message convention](#commit-messages).
4. Run quality checks and tests locally before pushing.
5. Repack (squash/reorder) commits into logical, self-contained units before opening a PR.
6. Rebase onto the latest `origin/dev`.
7. Open a **draft** pull request against `dev`.
8. Once the PR is ready, mark it as ready for review and request a review.
9. After reviews have started, address comments as new commits on top of the original PR. Do not
   rewrite or force-push over reviewed commits unless a maintainer explicitly requests history
   cleanup.
10. If CI fails or new review comments arrive. While the PR is still in **draft**: fix the issues,
repack/rebase as needed, and update the branch with `git push --force-with-lease` before requesting
review again.  After the PR has received reviews: fix the issues as new commits on top, without
rewriting history, unless a maintainer explicitly asks you to rebase and force-push.

## Commit Messages

Commit messages must follow this format:

```plain
[module-name] (B|E|F|W): Short Description
```

- **module-name** — the affected component (e.g., `kernel`, `nvx`, `nanvixd`, `scripts`, `doc`).
  Valid names are derived from Cargo workspace members plus: `build`, `ci`, `contrib`, `doc`, `git`,
  `scripts`, `tests`.
- **Type tag:**
  - `B` — Bug fix.
  - `E` — Enhancement.
  - `F` — Feature.
  - `W` — Work in progress.
- Title must be at most **50 characters**.

Examples:

```plain
[kernel] F: Add page fault handler
[nanvixd] B: Fix socket cleanup on exit
[doc] E: Update build instructions
```

A git hook (`.githooks/commit-msg`) enforces this convention automatically.

## Coding Standards

### General

- Follow the existing style of the component you are editing.
- Use module/file-scope constants; avoid magic numbers.
- Prefer typed errors over ad-hoc strings.
- Respect the 100-column line width limit in source code.
- Avoid machine-specific paths and hardcoded environment assumptions.
- Public APIs must have doc comments. Prefer comments that explain *why*.
- `TODO`/`FIXME` comments must reference an issue (e.g., `TODO (#1234): ...`).

### Rust

- No `panic!`, `unwrap()`, or `expect()` in production code; return `Result<T, E>`.
- In tests, prefer `expect()` over `unwrap()` for better diagnostics.
- Minimize `unsafe`; document invariants with a `# Safety` section.
- Use explicit type annotations and prefix imports with `::`.
- Log errors before returning from error paths; keep logs single-line.

### C/C++

- Use proper header guards and include parameter names in declarations.
- Validate pointers and buffer bounds. No leaks, double-free, or use-after-free.

### Python

- Follow PEP 8. Use type hints and docstrings. Line limit: 100 columns.

### Shell

- For Bash scripts, prefer `#!/usr/bin/env bash` (or `#!/bin/bash` where needed); quote variables
and use `set -euo pipefail` where appropriate.

## Quality Checks

Run these before submitting a pull request:

```bash
./z build -- format-check    # Formatting.
./z build -- lint-check      # Linting.
./z build -- spellcheck      # Spelling.
./z build -- verify          # Formal verification (Verus).
./z build -- test            # Unit + system tests.
```

Or run the full CI pipeline locally:

```bash
./scripts/pipeline.sh
```

The CI pipeline runs automatically on every pull request and push to `dev`.

## License

Nanvix is licensed under the [MIT License](LICENSE.txt). By contributing, you agree that your
contributions will be licensed under the same terms.

## Need Help?

- Browse [existing issues](https://github.com/nanvix/nanvix/issues) to see if your question has
  already been answered.
- Join the [Nanvix Slack](https://join.slack.com/t/nanvix/shared_invite/zt-1yu30bs28-nsNmw8IwCyh6MBBV~B~X7w)
  to chat with the community.
- Check [doc/troubleshooting.md](doc/troubleshooting.md) for common problems.
