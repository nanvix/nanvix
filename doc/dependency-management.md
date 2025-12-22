# Dependency Management

This document explains how dependency updates are managed in the Nanvix repository.

## Overview

Nanvix uses two tools for automated dependency updates:

- **Renovate**: Manages Rust (Cargo) dependencies
- **Dependabot**: Manages GitHub Actions dependencies

## Why Two Tools?

GitHub's Dependabot has a known limitation with Cargo workspaces: it does not support updating dependencies defined in the `[workspace.dependencies]` section of the root `Cargo.toml` file. Since Nanvix uses a Cargo workspace with centralized dependency management, we use Renovate for Cargo dependencies because it has full support for workspace dependencies.

Dependabot remains in use for GitHub Actions updates, as it works well for that ecosystem and is natively integrated with GitHub.

## Configuration

### Renovate Configuration

The Renovate configuration is defined in `renovate.json` at the repository root. Key features:

- **Schedule**: Runs daily before 4:00 AM (similar timing to Dependabot)
- **Labels**: All PRs are labeled with "enhancement"
- **Grouping**: 
  - KVM-related dependencies (`kvm-ioctls`, `kvm-bindings`) are grouped together
  - Workspace dependencies are grouped together
- **Ignored Dependencies**: Hyperlight packages (`hyperlight-common`, `hyperlight-host`, `hyperlight-guest`) are managed externally and excluded from updates
- **PR Limit**: Maximum of 20 concurrent PRs

### Dependabot Configuration

The Dependabot configuration is defined in `.github/dependabot.yml`. It only manages GitHub Actions dependencies.

## Setting Up Renovate

To enable Renovate for this repository:

1. Install the [Renovate GitHub App](https://github.com/apps/renovate) on the nanvix organization
2. Grant it access to the nanvix/nanvix repository
3. Renovate will automatically detect the `renovate.json` configuration and start creating PRs

No additional configuration is needed - the `renovate.json` file in the repository contains all necessary settings.

## References

- [Renovate Documentation](https://docs.renovatebot.com/)
- [Dependabot Documentation](https://docs.github.com/en/code-security/dependabot)
- [GitHub Issue #7896](https://github.com/dependabot/dependabot-core/issues/7896): Dependabot limitation with Cargo workspace dependencies
