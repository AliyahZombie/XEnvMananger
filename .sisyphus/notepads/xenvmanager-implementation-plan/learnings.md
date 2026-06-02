# Learnings (append-only)

Initial state (2026-03-27):
- Active plan: `.sisyphus/plans/xenvmanager-implementation-plan.md`
- Top-level implementation tasks found in plan: 1,2,3,4,99 (5 tasks)
- Final Verification Wave tasks: F1-F4 (4 tasks)
- Note: plan file contains nested checkboxes in Acceptance Criteria; ignore those for progress counting.
- 2026-03-27: Added fixture bins `src/bin/em_fixture_protocol_ok.rs` and `src/bin/em_fixture_no_protocol.rs` for protocol regression coverage.
- Schema alignment for fixture protocol JSON currently follows repo expectations from `src/config/model.rs` plus task requirements: top-level `version: "1.0"`, `program`, and `env_vars` array; each env var entry uses a lowercase `type` string compatible with existing `EnvVarType` values (`string`, `enum`, `boolean`, `number`) and supplies deterministic defaults.
- Normal fixture execution is intentionally limited to explicit `EM_FIXTURE_*` variables and prints `<unset>` for missing values so tests can assert env injection without exposing secret-like names.
- Verification in this environment required pinning direct Cargo dependencies to exact versions compatible with rustc/cargo 1.75.0 (notably avoiding newer `clap_derive`, `getrandom`, and `unicode-segmentation` releases that require Edition 2024 or newer Rust).

- 2026-03-27: Added Ubuntu GitHub Actions workflow `.github/workflows/ci.yml` that runs `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`.
- 2026-03-27: To keep `clippy -D warnings` green while modules are still scaffolding, introduced a library crate (`src/lib.rs`) and updated `src/main.rs` to use the library (`em::...`) instead of compiling modules directly in the binary. This avoids `dead_code` warnings in the bin crate without adding broad `#[allow(dead_code)]`.
- 2026-03-27: Store tests in `src/config/store.rs` redirect `directories::ProjectDirs` into a temp root by setting `XDG_CONFIG_HOME`/`HOME`/`APPDATA`/`LOCALAPPDATA`, guarded by a global mutex to avoid env-var races when tests run in parallel.
- 2026-03-27: Task 2 fixture follow-up fix was minimal and local to the two bin files: `PRINTABLE_ENV_VARS` was changed from a slice-typed const reference to a fixed array const to satisfy rust-analyzer, and protocol flag detection now compares `args_os()` values against `OsStr::new("--env-manager-protocol")` so the fixtures compile cleanly on the current toolchain.
- 2026-03-27: QA evidence for Task 2 lives under `.sisyphus/evidence/` as `task-2-protocol-ok.json` and `task-2-no-protocol.txt`; create the directory before redirecting command output, because parallel shell steps can race and cause evidence writes to fail even when fixture execution succeeds.

- 2026-03-27: `em --skip <cmd...>` needs to resolve `<cmd0>` for execution/protocol-detection without relying on `$PATH` (e.g., fixture bins live next to `em` under `target/debug/` when run via `cargo run`). A robust approach is: if argv0 has no path separators, check for an executable with that name in `current_exe().parent()` and prefer it; otherwise fall back to the raw argv0.
- 2026-03-27: Protocol parsing treats `default: null` as “required with no default” and must fail the skip run non-interactively by listing only missing var names + types (never values), exiting non-zero.
- 2026-03-27: Integration tests can reliably execute Cargo-built binaries using `env!("CARGO_BIN_EXE_em")` (and related fixture bins) and should sandbox config dirs by setting `XDG_CONFIG_HOME`/`HOME`/`APPDATA`/`LOCALAPPDATA` on the child `Command` rather than mutating the parent test process environment.

- 2026-03-27: Preset discovery uses `em::paths::app_dirs()?.presets_dir()` (via `directories::ProjectDirs`), so tests that need to *write* preset JSON must ensure the parent test process has `XDG_CONFIG_HOME`/`HOME`/`APPDATA`/`LOCALAPPDATA` set to the sandbox root for the duration of the `app_dirs()` call; guard this with a global mutex to avoid env-var races across parallel tests.

- 2026-03-27: Built-in presets live under repo root `presets/` (e.g. `presets/docker.json`, `presets/aws.json`) and are loaded via compile-time embedding (`include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/presets/<name>.json"))`) so `cargo run --bin em -- presets` works without relying on XDG directories.
- 2026-03-27: Built-in preset JSON uses a separate schema (`BuiltInPreset`/`BuiltInEnvVar`) from user presets (`Preset`/`StoredEnvVar`) to keep existing `find_preset()` behavior (used by `--skip` fallback tests) intact.

- 2026-03-27 (F2 Code Quality Review): Local verification passed: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- 2026-03-27 (F2 Code Quality Review): `--skip` runtime path in `src/main.rs` prints missing-required vars as **names + types only** (no values) via `print_missing()`; this behavior is covered by `tests/skip.rs`.
- 2026-03-27 (F2 Code Quality Review): Secret persistence is value-free by construction: `StoredEnvVar::Secret(StoredSecret)` in `src/config/model.rs` stores only `storage` + `keyring_key`, so profile JSON cannot contain secret values.
- 2026-03-27 (F2 Code Quality Review): Keyring integration is currently stubbed (`src/keyring/mod.rs` returns `NotImplemented` for get/set/delete). `resolve_env_vars()` treats keyring errors as “missing secret”, which is safe from a secrecy standpoint but means skip-mode cannot succeed for secret-backed vars until keyring is implemented.
- 2026-03-27 (F2 Code Quality Review): `find_preset()` (`src/presets/mod.rs`) parses each `*.json` file before confirming it matches the requested program; a single invalid/unrelated JSON file in the presets directory will error the whole lookup. Consider deferring parse until after a cheap match check (stem/commands) or treating parse failures as non-fatal for non-matching files.
- 2026-03-27 (F2 Code Quality Review): `rust-analyzer` diagnostics in this environment reported `non_snake_case` warnings on `None` pattern arms in `src/main.rs` and `src/protocol/mod.rs` even though `cargo clippy -D warnings` was clean; treat as a tooling quirk unless reproducible with clippy.

- 2026-03-27 (F2 RE-EVALUATION, scope=tasks 1/2/3/4/99): Verdict APPROVE.
  - Gates: `cargo fmt --all -- --check` exit=0; `cargo clippy --all-targets -- -D warnings` exit=0; `cargo test` exit=0.
  - Within-scope behavior confirmed:
    - `--skip` is non-interactive and follows profile → protocol → preset → plain exec ordering (`src/main.rs`).
    - Protocol parsing rejects unknown types, version mismatch, program mismatch; `default: null` treated as required missing (`src/protocol/mod.rs`).
    - Missing-required reporting prints only var name + type (`print_missing`), no values; integration test asserts this (`tests/skip.rs`).
    - Preset fallback warning is one-line and value-free; integration test asserts exact warning and injected outputs (`tests/skip_preset.rs`).
    - Built-in presets list prints only IDs (`em presets` prints `BuiltInPreset.id`). Preset JSON includes `secret` types but no secret values/defaults (`presets/aws.json`, `presets/docker.json`).
  - Prior REJECT items reclassified as non-blocking for this scope:
    - Keyring stubs (`src/keyring/mod.rs`) do not break tasks 1/2/3/4/99 tests and do not cause secret leakage/persistence; they only make secret resolution unavailable until implemented.
    - User preset parse strictness (`src/presets/mod.rs::find_preset`) could be hardened, but current plan tasks do not require tolerating unrelated malformed JSON files.
