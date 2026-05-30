# `.github/workflows/` — map

**Purpose:** Continuous-integration pipeline definitions, run on every push and PR.

## Contents

| File | What it is |
|---|---|
| ci.yml | Runs `cargo check`, `cargo test`, `cargo clippy`, `cargo fmt --check` on ubuntu-latest with rust-cache |

## I want to…

| I want to… | Go to |
|---|---|
| Understand the PR gate | `ci.yml` |
| Reproduce CI locally | Run the four cargo commands from `vital-fold-engine/` (see [CLAUDE.md](../../CLAUDE.md) §4) |

## Pointers

- Parent: [.github/](../map.md).
- The commands mirrored here are documented in [CLAUDE.md](../../CLAUDE.md) §4 and §10.

## Debug

- CI failing on `fmt` but local is clean? Confirm you ran `cargo fmt` from `vital-fold-engine/`, not the repo root.
- `clippy` job is not `-D warnings` gated here; warnings don't fail the build (see `ci.yml`).
