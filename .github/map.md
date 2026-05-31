# `.github/` — map

**Purpose:** GitHub-specific configuration — CI workflows + Dependabot.

## Contents

| Entry | What it is |
|---|---|
| [workflows/](workflows/map.md) | CI workflow definitions (test + audit jobs) |
| dependabot.yml | Weekly grouped dependency-update PRs for the Cargo crate (`/vital-fold-engine`) and GitHub Actions (`/`) |

## I want to…

| I want to… | Go to |
|---|---|
| See what CI runs on push/PR | [workflows/map.md](workflows/map.md) |
| Change dependency-update cadence / grouping | dependabot.yml |

## Pointers

- Parent: [repo root](../map.md).

## Debug

- Dependabot PRs come in two groups (`cargo`, `actions`); a noisy week usually means a security advisory landed (those bypass the weekly schedule).
