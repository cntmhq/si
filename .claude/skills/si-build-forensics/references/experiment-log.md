# Experiment Log

Progress tracking for the SI build forensics project.

## Phases

### Phase 0: Project Initialization ✅

- Created initial `CLAUDE.md` for the repository by having Claude analyze the repo structure.
- The generated document was extensive and covers high-level repo layout.

### Phase 1: Deep Repository Analysis ✅

**Objective**: Ingest and summarize all build/deployment manifests so Claude has a working mental model of the pipeline.

**Completed**: Full dependency chain traced from `dev:healthcheck` → `dev:up` through all toolchain layers.

**Key findings**:

- `dev:healthcheck` and `dev:up` are defined in `dev/BUCK` and orchestrated by Tilt.
- Buck2 hermetic toolchains (Python 3.13.6, Rust 1.88.0, Rust nightly, Clang 20.1.0, Deno 2.2.12) are downloaded from `https://artifacts.systeminit.com/toolchains/...` — a bucket that was deleted when SI was archived.
- The download chain: `toolchains/BUCK` → `prelude-si/toolchains/common.bzl` → `prelude-si/artifact.bzl` → `ctx.actions.download_file`.
- All five toolchain archives are missing and cause immediate build failure.
- Buck2 prelude provides `system_rust_toolchain`, `system_cxx_toolchain`, `system_python_bootstrap_toolchain` as alternatives that use PATH.

### Phase 2: Dependency Cataloguing ✅

**All broken remote references identified**:

| Toolchain | Version | Artifact URL (now dead) |
|-----------|---------|------------------------|
| Python | 3.13.6 | `artifacts.systeminit.com/toolchains/python/3.13.6/linux/x86_64/python-3.13.6-linux-x86_64.tar.gz` |
| Rust stable | 1.88.0 | `artifacts.systeminit.com/toolchains/rust/1.88.0/linux/x86_64/rust-1.88.0-linux-x86_64.tar.gz` |
| Rust nightly | nightly-2025-04-17 | `artifacts.systeminit.com/toolchains/rust/nightly-2025-04-17/linux/x86_64/...` |
| Clang | 20.1.0 | `artifacts.systeminit.com/toolchains/clang/20.1.0/linux/x86_64/...` |
| Deno | 2.2.12 | `artifacts.systeminit.com/toolchains/deno/2.2.12/linux/x86_64/...` |

No other remote dependencies found in the core build path.

### Phase 3: Build Restoration 🔄

**Objective**: Patch broken remote references and iterate until `buck2 run dev:healthcheck` and `buck2 run dev:up` pass.

**Status**: `dev:healthcheck` ✅ PASSES. `buck2 build //bin/sdf:sdf` 🔄 in progress.

---

#### Attempt A: Local HTTP Server Re-hosting (Abandoned)

**Approach**:
1. Point `prelude-si/artifact.bzl` default URL at `http://127.0.0.1:8080/toolchains` (changed from `https://artifacts.systeminit.com/toolchains`).
2. Build toolchain archives from Nix store paths, matching the `toolchain/{bin,lib}` directory structure.
3. Serve them via `python -m http.server 8080`.
4. Update SHA256 checksums in `*_distribution.bzl` files.

**Python archive**: Built from `nix build nixpkgs#python313` store path. Worked.

**Rust archive**: Built from `nix build nixpkgs#rustc`, `nixpkgs#cargo` store paths. Worked.

**Clang archive**: This is where things got painful on NixOS:
- `Scrt1.o`, `crti.o`, `crtn.o` (CRT startup files) not included — had to copy from `$(nix-build '<nixpkgs>' -A glibc)/lib`.
- `crtbeginS.o`, `crtendS.o` from `$(nix-build '<nixpkgs>' -A gcc)/lib/gcc/x86_64-unknown-linux-gnu/*/`.
- `-B {dir}/lib/x86_64-linux-gnu` flag needed in `linker_flags` (NOT `binary_linker_flags` — proc-macros and shared libs only use `linker_flags`).
- `-L {dir}/lib/x86_64-linux-gnu` similarly needed in `linker_flags`.
- `libgcc_s.so`: lld needs a `.so` (unversioned) but only `.so.1` exists. Fix: create GNU ld linker script stub: `echo "GROUP ( libgcc_s.so.1 )" > libgcc_s.so`.
- `libclang_rt.builtins.a` needed at `lib/clang/19/lib/x86_64-unknown-linux-gnu/` — copied from clang Nix store.

**Abandoned because**: Too much whack-a-mole. Each fix uncovered another NixOS path mismatch. The fundamental problem is that pre-built Linux binaries hardcode `/lib64/ld-linux-x86-64.so.2` and `/usr/lib` which don't exist on NixOS. Without `patchelf` on every binary in the archive, this is a losing battle.

**Garage S3 server**: Also tried `nix shell nixpkgs#garage_2` as the artifact server. Abandoned immediately — Garage requires authentication; Buck2's `download_file` sends no auth headers → 403.

---

#### Attempt B: System Toolchains ✅ (Current Approach)

**Insight**: Buck2 prelude provides `system_*_toolchain` rules that use whatever is in `$PATH`. The Nix devshell provides all needed tools (clang, rustc, cargo, python3, deno). No downloads needed at all.

**Changes made**:

**`toolchains/BUCK`** — Completely rewritten:
```python
load("@prelude//toolchains:rust.bzl", "system_rust_toolchain")
load("@prelude//toolchains:cxx.bzl", "system_cxx_toolchain")
load("@prelude//toolchains:python.bzl", "system_python_bootstrap_toolchain", "system_python_toolchain")
# ... plus SI custom loads

system_cxx_toolchain(name = "cxx", visibility = ["PUBLIC"])
system_python_bootstrap_toolchain(name = "python_bootstrap", visibility = ["PUBLIC"])
system_python_toolchain(name = "python", visibility = ["PUBLIC"])
system_rust_toolchain(
    name = "rust_release",
    default_edition = "2024",
    clippy_toml = "root//:clippy.toml",
    visibility = ["PUBLIC"],
    rustc_target_triple = "x86_64-unknown-linux-gnu",
    rustc_flags = ["-Copt-level=3", "-Cdebuginfo=line-tables-only", ...],
)
deno_toolchain(name = "deno-linux-x86_64", deno_exe = "deno", target_string = "linux-x86_64")
si_rust_toolchain(name = "si_rust", rustfmt_toml = "root//:rustfmt.toml", visibility = ["PUBLIC"])
toml_toolchain(name = "toml", taplo_config = "root//:.taplo.toml", ...)
```

**`flake.nix`** — Added to `buck2BuildInputs` (Linux only):
```nix
llvmPackages.lld   # provides ld.lld for -fuse-ld=lld
```

**Why `lld`?**: `system_cxx_toolchain` on Linux auto-adds `-fuse-ld=lld` when not using g++. This requires `ld.lld` in PATH. The devshell had `clang` but not `lld`.

**Result**: `buck2 run dev:healthcheck` ✅ passes instantly — no downloads, no extraction, no NixOS path issues.

**Remaining**: `buck2 build //bin/sdf:sdf` is next. After `direnv reload` to pick up `llvmPackages.lld`, the `-fuse-ld=lld` issue should be resolved. Further NixOS-specific issues may emerge as the full Rust build runs.

---

#### Future: buck2.nix Integration

[Tweag's buck2.nix](https://github.com/tweag/buck2.nix) offers an architecturally cleaner approach:
- Expose Nix derivation outputs as first-class Buck2 targets via `flake.package()` + Buck2 external cells
- Solves NixOS ELF/shebang problems at the derivation level — binaries are patched by Nix, not hacked around by Buck2 flags
- Compatible with remote execution setups
- More setup complexity but genuinely reproducible

The current `system_*_toolchain` approach is pragmatic for local dev. If reproducibility across machines matters, buck2.nix is the right next step.

---

## Key Lessons

1. **Don't fight NixOS with non-Nix binaries.** Pre-built Linux binaries won't work on NixOS without `patchelf`. Buck2's hermetic toolchain model assumes a standard FHS Linux and is fundamentally incompatible with NixOS without patching every binary.

2. **Buck2 system toolchains exist for exactly this situation.** `system_rust_toolchain`, `system_cxx_toolchain` etc. are designed for cases where the environment provides the tools. Use them.

3. **`binary_linker_flags` ≠ `linker_flags` in Buck2.** `binary_linker_flags` only applies to final executable linking. Proc-macros, build scripts, and shared libraries only get `linker_flags`. The `-B` and `-L` flags for CRT/lib search must go in `linker_flags`.

4. **`system_cxx_toolchain` adds `-fuse-ld=lld`.** Ensure `llvmPackages.lld` is in the devshell — not just `clang`.

5. **Buck2 daemon caches file sizes.** After rebuilding a tarball, kill the Buck2 daemon (`buck2 kill`) and delete `buck-out/v2/gen/toolchains/` before retrying. Also `find buck-out/ -not -perm -u=w -exec chmod u+w {} \;` before `rm -rf` since extracted toolchain dirs have read-only files.
