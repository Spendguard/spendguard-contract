# Contributing to SpendGuard Contract

## How to contribute

1. **Fork** the repository.
2. **Create a feature branch** from `master`.
3. **Make your changes** following the coding standards below.
4. **Run the CI gates** before opening a PR:
   ```bash
   cargo test --workspace
   cargo clippy --workspace --all-targets -- -D warnings
   ```
5. **Stage the files you changed** (`git add <path>`) rather than `git add -A`.
   `target/` and `contracts/*/test_snapshots/` are build and test output; they
   are listed in `.gitignore` and are never committed.
6. **Open a pull request** with a clear title and description.

## Commit convention

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): description
```

Types: `feat`, `fix`, `chore`, `test`, `docs`, `refactor`, `perf`.

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

## Review process

All PRs require at least one approval. The reviewer checks:
- Correctness of cross-contract call signatures.
- Proper error handling (no silent failures).
- Test coverage (positive + negative cases for every function).
