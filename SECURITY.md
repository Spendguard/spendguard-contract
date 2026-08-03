# Security Policy

## Scope

This contract is a **read-only convenience wrapper** over public on-chain data.
It holds no funds, enforces no rules, and emits no events.

The security boundary is limited to:
- Correctness of cross-contract call decoding.
- Proper error propagation (no silent truncation of policy state).
- No panic in production code paths.

## Audit status

The narrow read-only scope described above reduces the practical risk surface, but
that is not the same as having been independently audited. This project has not
undergone a professional security audit.

## No confidential data

Every function in this contract reads public on-chain data and returns
public data. There is no secret or confidential information anywhere
in this contract's execution path.

## Reporting a vulnerability

If you find a bug that could lead to incorrect data being returned to
the SpendGuard indexer, please open a GitHub Issue with the label
`security`. We aim to respond within 48 hours.
