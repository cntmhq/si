# Experiment Log

Progress tracking for the SI build forensics project.

## Phases

### Phase 0: Project Initialization ✅

- Created initial `CLAUDE.md` for the repository by having Claude analyze the repo structure.
- The generated document was extensive and covers high-level repo layout.

### Phase 1: Deep Repository Analysis 🔄

**Objective**: Ingest and summarize all build/deployment manifests so Claude has a working mental model of the pipeline.

**Key challenges**:

- Bazel files total ~100k lines (80k code, 10k comments, 10k blanks).
- BUCK files total ~30k lines.
- Neither fits in a single context window.

**Strategy**:

1. Use `fd` and `rg` to locate and triage files by relevance.
2. Read in batches, summarize findings into `CLAUDE.md` memory files placed in each subdirectory.
3. Use Plan Mode to coordinate the analysis before executing.
4. Focus on the dependency graph rooted at `dev:healthcheck` and `dev:up` — ignore unrelated build targets.

### Phase 2: Dependency Cataloguing (not started)

Systematically catalogue all broken remote references (S3 artifacts, dead URLs) across the build manifests, documented in per-directory `CLAUDE.md` files with enough detail to plan replacements.

### Phase 3: Build Restoration (not started — future skill)

Using the `CLAUDE.md` memory files from phases 1–2, patch broken remote references and iterate until `buck2 run dev:healthcheck` and `buck2 run dev:up` both pass.
