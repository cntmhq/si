# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build System

This repository uses **Buck2** as the primary build system with Nix for development environment management. All commands should be run from within the Nix environment (automatically activated via direnv).

### Common Commands

```bash
# Run the full stack locally (requires Docker)
buck2 run dev:up

# Stop stack (keeps data)
buck2 run dev:stop

# Tear down stack (removes data)
buck2 run dev:down

# Check environment readiness
buck2 run dev:healthcheck

# Start dependent services only (PostgreSQL, NATS, etc.)
buck2 run dev:platform

# Pull latest Docker images
buck2 run //dev:pull
```

### Building

```bash
# Build a Rust crate
buck2 build //lib/dal

# Build with release mode
buck2 build @//mode/release //bin/sdf

# List all targets in a directory
buck2 targets //lib/dal:
```

### Testing

```bash
# Run all tests for a crate
buck2 test //lib/dal:test

# Run integration tests
buck2 run //lib/dal:test-integration

# Run a specific test
buck2 run //lib/dal:test-integration -- edge::new

# Run with exact pattern
buck2 run //lib/dal:test-integration -- --test integration integration_test::internal::edge::new -- --exact

# Run ignored tests
buck2 run //lib/dal:test-integration -- edge::new -- --ignored

# See live logs during tests
SI_TEST_LOG=info buck2 run //lib/dal:test-integration -- <pattern> -- --nocapture
```

### Rust Dependencies

```bash
# Update all crates
cargo update
buck2 run //support/buck2:sync-cargo-deps

# Update a single crate
cargo update -p <crate> --precise <version>
buck2 run //support/buck2:sync-cargo-deps
```

### Documentation

```bash
# Generate and open Rust docs
buck2 run //lib/dal:doc -- --document-private-items --open
```

## NixOS Build Restoration (Post-Archive)

SI was archived on 2025-02-06 and its S3 artifact hosting at `artifacts.systeminit.com` was removed. The build pipeline has been restored for NixOS using **system toolchains** instead of hermetic downloads.

### What was changed

| File | Change |
|------|--------|
| `toolchains/BUCK` | Rewritten to use `system_rust_toolchain`, `system_cxx_toolchain`, `system_python_bootstrap_toolchain` — tools from Nix devshell PATH instead of S3 downloads |
| `flake.nix` | Added `llvmPackages.lld` (for `-fuse-ld=lld`), `prisma-utils` input for Prisma engine binaries |
| `dev/Tiltfile` | Prisma engine env vars for `auth-db-seed` and `auth-api`; `http://` URLs instead of `https://` for local dev; `module-index` auto-start enabled |
| `app/web/.env` | `VITE_AUTH_PORTAL_URL=http://localhost:9000`, `VITE_MODULE_INDEX_API_URL=http://localhost:5157` |
| `prelude-si/deno/*.py` | Fixed `resolve_exe()` to use `shutil.which()` for plain binary names (NixOS PATH lookup) |
| `bin/auth-api/src/lib/stripe.ts` | Lazy Stripe init — doesn't crash on startup when `STRIPE_API_KEY` is unset |

### Prisma on NixOS

Prisma 5.20.0 engine binaries are provided via `nix-prisma-utils` (see `flake.nix`). The nix store path is referenced directly in `dev/Tiltfile` for the `auth-db-seed` and `auth-api` resources.

If you bump the Prisma version in `bin/auth-api/package.json`:
1. Find the new engine commit hash in `node_modules/prisma/build/index.js` (search for the 40-char hex string)
2. Update `versionString` in `flake.nix`
3. Run `nix build .#devShells.x86_64-linux.default` with `hash = ""` to get the correct hash
4. Fill in the hash and update the `_prisma_bin` path in `dev/Tiltfile`

### Known limitations

- `STRIPE_API_KEY`, `AUTH0_CLIENT_SECRET`, `AUTH0_M2M_CLIENT_SECRET`, `LAGO_API_KEY`, `GH_TOKEN` are not set for local dev — Stripe/Auth0/Lago routes will fail if called, but the service starts fine
- `module-index` starts but has no data — assets/components/functions must be imported manually or seeded

## Architecture Overview

System Initiative is an AI-native infrastructure automation platform built as a Rust monorepo with a Vue 3 frontend.

### Directory Structure

| Directory | Contents |
|-----------|----------|
| `app/` | Frontend applications (web UI, auth portal, docs) |
| `bin/` | Backend services and CLI tools |
| `lib/` | Shared Rust libraries |
| `component/` | Docker images and ancillary tooling |
| `prelude-si/` | Custom Buck2 rules |

### Core Services (bin/)

- **sdf**: Main API server - handles all frontend requests, schema management, graph operations
- **veritech**: Function execution engine - computes attributes and runs user-defined functions
- **rebaser**: Change set management - git-like conflict detection and resolution
- **pinga**: Job queue execution service
- **forklift**: Data warehouse integration
- **edda**: Builds materialized views for the frontend
- **cyclone**: Secure container execution runtime

### Key Libraries (lib/)

- **dal**: Data Access Layer - central ORM providing models for components, functions, attributes, change sets. This is where most backend business logic lives.
- **sdf-server**: HTTP server implementation using Axum
- **si-data-pg**: PostgreSQL connection pooling (deadpool + tokio-postgres)
- **si-data-nats**: NATS messaging client for async pub/sub between services
- **si-data-spicedb**: SpiceDB integration for authorization
- **si-frontend-types-rs**: Shared type definitions between Rust backend and TypeScript frontend

### Service Communication

Services follow a three-tier pattern:
```
bin/<service>/           # Binary entry point
lib/<service>-server/    # HTTP/NATS endpoints
lib/<service>-core/      # Business logic (optional)
lib/<service>-client/    # Client library for other services
```

Communication happens via:
- **HTTP/WebSocket**: Frontend to sdf-server
- **NATS**: Inter-service messaging and events
- **Direct client libraries**: Service-to-service calls

### Frontend (app/web/)

Vue 3 + TypeScript application using:
- **Vite** for building
- **Pinia** for state management
- **Tailwind CSS** for styling
- **D3/Graphology/Sigma.js** for graph visualization
- **Yjs** for real-time collaboration (CRDT)

Key directories:
- `src/store/`: Pinia stores for state management
- `src/components/`: Reusable Vue components
- `src/api/`: Backend API client integration

### Data Flow

```
Frontend (Vue) → HTTP/WebSocket → SDF Server → DAL → PostgreSQL
                                      ↓
                               NATS Events → Other Services
```

## Adding New Rust Libraries

1. Create `lib/<crate>/Cargo.toml`:
```toml
[package]
name = "<crate>"
edition = "2024"
version.workspace = true
authors.workspace = true
license.workspace = true
repository.workspace = true
rust-version.workspace = true
publish.workspace = true
```

2. Add `src/lib.rs`
3. Add to workspace members in root `Cargo.toml`
4. Run `cargo check --all-targets --all-features --workspace`
5. If adding/modifying third-party crates: `buck2 run //support/buck2:sync-cargo-deps`

## Environment Setup Notes

- Requires `nix` with flakes enabled (use Determinate Nix Installer)
- Requires Docker
- On macOS/WSL2: increase file descriptor limit (`ulimit -n 10240`)
- On Linux: may need to increase `fs.inotify.max_user_watches`
- On NixOS: the build pipeline uses system toolchains from the Nix devshell — no S3 downloads required
