# Copilot Instructions for Nanvix

This file contains only cross-cutting instructions that apply to most tasks.
Task-specific and procedural details must live in `.github/skills/*/SKILL.md`.

## Scope and Priority

- Follow this file for global behavior.
- For task-specific work, load and follow the matching skill document.
- If there is overlap, keep this file generic and keep procedures in skills.

## Skill Routing

Use the corresponding skill when the user request matches the topic:

- `benchmarking` → benchmark setup, execution, and result interpretation.
- `build` → build, format, lint, and spell-check commands.
- `test` → running unit tests, integration tests, and the full test suite.
- `ci-cd-pipeline` → CI workflow behavior, failures, and release flow.
- `coding-standards` → detailed style, documentation, and review conventions.
- `daemon-development` → daemon architecture, implementation, and debugging.
- `gdb-debugging` → GDB remote debugging of Nanvix guests via the microvm GDB server.
- `kernel-development` → kernel internals and kernel-call paths.
- `library-development` → crates under `src/libs` and related architecture.
- `test-development` → writing and debugging tests.
- `troubleshooting` → diagnosis of build/runtime/test failures (includes Windows-specific issues).
- `user-app-development` → user-space app implementation and execution (includes Windows standalone mode).

## Platform Support

Nanvix supports two host platforms:

- **Linux** — Full development workflow: build, run, test, benchmark, and debug.
  Uses KVM for the microvm backend.
- **Windows 11** — Host-side development with the Windows Hypervisor Platform (WHP)
  backend. Guest components (kernel, user binaries) are cross-compiled using a local
  toolchain; the UserVM is built natively. Only standalone interactive mode is available
  (no HTTP mode). Use `z.ps1` instead of `./z`.

When modifying or generating code, keep platform differences in mind:

- Use `z.ps1` for Windows commands and `./z` for Linux commands.
- Linux-only features: HTTP mode, GDB debugging.
- Windows-supported features: standalone interactive mode, benchmarking
  (`boot-time`, `cold-start`, `warm-start-vmm`).
- Windows-only concerns: WHP enablement, Developer Mode for symlinks, GNU Make
  on PATH.

## Repository-wide Engineering Rules

- Keep changes minimal, focused, and consistent with existing style.
- Fix root causes rather than adding superficial patches.
- Do not modify unrelated code paths or tests.
- Update documentation when behavior or interfaces change.
- Prefer existing project tooling and scripts over ad-hoc commands.
- Never stage or commit changes unless the user explicitly asks. Leave edits in the working
  tree for review.
- When the user does ask to commit, follow the commit-message convention in
  [CONTRIBUTING.md](../CONTRIBUTING.md) (`[module] (B|E|F|W): Subject`, title at most 50
  characters) and never bypass git hooks (no `--no-verify`).

## Validation

- Validate changes with the smallest relevant check first.
- Expand to broader checks only as needed for confidence.
- Report any unrelated failures without attempting broad refactors.

## Language and Safety Baseline

- Respect language-specific style and lint rules already enforced by the repository.
- Avoid unsafe patterns unless strictly necessary and justified.
- Ensure robust error handling and clear diagnostics.
- Do not introduce machine-specific paths or hardcoded environment assumptions.

## Notes for Maintaining This File

- Keep this document short and generic.
- Move detailed command lists, workflows, and checklists to skill files.
- Add new topics as skills rather than expanding this file with procedures.
