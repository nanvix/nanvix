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

| Machine       | Build Types    |
|---------------|----------------|
| `microvm`     | debug, release |

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
- `lint`, `verify`, `ci-build`: `microvm` in debug and release configurations.
- `ci-test`: same matrix.
- Windows X64 jobs run source checks, debug/release builds, unit tests, in-kernel tests,
  integration tests, and POSIX suites on GitHub-hosted runners.
- Native Windows ARM64 jobs run build-driver target-selection tests, lint, debug/release builds,
  unit tests, in-kernel tests, integration tests, and POSIX suites on self-hosted runners labeled
  `self-hosted`, `Windows`, and `ARM64` in the `Benchmark` runner group.
- Windows benchmarks run on the matching X64 and ARM64 self-hosted runners and persist separate
  architecture-specific histories.

Self-hosted jobs are gated by `.github/actions/check-runners`. Configure `RUNNER_ADMIN_TOKEN` with
organization administration read access so CI can detect online runners without leaving jobs
queued indefinitely.

## Release Process

```bash
# Create a release archive.
./z build -- release

# The archive name follows this pattern:
# nanvix-<ver>-<target>-<machine>-standalone-<mode>-<log>-<memory>mb.tar.bz2
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
