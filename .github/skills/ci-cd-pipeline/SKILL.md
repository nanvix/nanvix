---
name: ci-cd-pipeline
description: Guide for Nanvix CI and GitHub Actions workflow behavior, including local pipeline execution and matrix coverage. Use this when asked about CI checks, workflow failures, or release flow.
---

# CI/CD Pipeline

Use this skill when the user asks about the continuous integration and deployment pipeline, GitHub
Actions workflows, or automated quality checks.

## Running the Full Pipeline Locally

```bash
./scripts/pipeline.sh
```

The pipeline runs all quality checks and tests across the supported configuration matrix.

## Pipeline Steps

The pipeline runs these steps in order:

1. **Spell Check** — Checks spelling in source code.
2. **Format Check** — Verifies code formatting.
3. **Lint Check** — Runs Clippy and other linters.
4. **Verify** — Runs Verus formal verification on annotated crates.
5. **Build** — Compiles all targets.
6. **Test** — Runs unit and system integration tests.

## Configuration Matrix

### Machine-Independent Steps

- `spellcheck` and `format` run once (not per-machine).

### Machine-Dependent Steps

`scripts/pipeline.sh` executes machine-dependent
steps for `microvm`.

| Machine       | Build Types    | Deployment Types              |
|---------------|----------------|-------------------------------|
| `microvm`     | debug, release | standalone, single, multi     |

### Build Parameter Mapping

| Deployment Type | `DEPLOYMENT_MODE` |
|-----------------|-------------------|
| standalone      | `standalone`      |
| single-process  | `single-process`  |

## Individual Quality Checks

```bash
# Spell check.
./z build -- spellcheck

# Format check.
./z build -- format-check

# Lint check.
./z build -- lint-check

# Formal verification.
./z build -- verify

# Unit tests.
./z build -- run-unit-tests
```

## GitHub Actions Workflows

Workflows are defined in `.github/workflows/`. They follow the same quality gates as the local
pipeline, but matrix coverage is split across multiple jobs and run on pull requests and pushes to
`dev`.

Matrix coverage in GitHub Actions:

- `checks`: format + spellcheck (single run).
- `lint`, `verify`, `ci-build`: `microvm` with `standalone`
  and `single-process`.
- `ci-test`: same matrix.

> **Note:** The `ci-windows` workflow validates Windows host builds (nanvixd, UserVM, source checks)
> and runs a smoke test using nanvixd in standalone interactive mode on WHP-enabled runners.

## Release Process

```bash
# Create a release archive.
./z build -- release

# The archive name follows this pattern:
# nanvix-<ver>-<target>-<machine>-<deploy>-<mode>-<log>-<memory>mb.tar.bz2
```

Releases can be created with:

```bash
# Patch release (X.Y.Z -> X.Y.(Z+1))
python3 scripts/create-release.py --patch   # Linux / macOS
python scripts/create-release.py --patch    # Windows

# Minor release (X.Y.Z -> X.(Y+1).0)
python3 scripts/create-release.py --minor   # Linux / macOS
python scripts/create-release.py --minor    # Windows

# Major release (X.Y.Z -> (X+1).0.0)
python3 scripts/create-release.py --major   # Linux / macOS
python scripts/create-release.py --major    # Windows
```

## Pipeline Output

The pipeline tracks and reports:

- Pass/fail/skip counts per step.
- Total elapsed time.
- Detailed error output for failed steps.
