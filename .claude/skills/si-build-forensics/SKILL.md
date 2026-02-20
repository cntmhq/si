---
name: si-build-forensics
description: >-
  Reverse-engineer and restore the build and deployment pipeline of the archived
  System Initiative monorepo on NixOS. Covers Buck2, Bazel, Nix flake, and Tilt
  analysis for getting containers running locally. Use when working on SI build
  restoration, Buck2/Bazel pipeline analysis, or NixOS-based dev environment repair.
compatibility: >-
  Requires NixOS with direnv, buck2, and Tilt. Designed for Claude Code.
metadata:
  author: connectome
  version: "0.1"
allowed-tools: Bash(fd:*) Bash(rg:*) Bash(wc:*) Bash(buck2:*) Bash(nix:*) Read
---

# SI Build Forensics

Restore a working local development environment for the archived [System Initiative](https://github.com/systeminit/si) monorepo on NixOS by reverse-engineering its build and deployment pipeline.

## Background

System Initiative was archived on 2025-02-06. The maintainers removed dependency artifacts from their S3 bucket, breaking the build pipeline. The goal is to reconstruct enough of the build and deployment process to get the containers running locally without depending on those remote artifacts.

## Success Criteria

The task is complete when **both** of these pass without error:

```sh
buck2 run dev:healthcheck
buck2 run dev:up              # Tilt spins up all containers cleanly
```

We do **not** care about application-layer correctness at this stage — only that the containers build and start.

## Build Toolchain Overview

The repo uses three interlocking build systems plus a container orchestrator:

| Tool   | Role                              | Entry point                        |
|--------|-----------------------------------|------------------------------------|
| Nix    | Reproducible devshell             | `./flake.nix`, activated via `direnv` (`.envrc`) |
| Buck2  | Primary build orchestration       | `BUCK` files in service directories |
| Bazel  | Rule definitions and macros       | `*.bzl` files throughout the repo  |
| Tilt   | Local container orchestration     | Invoked via `buck2 run dev:up`     |

## Step-by-Step Approach

### 1. Discover build manifests

Find all relevant files:

```sh
fd --follow --no-ignore "^.*\.bzl$|^.*BUCK$"
```

The Nix flake is at `./flake.nix` and the shell is controlled by `.envrc`.

### 2. Handle context-window limits

The build manifests are far too large to fit in context at once (Bazel files alone are ~100k lines; BUCK files ~30k lines). Use these strategies:

- **Incremental ingestion**: read files in batches, summarize each batch, then proceed.
- **Memory files**: write `CLAUDE.md` files in subdirectories to persist analysis across sessions. Build these out recursively as you go — see [Claude Code memory docs](https://code.claude.com/docs/en/memory).
- **Plan mode**: use Claude Plan Mode to outline the full analysis before executing, so you don't waste context on wrong paths.
- **Targeted search**: use `rg` and `fd` to answer specific questions rather than reading entire files.

### 3. Analyze the Nix devshell

Read `flake.nix` and `.envrc`. Identify:

- All dependencies the devshell provides (buck2, tilt, language toolchains, etc.)
- Any `fetchurl` / `fetchFromGitHub` calls that point at now-dead S3 URLs
- NixOS-specific patches or overrides

### 4. Analyze Buck2 build targets

Starting from `dev:healthcheck` and `dev:up`, trace the dependency graph:

```sh
buck2 targets //dev:
buck2 audit dep-files //dev:up
```

For each target, identify what it builds and what external resources it fetches.

### 5. Identify broken remote dependencies

Search for references to the removed S3 bucket or other now-unavailable URLs:

```sh
rg -n 's3://|https://.*systeminit.*\.s3' --type-add 'build:*.bzl,BUCK,*.nix'
```

For each broken reference, determine:

- What artifact it was fetching
- Whether the artifact can be rebuilt from source in the repo
- Whether an alternative source exists (e.g., Nix cache, container registries)

### 6. Patch and rebuild

Replace broken remote references with local builds or alternative sources. Iterate until the success criteria pass.

## Important Constraints

- **Do not modify application code** — only build/deployment configuration.
- **Preserve reproducibility** — prefer Nix-pinned replacements over ad-hoc downloads.
- **Document everything** — write findings into `CLAUDE.md` files so progress survives context resets.

## Reference Material

See [references/experiment-log.md](references/experiment-log.md) for the phased experiment plan and progress log.
