---
name: fix-issue
description: Guide for fixing a GitHub issue as a stack of small, reviewed, CI-green pull requests. Use this when asked to fix an issue and open PRs.
---

# Fix an Issue and Deliver It

Use this skill when asked to fix a GitHub issue and deliver the result as pull requests. Commands
use `./z` on Linux and `.\z.ps1` on Windows.

## Rules

- Do not stage or commit unless the request asks for commits or PRs. If only a fix is requested,
  leave changes uncommitted.
- Never merge, approve your own PR, push to `dev`, bypass hooks, or weaken tests.
- Each PR must change at most 1000 lines (additions plus deletions). Keep stack dependencies
  downward; the final PR closes the issue.
- Rewrite history (fold, amend, or rebase) only with an explicit request from a maintainer, even on
  drafts; this applies to every affected layer, including descendants rebased in steps 5 and 6.
  Without one, use follow-up commits. If a rebase is needed, stop and request maintainer
  authorization; report CI as blocked until it is granted. Do not merge the base as a workaround.
  Confirm the requester is a maintainer; if unsure, ask.

## 1. Understand and reproduce

```bash
gh issue view <N> --comments
gh pr list --state all --search "#<N> in:body"
```

Extract acceptance criteria and check for existing work. Load the relevant repository skills.
Reproduce the bug before changing code; for a feature or performance request, capture the current
behavior or baseline. Record commands and results for the PR.

## 2. Plan and implement

Run `git fetch origin`, then `git switch -c <issue>/<slug> --no-track origin/dev` (or
`git worktree add --no-track -b <issue>/<slug> <dir> origin/dev`). Run `./z setup` (installs
hooks) and `gh extension install github/gh-stack`, and confirm `git config commit.gpgsign` prints
`true`.

Split work by dependency and review value, not by file type. A small, cohesive fix should be one PR.
Commit each layer (step 3) before branching the next from it; keep layers buildable and tested
alone. Implement the root cause; add focused tests and update related docs and generated headers in
the same layer.

Validate each layer at its own tip:

```bash
./z build -- all
./z build -- format-check lint-check spellcheck test-layout-check
./z build -- test
./z build -- verify
```

If Verus cannot be installed, report that and rely on CI's `Verify` job. Re-run the reproduction
and retain the result.

## 3. Commit

Commit each validated unit with a signed, detailed message. Titles follow
`[module] X: Subject` (at most 50 characters; `X` is `B`, `E`, `F`, or `W`). Explain the problem,
approach, compatibility, issue/stack relationship, tests, and docs in the body; see
`.github/prompts/commit-message.prompt.md`. Add the Copilot co-author trailer when applicable.
Leave no `fixup!`, `squash!`, or `wip` commits.

The pre-commit hooks check the working tree, not the index. Commit each split unit with only
its own changes present. Do not disable signing or use `git replay` (it drops signatures).

## 4. Publish

For a stack, initialize the branches bottom-up and submit draft PRs:

```bash
gh stack init <layer-1> <layer-2>
gh stack submit --auto
gh stack view --short
```

For one PR, run `git push -u origin HEAD` and `gh pr create --draft --base dev --head <branch> ...`,
and replace later `gh stack` commands with `git fetch origin`, `git rebase -S origin/dev`, and
`git push --force-with-lease`. Give each PR a commit-style title and a body with summary, changes,
reviewer notes, and validation; only the last stack PR should say `Closes #<N>`. Never hand-chain
PRs with `--base <branch>`: `ci.yml` runs only for PRs targeting `dev` or in native stacks. Avoid
interactive `gh stack` commands; pass branch names and `--auto`.

## 5. Resolve reviews and update the stack

Copilot reviews each push, drafts included, within about five minutes (pure rebases may be
skipped); never request it. List unresolved threads, outdated ones included, with the paginated
GraphQL `reviewThreads` connection and read every comment. Fix valid comments in the layer that
owns the code; explain invalid comments with evidence. Leave threads needing human judgment open.
Repeat until the latest review of the current head leaves none unresolved.

To fold (needs the maintainer request from Rules) without the interactive `git rebase -i`, make the fix on the layer tip, then
run:

```bash
git stash -u
git switch --detach <owner-sha>
git stash pop
git add <paths>
git commit --amend --no-edit
git rebase --onto HEAD <owner-sha> <layer>
gh stack rebase --upstack
gh stack view --short
gh stack push
```

Revalidate affected layers after rebasing. Reply (`addPullRequestReviewThreadReply`) and resolve
(`resolveReviewThread`) only after the final push, so replies cite final SHAs.

## 6. Reach green CI

Rebase onto current `origin/dev`, cascade the stack, and push before marking PRs ready, but only
with the Rules' maintainer authorization for every affected layer. If a rebase is needed and that
authorization is missing, stop and request it; report CI as blocked until it is granted. Do not
merge `origin/dev` as a workaround: branch-specific merge commits fail the up-to-date gate and skip
source validation.

```bash
gh stack rebase
gh stack push
gh pr ready <pr>
gh pr checks <pr> --watch --fail-fast
```

CI skips important jobs on drafts, branches behind their base, or branches with merge commits; a
passing `Required status check` alone is not enough. Docs-only PRs (Markdown or `doc/`, except the
root `README.md`) skip build, test, and benchmark jobs by design; source checks, lint, and Verify
still run. Benchmark jobs also skip for fork PRs or unavailable self-hosted runners (see
`Benchmark Gate` logs); release jobs always skip on PRs. Other skips mean the PR was not validated.
Fix real failures as in step 5 (`gh run view <run-id> --log-failed`); rerun flakes with
`gh run rerun <run-id> --failed`.

Before reporting, confirm each PR is ready, within the line budget, has signed commits, green CI,
and no unresolved review threads. For signatures, check:

```bash
gh api --paginate 'repos/{owner}/{repo}/pulls/<pr>/commits' --jq '.[] | [.sha[0:9], .commit.verification.verified, .commit.verification.reason] | @tsv'
```

Each commit must report `true valid`.

## Report

List PRs in stack order, their sizes and purpose, validation results, review resolutions, CI status,
assumptions, and remaining human actions (Maintainers approval after the last push, merge queue).
Delete temporary message, body, and reply files.

## Pitfalls

- `git commit --fixup` may exceed the hook's title limit; use the step 5 recipe.
- After rebasing a new workspace crate across a release bump, refresh stale `Cargo.lock` entries.
- Direct Cargo commands for crates depending on `config` may require
  `MEMORY_SIZE_BYTES=268435456`; prefer `./z build -- <target>`.

Related skills: `build`, `test`, `test-development`, `troubleshooting`, `gdb-debugging`,
`benchmarking`, `coding-standards`, `ci-cd-pipeline`, and relevant module skills.
