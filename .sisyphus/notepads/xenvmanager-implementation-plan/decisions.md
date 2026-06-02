# Decisions (append-only)

Initial decisions (2026-03-27):
- Treat only top-level checkboxes under `## TODOs` and `## Final Verification Wave` as executable tasks.

## F4. Scope Fidelity Check - 2026-03-26

### VERDICT: APPROVE

All changes map cleanly to the 5 planned tasks (1, 2, 3, 4, 99). No scope creep detected.

### File-to-Task Mapping

**Task 1: CI + unit tests baseline**
- `.github/workflows/ci.yml` - CI workflow (fmt/clippy/test on Ubuntu)
- `Cargo.toml` - Added dev-dependencies (assert_cmd, predicates, tempfile)
- `tests/skip.rs` - Integration tests for --skip with protocol fixtures
- `tests/skip_preset.rs` - Integration test for preset fallback
- `src/cli/mod.rs` - Unit tests for clap parsing (skip flag, external subcommand)
- `src/config/store.rs` - Unit tests for save/load/list + 0600 permissions

**Task 2: Fixture bins (protocol compatible/incompatible)**
- `src/bin/em_fixture_protocol_ok.rs` - Protocol v1.0 compatible fixture
- `src/bin/em_fixture_no_protocol.rs` - Non-protocol fixture
- `src/bin/em_fixture_protocol_missing_required.rs` - Protocol fixture with missing required vars

**Task 3: --skip non-interactive execution path**
- `src/main.rs` - Implements run_skip() with profile→protocol→preset→manual fallback chain
- `src/protocol/mod.rs` - Protocol detection and parsing logic
- `src/executor/mod.rs` - Process execution with env injection
- `src/keyring/mod.rs` - Keyring interface (stub for Phase 1)
- `src/paths.rs` - XDG directory resolution
- `src/config/model.rs` - EnvVar types and schema
- `src/config/store.rs` - Profile persistence with 0600 permissions
- `src/lib.rs` - Library root exposing modules

**Task 4: Preset fallback integration test**
- `tests/skip_preset.rs` - Validates protocol failure → preset fallback with warning

**Task 99: Built-in presets (docker + aws) + `em presets` command**
- `presets/docker.json` - Docker preset with 23 env vars (includes secrets)
- `presets/aws.json` - AWS CLI preset with 23 env vars (includes secrets)
- `src/presets/mod.rs` - Preset loader + built-in preset system with separate schema
- `src/main.rs` - `em presets` command implementation

**Evidence artifacts (not runtime code)**
- `.sisyphus/evidence/task-*.{txt,json}` - QA scenario outputs (7 files)

**Project infrastructure (expected)**
- `.gitignore` - Standard Rust ignores
- `Cargo.lock` - Dependency lock file
- `.sisyphus/*` - Plan/notepad/evidence files (orchestrator-managed)

### Scope Compliance Notes

1. **No TUI implementation**: Correctly deferred per plan (Wave 3+). Only stubs present.
2. **Minimal --skip path**: Implements exactly the 4-step fallback specified in Task 3.
3. **Preset schema separation**: Built-in presets use `BuiltInPreset` schema (separate from user `Preset`), avoiding conflicts with profile fallback logic.
4. **No extra subcommands**: Only `list/show/edit/delete/reset/presets` + external Run - all from plan.
5. **No keyring integration**: Correctly stubbed for Phase 1 (Linux-only MVP).
6. **Test coverage**: Matches plan's "tests-after" strategy with focused unit + integration tests.

### No Scope Creep Items

All 38 new/modified files serve the 5 completed tasks. No unrelated features, refactors, or documentation beyond evidence artifacts.

