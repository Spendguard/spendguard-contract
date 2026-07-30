# Contributing to SpendGuard Contract

## How to contribute

1. **Fork** the repository.
2. **Create a feature branch** from `main`.
3. **Make your changes** following the coding standards below.
4. **Run tests** before opening a PR:
   ```bash
   cargo test --workspace
   ```
5. **Open a pull request** with a clear title and description.

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
