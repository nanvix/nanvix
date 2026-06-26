---
description: "Address review comments left on a PR without committing the fixes"
name: "Address PR Comments"
argument-hint: "PR number (e.g. 2438)"
agent: "agent"
---
Address the review comments left on the pull request identified in the input.

- Fetch the PR's review comments and unresolved threads using GitHub tooling (e.g. `gh pr view` / `gh api`).
  Focus on unresolved threads first.
- For each comment, make the corresponding code edit and briefly note how it was addressed.
- When verification is needed, run the relevant tests per the
  [test skill](../skills/test/SKILL.md), piping large output through `tail`/`grep`.
- Do **not** commit the fixes. Leave the changes in the working tree for review.
