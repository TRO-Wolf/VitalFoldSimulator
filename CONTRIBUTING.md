# Contributing to VitalFold Engine

Thanks for your interest in contributing. This is a synthetic healthcare data generation project — no real PHI ever touches the code. Patches, issue reports, and discussion are all welcome.

## Quick setup

```bash
git clone https://github.com/TRO-Wolf/VitalFoldSimulator.git
cd VitalFoldSimulator/vital-fold-engine
cp .env.example .env            # Fill in DSQL endpoint, AWS creds, JWT_SECRET
cargo build
cargo test
cargo run                       # Serves on http://0.0.0.0:8787
```

AWS prerequisites (Aurora DSQL cluster + two DynamoDB tables + IAM permissions) are documented in [`vital-fold-engine/INSTALLATION.md`](vital-fold-engine/INSTALLATION.md).

Generate a JWT secret:

```bash
openssl rand -base64 32
```

## Running the test suite

```bash
cd vital-fold-engine
cargo test --all-targets
```

CI runs `cargo check`, `cargo test`, and `cargo clippy` on every PR — see [`.github/workflows/ci.yml`](.github/workflows/ci.yml).

## Pull request expectations

- **Branch from `main`**, name it something descriptive (`feat/...`, `fix/...`, `docs/...`, `chore/...`).
- **Keep commits focused.** One logical change per commit; squash fixups before opening the PR.
- **Describe the why, not just the what.** The PR body should explain motivation, not duplicate the diff.
- **CI must be green.** `cargo check` + `cargo test` + `cargo clippy` all pass on `ubuntu-latest`.
- **Update the docs in the same PR** when you change the schema, an endpoint, or a generator step. The [`CHANGELOG.md`](CHANGELOG.md) `[Unreleased]` section is the right place for a short entry.
- **Never commit secrets.** `.env`, AWS keys, and JWT secrets are in `.gitignore` for a reason — don't override it.

## Code style

- Rust 2021 edition. Run `cargo fmt` before committing.
- Never use bare `.unwrap()` or `.expect()` — prefer `?`, `.ok_or_else()`, or `.unwrap_or_else()` with a safe fallback. This is enforced in review.
- Generators that use `rand::rng()` must drop the RNG before any `.await` (it's `!Send`). See existing generators in `src/generators/` for the pattern.
- Bulk inserts use `UNNEST` with a 2500-row batch cap to stay within Aurora DSQL's per-statement row limit.

## Code of conduct

Be respectful. Assume good faith. Focus on the code and the problem, not on the contributor.

## Questions

Open an issue with the `question` label or start a discussion. Security-sensitive reports should be sent privately to the repo owner rather than filed as a public issue.
