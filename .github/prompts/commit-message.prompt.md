---
description: Generate a Nanvix-conformant commit message for the currently staged changes.
name: "Commit Message"
agent: "agent"
---

# Suggest a Commit Message

Generate a commit message for the **currently staged** changes only. Do not stage,
commit, amend, or push anything — only produce the message text for the user to use.

Consider **only** what is in the staging area (the index). Ignore unstaged and
working-tree changes entirely. If nothing is staged, say so and stop without
producing a message.

## Steps

1. Inspect staged changes with `git diff --cached --stat` first to see scope, then
   `git diff --cached` for detail. Do **not** paste the full diff back into the chat —
   read it, then write the message.
   - Work from the diff hunks alone. Do **not** open whole source files with
     `read_file` to gather context — the staged hunks are sufficient to describe
     what changed and why.
   - Only if a hunk is genuinely ambiguous, open **at most one or two** files, and
     read just the relevant range rather than the entire file.
2. Determine the single most appropriate `module-name` and type tag (below). If the
   change spans modules, pick the dominant one; do not invent multi-scope subjects.
3. Write the message following the format and body rules below.
4. Output the final message inside one fenced ```text block so it can be copied as-is.

## Subject Line

Format (enforced by `.githooks/commit-msg`):

```text
[module-name] (B|E|F|W): Short Description
```

- **module-name** — affected component. Valid names are Cargo workspace members plus:
  `build`, `ci`, `contrib`, `doc`, `git`, `scripts`, `tests`
  (e.g. `kernel`, `proc`, `sys`, `syscall`, `libc_time`, `tests`, `doc`).
- **Type tag** — `B` bug fix · `E` enhancement · `F` feature · `W` work in progress.
- **Title must be at most 50 characters** (including the `[module] X:` prefix).
- Use the imperative mood and Title Case for the description (e.g. `Add`, `Harden`,
  `Implement`), matching existing history.

## Body

- Separate the subject from the body with one blank line; wrap the body at ~72 columns.
- Open with a short imperative paragraph stating **what** changed and **why** /
  the observable effect — not a restatement of the diff.
- Group mechanical details under labelled bullet sections when the change touches
  several areas (mirror existing commits), for example:
  - `Kernel:` / `Libraries:` / `Tests:` / `Build and harness wiring:` — adjust to fit.
  - Each bullet is a concise, specific statement; sub-bullets are allowed.
- Reference issues/PRs when relevant using the project's style:
  `Part of #2690`, `closes #2693`. Only include refs you can justify from the diff or
  the user's explicit input — never fabricate issue numbers.
- Mention test/run commands only when they add real value (e.g. how to run new tests).

## Reference Examples

```text
[proc] E: Harden authorize_kill()

Rework ProcessDaemon::authorize_kill() to fail closed when process
identities are missing. Previously, an absent caller or target identity
fell through to `_ => Ok(())`, implicitly authorizing the signal — a
fail-open authorization gap. Each lookup now returns
ErrorCode::NoSuchProcess with a logged reason, and authorization only
succeeds when both identities exist and can_signal() permits it.

Add unit tests covering:
- root caller and same-user callers (allowed),
- different-user caller (PermissionDenied),
- unknown caller and unknown target (NoSuchProcess).
```

```text
[doc] E: Update build instructions
```

## Rules

- Read the diff; do not echo it back.
- Consider only staged changes; never describe unstaged or working-tree changes.
- If nothing is staged, report that and stop; do not produce a message.
- Base the message on the diff hunks; do not open whole source files for context.
- One subject line only, ≤ 50 chars, exact `[module] (B|E|F|W): ...` format.
- Do not run any mutating git command. Output the message and stop.
