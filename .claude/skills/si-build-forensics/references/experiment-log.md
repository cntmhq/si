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
- Buck2 hermetic toolchains (Python 3.13.6, Rust 1.88.0, Rust nightly, Clang 20.1.0, Deno 2.2.12) are downloaded from `https://artifacts.systeminit.com/toolchains/...` — a bucket deleted when SI was archived.
- The download chain: `toolchains/BUCK` → `prelude-si/toolchains/common.bzl` → `prelude-si/artifact.bzl` → `ctx.actions.download_file`.
- All five toolchain archives are missing and cause immediate build failure.
- Buck2 prelude provides `system_rust_toolchain`, `system_cxx_toolchain`, `system_python_bootstrap_toolchain` as alternatives that use PATH.

### Phase 2: Dependency Cataloguing ✅

**All broken remote references identified**:

| Toolchain | Version | Artifact URL (now dead) |
|-----------|---------|------------------------|
| Python | 3.13.6 | `artifacts.systeminit.com/toolchains/python/3.13.6/linux/x86_64/...` |
| Rust stable | 1.88.0 | `artifacts.systeminit.com/toolchains/rust/1.88.0/linux/x86_64/...` |
| Rust nightly | nightly-2025-04-17 | `artifacts.systeminit.com/toolchains/rust/nightly-2025-04-17/linux/x86_64/...` |
| Clang | 20.1.0 | `artifacts.systeminit.com/toolchains/clang/20.1.0/linux/x86_64/...` |
| Deno | 2.2.12 | `artifacts.systeminit.com/toolchains/deno/2.2.12/linux/x86_64/...` |

Secondary broken dependencies found during full stack bringup:
- `binaries.prisma.sh` — Prisma engine binaries for NixOS (404 for `linux-nixos` target)
- `module-index.systeminit.com` — production module index (archived, taken down)
- `auth.systeminit.com` — production auth portal referenced in Tiltfile as `https://localhost:9000`

### Phase 3: Build Restoration ✅

**Objective**: Patch broken remote references and iterate until `buck2 run dev:healthcheck` and `buck2 run dev:up` pass.

**Final status**:
- `buck2 run dev:healthcheck` ✅
- `buck2 build @//mode/release //bin/sdf //bin/veritech //bin/edda //bin/pinga //bin/rebaser //bin/luminork //bin/forklift` ✅ (~37 min, 2067 local actions)
- `buck2 run dev:up` / Tilt stack ✅ — all services green

---

#### Fix 1: System Toolchains (replaces all S3 downloads)

**`toolchains/BUCK`** — Completely rewritten to use Buck2 prelude system toolchain rules:

```python
system_cxx_toolchain(name = "cxx", visibility = ["PUBLIC"])
system_python_bootstrap_toolchain(name = "python_bootstrap", visibility = ["PUBLIC"])
system_python_toolchain(name = "python", visibility = ["PUBLIC"])
system_rust_toolchain(name = "rust_release", default_edition = "2024", ...)
deno_toolchain(name = "deno-linux-x86_64", deno_exe = "deno", ...)
si_rust_toolchain(name = "si_rust", ...)
toml_toolchain(name = "toml", ...)
```

**`flake.nix`** — Added `llvmPackages.lld` to `buck2BuildInputs` (Linux). Required because `system_cxx_toolchain` auto-adds `-fuse-ld=lld` and needs `ld.lld` in PATH.

---

#### Fix 2: Deno PATH lookup in build scripts

**Problem**: `prelude-si/deno/deno_run.py`, `deno_binary.py`, `deno_target_runtime.py` all call `pathlib.Path("deno").resolve()` which resolves relative to CWD, not PATH.

**Fix**: Added `resolve_exe()` helper using `shutil.which()`:

```python
def resolve_exe(p: pathlib.Path) -> pathlib.Path:
    if p.parent == pathlib.Path('.'):
        found = shutil.which(p.name)
        if found:
            return pathlib.Path(found)
    return p.resolve()
```

Applied to all three files.

---

#### Fix 3: Prisma engines on NixOS

**Problem**: Prisma 5.20.0 tries to download `linux-nixos` engine binaries from `binaries.prisma.sh` at runtime — 404 since NixOS target was never hosted there.

**Fix**: Use [`nix-prisma-utils`](https://github.com/VanCoding/nix-prisma-utils) to download and patchelf the engine binaries at Nix eval time.

**`flake.nix`** additions:
```nix
inputs.prisma-utils.url = "github:VanCoding/nix-prisma-utils";

# in let block:
prisma = prisma-utils.lib.prisma-factory {
  inherit pkgs;
  versionString = "5.20.0-12.06fc58a368dc7be9fbbbe894adf8d445d208c284";
  hash = "sha256-JPam6PUgSCVXvpSguiGEH6cap4hOODpnNo+vj9+Vvd4=";
};

# shellHook prepended with:
shellHook = prisma.shellHook + ''
  export PRISMA_ENGINES_CHECKSUM_IGNORE_MISSING=1
'' + ...
```

The engine commit hash (`06fc58a3...`) is found in `node_modules/prisma/build/index.js`.

The nix store path (`/nix/store/rx01pji3d00kipjfvxqcqii7din6zbsn-prisma-bin-06fc58a3.../`) is hardcoded in `dev/Tiltfile` because Buck2 subprocesses don't inherit the devshell `shellHook` exports.

**`dev/Tiltfile`** — `auth-db-seed` and `auth-api` both get explicit Prisma env vars:
```python
_prisma_bin = "/nix/store/rx01pji3d00kipjfvxqcqii7din6zbsn-prisma-bin-06fc58a368dc7be9fbbbe894adf8d445d208c284"
# used as prefix on auth-db-seed cmd and in auth-api serve_env
```

---

#### Fix 4: Stripe lazy initialization

**Problem**: `bin/auth-api/src/lib/stripe.ts` instantiates `new Stripe(process.env.STRIPE_API_KEY)` at module load time. No key set for local dev → crash on startup.

**Fix**: Lazy initialization — `getStripe()` only called when a Stripe route is actually hit.

---

#### Fix 5: Local URL fixes in Tiltfile and web .env

**`dev/Tiltfile` line 332**: `"VITE_AUTH_PORTAL_URL": "https://localhost:9000"` → `"http://localhost:9000"`

**`app/web/.env`**:
- Added `VITE_AUTH_PORTAL_URL=http://localhost:9000`
- Switched `VITE_MODULE_INDEX_API_URL` from `https://module-index.systeminit.com` (archived) to `http://localhost:5157`

**`dev/Tiltfile`**: `module-index` changed from `auto_init = False` to `auto_init = True`

---

## Key Lessons

1. **Don't fight NixOS with non-Nix binaries.** Pre-built Linux binaries hardcode `/lib64/ld-linux-x86-64.so.2` and `/usr/lib` RPATHs that don't exist on NixOS. Buck2's hermetic toolchain model assumes standard FHS Linux.

2. **Buck2 system toolchains exist for exactly this.** `system_rust_toolchain`, `system_cxx_toolchain` etc. use whatever is in PATH. Combined with a well-stocked Nix devshell, no downloads are needed.

3. **`system_cxx_toolchain` adds `-fuse-ld=lld` on Linux.** Ensure `llvmPackages.lld` is in the devshell — not just `clang`.

4. **Buck2 subprocesses don't inherit `shellHook` exports.** Environment variables set in `shellHook` are only available in the interactive shell, not in `buck2 run` subprocesses. Pass them explicitly via `serve_env` in Tiltfile or as prefixes on `cmd`.

5. **`pathlib.Path.resolve()` on a bare name resolves to CWD.** On NixOS, tools are on PATH but not in CWD. Always use `shutil.which()` for bare binary names.

6. **`binary_linker_flags` ≠ `linker_flags` in Buck2.** `binary_linker_flags` only applies to final executable linking — proc-macros and shared libs only get `linker_flags`.

7. **Tiltfile hardcoded `https://` URLs break local dev.** Check `serve_env` in Tiltfile for any `https://localhost:*` URLs that should be `http://`.

8. **`module-index` was `auto_init = False`.** Assets, components and functions won't appear in the UI until it's running and pointed to by `VITE_MODULE_INDEX_API_URL`.
