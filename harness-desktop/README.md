# Harness Desktop

Harness Desktop is a local-first, provider-agnostic desktop runtime for managing
projects, conversations, tasks, providers, and normalized usage in one place.
It targets macOS and Windows with Tauri 2, React, TypeScript, Rust, and SQLite.

This repository currently provides an executable product foundation and a
testable vertical slice. Cloud tenant access, production signing credentials,
remote worker certificates, and model artifacts are intentionally supplied by
the operator and are never embedded in source control. See
[`docs/IMPLEMENTATION_STATUS.md`](docs/IMPLEMENTATION_STATUS.md) for the exact
phase-by-phase boundary.

## Development

Prerequisites:

- Node.js 22 or newer
- Rust 1.77.2 or newer
- platform prerequisites from the Tauri 2 documentation

```sh
npm install
npm run check
npm run tauri dev
```

Backend-only validation:

```sh
cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

## Security defaults

- Claude Code and Codex authentication is inherited through their official CLI
  status and execution commands. Harness does not read their credential files.
- Microsoft authentication is a desktop public-client boundary. No client
  secret belongs in this application; refresh credentials must use the OS
  credential store.
- Provider processes receive an explicit workspace and permission profile.
- Destructive operations are never auto-approved.
- Provider manifests, model downloads, runtime downloads, and app updates must
  pass their configured integrity or signature checks before activation.

## Project map

- `src/` — desktop interface, native IPC bridge, and explicit Vite-only demo adapter
- `src-tauri/` — Rust Harness Core, SQLite store, router, scheduler, providers
- `providers/` — versioned provider manifests and JSON-RPC schema
- `catalog/` — signed model-catalog payload schema
- `docs/` — architecture, security, protocol, testing, and phase traceability

## External configuration

Copy `.env.example` only for local development. Do not add tokens, client
secrets, signing keys, tenant data, or model license acceptances to `.env` or
the database. Production builds must provision those through platform-specific
secure channels.
