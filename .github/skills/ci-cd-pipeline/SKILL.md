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

`scripts/pipeline.sh` currently executes machine-dependent
steps only for `microvm` and `hyperlight` (qemu variants are
excluded in `is_excluded()`).

| Machine       | Build Types    | Deployment Types     |
|---------------|----------------|----------------------|
| `microvm`     | debug, release | single, multi, l2    |
| `hyperlight`  | debug, release | single, multi, l2    |

### Build Parameter Mapping

| Deployment Type | `DEPLOYMENT_MODE` |
|-----------------|-------------------|
| standalone      | `standalone`      |
| single-process  | `single-process`  |
| multi-process   | `multi-process`   |
| l2              | `l2`              |

## Individual Quality Checks

```bash
# Spell check.
./z build --with-cached-options -- spellcheck

# Format check.
./z build --with-cached-options -- format-check

# Lint check.
./z build --with-cached-options -- lint-check

# Formal verification.
./z build --with-cached-options -- verify

# Unit tests.
./z build --with-cached-options -- run-unit-tests
```

## GitHub Actions Workflows

Workflows are defined in `.github/workflows/`. They follow the same quality gates as the local
pipeline, but matrix coverage is split across multiple jobs (including dedicated L2 jobs) and run on
pull requests and pushes to `dev`.

Matrix coverage in GitHub Actions:

- `checks`: format + spellcheck (single run).
- `lint`, `verify`, `ci-build`, `ci-test`: `qemu-pc`, `microvm`, `hyperlight` with `single-process`
  and `multi-process` (excluding `qemu-pc + single-process`).
- `ci-l2`: separate L2 jobs for `microvm` and `hyperlight`.

> **Note:** There is no Windows CI job yet. Windows builds must be verified manually.
> Once a `ci-windows` job is added, it should validate that platform-independent library crates
> compile and pass Clippy on Windows, and eventually build and test the UserVM with the WHP
> backend.

## Release Process

```bash
# Create a release archive.
./z build -- release

# The archive name follows this pattern:
# nanvix-<ver>-<machine>-<deploy>-<mode>-<log>.tar.bz2
```

Minor releases can be created with:

```bash
./scripts/create-minor-release.sh
```

## Pipeline Output

The pipeline tracks and reports:

- Pass/fail/skip counts per step.
- Total elapsed time.
- Detailed error output for failed steps.
