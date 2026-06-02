# Draft: XEnvManager 技术实现计划（细化）

## Requirements (confirmed)
- 统一环境变量管理工具：通过 TUI 简化配置，支持安全凭证管理和配置复用。
- 语言 Rust；TUI: ratatui；凭证: keyring-rs；配置存储: serde + JSON；执行: std::process::Command。
- 两种配置来源：预设（presets）与协议（protocol v1.0：`--env-manager-protocol` 输出 JSON）。
- Secret 类型必须进系统 keyring；Profile 文件权限 600；不在日志输出 secret；环境变量仅传给子进程。
- Phase 1 (MVP)：CLI 框架、配置管理、keyring 集成、基础 TUI、程序执行。
- Phase 2：预设系统 + 管理命令。
- Phase 3：协议检测/解析 + 自动模式选择；SDK 后续。
- Phase 4：高级 TUI / 模板变量 / 导入导出 / 协作。

## Technical Decisions (initial)
- CLI 命令集合以用户给出的为准（`em <program>`, `-s/--skip`, list/show/edit/delete/reset, presets, protocol, version/help）。
- 配置目录遵循 XDG（Linux/macOS）与 Windows 等效目录（需在代码中统一抽象）。
- Profile JSON schema 以文档为目标形态，但需结合当前 repo 代码结构落地（待探索确认）。

## Requirements (confirmed) — user choices
- 计划范围：覆盖 Phase 1-4（全部细化到可执行任务）。
- 目标平台：Phase 1 先保证 Linux（macOS/Windows 作为后续任务纳入 Phase 4 或“可选增强”）。
- 交互入口：MVP 默认 TUI（`em <cmd>` 进入 TUI；`--skip`/`-s` 直接用上次配置运行）。

## Research Findings (pending)
### Repo 现状（explore）
- 现有模块基本已按目标架构落位：`src/cli`, `src/config`, `src/keyring`, `src/executor`, `src/tui`, `src/presets`, `src/protocol`, `src/paths.rs`, `src/main.rs`。
- `src/config/model.rs` 已实现 Profile/EnvVar JSON schema（serde tagged enum），`src/config/store.rs` 已实现 profile 的原子写入、Unix 600 权限、list/load/save。
- `src/cli/mod.rs` 已实现 clap derive + `external_subcommand`（Run(Vec<OsString>)) + 全局 `--skip`。
- `src/keyring/mod.rs`/`src/executor/mod.rs`/`src/tui/mod.rs`/`src/presets/mod.rs`/`src/protocol/mod.rs` 目前是 stub。
- `Cargo.toml` 已有：clap/serde/serde_json/directories/tempfile/thiserror/color-eyre/ratatui/crossterm；但**尚未加入 keyring crate**。

### 测试与 CI（explore）
- 当前无 tests/ 目录、无 `#[test]`，`Cargo.toml` 无 `[dev-dependencies]`。
- 无 CI（未发现 `.github/workflows/*`），无 rustfmt/clippy 配置文件。
- 推荐 Phase 1 最小质量门：`cargo fmt --all -- --check` + `cargo clippy --all-targets -- -D warnings` + `cargo test`。

## Open Questions
- （已确认）目标平台优先级：Phase 1 先 Linux。
- （已确认）TUI 是否为默认入口：是。
- protocol 检测失败（命令不存在/exit!=0/输出非 JSON）时的用户体验：静默降级还是提示？
- 预设匹配逻辑：按 argv[0] 名称匹配、还是支持子命令/别名（如 `kubectl` vs `k`）？

## Technical Decisions (confirmed)
- protocol 检测失败：**Warn + fallback**（stderr 一行提示 + 降级到 preset/manual；详细诊断放到 `em protocol <cmd>`）。
- preset 匹配：**Exact command**（仅按 argv[0] 精确匹配；不做启发式）。

## Scope Boundaries
- INCLUDE: Phase 1-3 的实现路径与可执行验证策略（tests/QA/evidence）。
- EXCLUDE: SDK、多端同步、Web UI、IDE 插件（除非明确要求纳入计划）。

## New Requirements (added later)
- 加几个测试：用两类“fixture 程序”覆盖运行行为与协议/非协议分支。
  - 程序 A：兼容协议（支持 `--env-manager-protocol` 输出 JSON）。
  - 程序 B：不兼容协议（不支持协议），并提供可用的配置文件（**preset JSON fixture**）。
- 通过自动化测试检查：
  - `em protocol <cmd>` 行为（可解析/不可解析）
  - `em --skip <cmd>` 的运行行为（无 TUI、env 注入、生效、退出码/输出）
- 全部完成后：新增 TODO（研究 1-2 个热门项目，查资料生成 presets）。
