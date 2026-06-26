---
description: "Review current changes, address findings, and verify tests pass without committing"
name: "Review Changes"
argument-hint: "Optional: issue number this change should fix (e.g. 2674)"
agent: "agent"
---
Review the current changes (staged and unstaged) in this repository.

- Inspect the diff with `git --no-pager diff` and `git --no-pager diff --staged`. Prefer
  `--stat` first, then drill into specific files — do not dump full diffs into context.
- Report findings, issues, and observations. Address them with code edits.
- If an issue number was provided in the input, confirm the change actually fixes it.
- Ensure unit, in-kernel, and integration tests pass. Follow the
  [test skill](../skills/test/SKILL.md) for the exact `./z` commands. When viewing test
  output, pipe through `tail`/`grep` so large logs do not flood the conversation.
- Do **not** commit nor stage any fixes. Leave changes in the working tree for review.
