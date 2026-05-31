# `.github/workflows/` — map

**Purpose:** Continuous-integration pipeline definitions, run on every push and PR.

## Contents

| File | What it is |
|---|---|
| ci.yml | Two jobs on ubuntu-latest. **test**: `cargo fmt --check` → `cargo clippy --all-targets --all-features -- -D warnings` (strict) → `cargo check` → `cargo test`. **audit**: `cargo audit` (RustSec advisories). Actions are SHA-pinned; jobs run least-privilege (`permissions: contents: read`). |

## I want to…

| I want to… | Go to |
|---|---|
| Understand the PR gate | `ci.yml` |
| Reproduce CI locally | From `vital-fold-engine/`: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo check --all-targets`, `cargo test --all-targets` (see [CLAUDE.md](../../CLAUDE.md) §4) |
| Bump a pinned action | Edit the `@<sha> # vX.Y.Z` pin (Dependabot also opens weekly PRs — see [../dependabot.yml](../map.md)) |

## Pointers

- Parent: [.github/](../map.md).
- The commands mirrored here are documented in [CLAUDE.md](../../CLAUDE.md) §4 and §10; the lint policy lives in [Cargo.toml](../../vital-fold-engine/Cargo.toml) `[lints]`.

## Debug

- CI failing on `fmt` but local is clean? Confirm you ran `cargo fmt` from `vital-fold-engine/`, not the repo root.
- clippy is now `-D warnings`-gated — any warning fails the build. Run the exact clippy command above locally before pushing.
- `audit` job red? It's a new dependency advisory. `RUSTSEC-2023-0071` (rsa, via the unused sqlx MySQL driver) is intentionally ignored via `--ignore` in `ci.yml`; new advisories need triage (update the dep or add a justified `--ignore`).
