---
description: "Push the current branch and open a draft pull request against the default branch"
name: "Create Draft PR"
argument-hint: "Optional: issue number this PR addresses (e.g. 2698)"
agent: "agent"
---
Open a **draft** pull request for the current branch.

- Confirm the working state first: run `git status -sb` and `git --no-pager log
  origin/dev..HEAD --oneline` to see the branch and the commits that will ship. Never
  create a PR from `dev`; if the current branch is the default branch, stop and report.
- Do **not** stage, commit, or amend anything. Only push and open the PR. If there are
  uncommitted changes, report them and stop — let the user commit first.
- Push the current branch to `origin` with upstream tracking
  (`git push -u origin HEAD`).
- Derive the PR title from the commit history (use the subject when there is a single
  commit). Keep it consistent with the project's commit-message convention.
- Write a concise body summarizing **what** changed and **why** based on the commits and
  diff (`git --no-pager diff origin/dev...HEAD`); do not dump the full diff. If an issue
  number was provided in the input, reference it (e.g. `Part of #2698`, `closes #2698`)
  only when justified.
- Open the PR as a draft against the default branch `dev`, for example:
  `gh pr create --draft --base dev --title "..." --body "..."`. Prefer the GitHub PR
  tools when available.
- Report the resulting PR URL. Do not mark it ready for review.
