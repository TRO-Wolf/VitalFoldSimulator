# `/` — repo root map

**Purpose:** Top-level navigation for the VitalFold Engine repository — a Rust REST API that generates synthetic cardiac-clinic data into Aurora DSQL + DynamoDB. Start here, then drop into the directory map for wherever your task lives.

## Contents

| Entry | What it is |
|---|---|
| [README.md](README.md) | Front-door overview + fictitious-data disclaimer (load-bearing — never weaken) |
| [CLAUDE.md](CLAUDE.md) | Agent orientation; read first. Documents the `map.md` navigation rule (§3.1) |
| [CHANGELOG.md](CHANGELOG.md) | Keep-a-Changelog; update under `[Unreleased]` |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Code style + no-unwrap policy |
| LICENSE | MIT |
| [docs/](docs/map.md) | Supporting reference docs (models, dynamo, frontend, airflow) + archived history |
| [vital-fold-engine/](vital-fold-engine/map.md) | The Rust crate — all source, migrations, the Preact UI, and crate-level docs |
| [.github/](.github/map.md) | CI configuration |

## I want to…

| I want to… | Go to |
|---|---|
| Get the service running | [vital-fold-engine/QUICKSTART.md](vital-fold-engine/QUICKSTART.md) |
| Understand the data model / tables | [docs/models-spec.md](docs/models-spec.md), [vital-fold-engine/migrations/](vital-fold-engine/migrations/map.md) |
| Read or change Rust source | [vital-fold-engine/src/](vital-fold-engine/src/map.md) |
| Touch the admin dashboard | [vital-fold-engine/static/](vital-fold-engine/static/map.md) |
| Understand project invariants before editing | [CLAUDE.md](CLAUDE.md) §7 |
| See historical build/deprecation notes | [docs/history/](docs/history/map.md) |

## Pointers

- [CLAUDE.md](CLAUDE.md) is the authoritative orientation document; this map is just the directory index.
- Every tracked directory carries its own `map.md` — follow the child-map links above to navigate down.

## Debug

- All build/test/lint commands run from `vital-fold-engine/` (the crate root), not from here.
- CI mirror: [.github/workflows/ci.yml](.github/workflows/ci.yml).
