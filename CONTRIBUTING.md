# Contributing to SpendGuard Contract

## How to contribute

1. **Fork** the repository.
2. **Create a feature branch** from `master`.
3. **Make your changes** following the coding standards below.
4. **Run the CI gates** before opening a PR:
   ```bash
   make check        # cargo test --workspace + cargo clippy … -D warnings
   make spec-check    # the committed interface still describes your built WASM
   ```
5. **Stage the files you changed** (`git add <path>`) rather than `git add -A`.
   `target/` and `contracts/*/test_snapshots/` are build and test output; they
   are listed in `.gitignore` and are never committed.
6. **Open a pull request** with a clear title and description.

## If your PR changes the public interface

`WASM_VERSION` in `src/lib.rs` is the marker the indexer checks before it decodes
anything, and the contract ID in the README is the address it trusts, so an API
change is only half a change:

1. **Bump `WASM_VERSION`** and add a row to the README Changelog. A new
   entrypoint, a new `PolicyViewError` variant, or a changed return type all
   count; adding a test does not.
2. **Regenerate the committed interface**: `make spec-file`, then commit
   `specs/policy-view-helper.txt`. CI builds your WASM and diffs that file
   against it, so the API change lands in review as a diff of the interface
   instead of surfacing later as a broken indexer. CI pins the stellar-cli in the
   Makefile (`PINNED_STELLAR_CLI`) because the file is the CLI's formatting of
   the spec, not the WASM's; if your `spec-check` diff looks like whitespace
   rather than signatures, check `stellar --version` first.
3. **Run `make verify`.** It compares this checkout's WASM hash *and* published
   spec against the deployed instance and says plainly when the README describes
   a contract that is not the one running.
4. **Deploying is a separate, deliberate step** —
   `make deploy SOURCE_ACCOUNT=<testnet key alias>`, and `NETWORK` anything but
   `testnet` additionally needs `ALLOW_MAINNET=1`. A deploy that you intend to
   publish updates the README's Deployed instances table, the Makefile's
   `CONTRACT_ID` / `EXPECTED_WASM_HASH`, and the app's
   `POLICY_VIEW_HELPER_CONTRACT_ID` together. See README → *Build & deploy*.

## Commit convention

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): description
```

Types: `feat`, `fix`, `chore`, `test`, `docs`, `refactor`, `perf`, `build`, `ci`.
The last two are for the Makefile and the workflows: this repo's reproducibility
claims live there, so they get their own prefix rather than hiding under `chore`.

Scope for this repo: `types`, `read`, `workspace`, `deps`.

Examples:
- `feat(read): add get_spending_limit_state`
- `test(read): add negative case for missing policy`
- `docs: add README`

## Coding standards

- **No `unwrap()`** outside `#[cfg(test)]` blocks.
- **No floats** — token amounts use `i128` and basis-point integer math.
- **Snake case** functions, **PascalCase** types (Soroban conventions).
- Every public function must have doc comments.
- Every function must have at least one positive and one negative test.
- **Negative tests name the error variant.** Use the `policy_view_error(..)`
  helper in `test.rs` and compare against a `PolicyViewError`, not
  `assert!(result.is_err())` — `is_err()` passes for any of the four variants
  and for a call that never reached the contract at all.

## Review process

What blocks a merge is CI: both jobs in `.github/workflows/ci.yml` have to be
green. On top of that, a maintainer reads every PR against this list — approvals
are not currently *required* by branch protection, so the list is a convention,
not a gate:

- Correctness of cross-contract call signatures.
- Proper error handling (no silent failures).
- Test coverage (positive + negative cases for every function).
- `specs/policy-view-helper.txt` regenerated when the interface changed, and
  `WASM_VERSION` bumped with it.
