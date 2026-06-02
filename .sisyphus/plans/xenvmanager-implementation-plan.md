# XEnvManager（em）环境变量管理器 — 技术实现计划（Phase 1-4 全量细化）

## TL;DR
> **Summary**: 实现一个 TUI-first 的环境变量管理器 `em`：可为任意程序维护可复用的 env profile，secret 自动进 Linux keyring，并以注入 env 的方式启动目标程序；逐步加入 presets、protocol、以及 Phase 4 的增强能力。
> **Deliverables**:
> - 可用的 `em` CLI（含 list/show/edit/delete/reset/presets/protocol/export/import 等）
> - Profile JSON 存储（0600）+ secret keyring 存取（不落盘）
> - Ratatui TUI：编辑/校验/保存/运行
> - Presets 系统：内置 + 用户自定义
> - Protocol v1.0：`--env-manager-protocol` 检测与解析 + 自动模式选择
> - Phase 4：高级 TUI、模板变量、导入导出、团队模板（不含真实 secret）
> **Effort**: XL
> **Parallel**: YES — 6-8 waves
> **Critical Path**: Profile/paths+store 稳定化 → keyring+executor → TUI 编辑/校验/保存/运行 → presets → protocol → Phase 4 增强

## Context
### Original Request
- 用户提供了完整的产品/协议/目录/CLI/TUI 草案，并要求“细化 plan”。
- 约束/选择：
  - 覆盖 Phase 1-4
  - Phase 1 只保证 **Linux**
  - MVP 默认 **TUI**（`em <cmd>` 进入 TUI；`--skip/-s` 使用上次配置直接运行）
  - protocol 检测失败：**Warn + fallback**
  - preset 匹配：**Exact command**（仅按 argv0 精确匹配）

### Interview Summary
- Repo 已有模块骨架与部分实现：`src/config/model.rs` + `src/config/store.rs` 已实现 profile schema 与原子写入/0600；其余（tui/keyring/executor/presets/protocol）基本为 stub。
- 当前无 tests/ 与 CI。

### Metis Review (gaps addressed)
> 待 Metis 输出补全：遗漏问题、护栏、边界条件、验收标准补强。

## Work Objectives
### Core Objective
- 交付一个可在 Linux 上稳定工作的 `em`：
  1) 识别目标程序（Run）
  2) 通过 TUI/预设/协议/手动模式生成或更新 profile
  3) secret 仅存 keyring，profile 仅存 keyring key
  4) 注入 env 并启动目标程序（stdin/stdout/stderr 透传）

### Deliverables
- `em` 可执行文件（Rust binary crate）
- Profile & Preset JSON schema（serde）
- 协议解析器（Protocol v1.0）
- TUI：编辑与校验 UI（基础 + Phase 4 增强）
- 质量门禁：fmt/clippy/test + 最小 CI（Ubuntu）

### Definition of Done (agent-verifiable)
- `cargo fmt --all -- --check` 通过
- `cargo clippy --all-targets -- -D warnings` 通过
- `cargo test` 通过
- 关键场景通过（见每个 TODO 的 QA Scenarios），并产出证据文件：`.sisyphus/evidence/task-*-*.{md,txt,json,png}`

### Must Have
- Linux profile 目录与文件权限正确（0600）
- Secret 不写入 profile JSON、不输出到日志
- `--skip/-s` 仅在可用 profile 时跳过 TUI；否则回落到交互
- protocol 检测失败时警告 + fallback

### Must NOT Have (guardrails)
- 不得将 secret 以明文写入：profile JSON / stdout / stderr / panic 报错
- 不得污染父进程环境（只给子进程注入 env）
- 不得把 argv0 未校验直接用作路径（防止 path traversal）
- 不得在 CI 中依赖真实桌面 keyring（Phase 1 仅做接口可测 + 可选手工验证）

## Verification Strategy
> ZERO HUMAN INTERVENTION — all verification is agent-executed.
- Test decision: **tests-after**（先实现核心逻辑，再补齐单元/集成测试）
- Unit tests focus:
  - `src/config/store.rs`：save/load/list + 0600（`#[cfg(unix)]`）
  - `src/cli/mod.rs`：clap parse（不 spawn 进程）
- Integration tests（可选最小）：binary `--help`/`list` smoke
- 运行行为回归：引入 2 个 fixture 程序（协议兼容/不兼容）+ profile fixture，用 `assert_cmd` 覆盖 `em protocol` 与 `em --skip` 的注入与执行。
- 证据产出：每个任务写入 `.sisyphus/evidence/`（文本/JSON/截图）

## Execution Strategy
### Parallel Execution Waves
Wave 1: 基础设施与“可测试”内核（错误体系/paths/store/CI/tests/CLI wiring）
Wave 2: Executor + Keyring 接口（可 mock）
Wave 3: Phase 1 基础 TUI（状态机 + 编辑控件 + 保存/运行流）
Wave 4: Phase 2 Presets（schema + 内置 presets + 管理命令）
Wave 5: Phase 3 Protocol（runner + schema + validation + profile 生成/更新）
Wave 6: 自动模式选择整合（profile→protocol→preset→manual）+ UX polish
Wave 7: Phase 4 高级 TUI（搜索/过滤/历史/对比）
Wave 8: Phase 4 模板变量 + 导入导出 + 团队模板（占位符）+ 平台扩展（可选）

### Dependency Matrix (full, all tasks)
> 在 TODOs 完整列出后补齐：Task N 的 Blocks / Blocked By。

### Agent Dispatch Summary
> 计划执行时由 Sisyphus 按 wave 并行分发；每个任务都包含推荐 agent category。

## TODOs
> Implementation + Test = ONE task. Never separate.
> EVERY task MUST have: Agent Profile + Parallelization + QA Scenarios.

<!-- TODO items will be appended here in batches. -->

- [x] 1. 建立测试与 CI 基线（Ubuntu）

  **What to do**:
  - 新增 GitHub Actions workflow（Ubuntu）运行：`cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test`。
  - 在 `Cargo.toml` 增加必要的 `[dev-dependencies]`（建议：`assert_cmd`, `predicates`；如需 temp 目录用现有 `tempfile`）。
  - 添加最小单元测试：
    - `src/config/store.rs`: save/load/list + `#[cfg(unix)]` 断言 0600
    - `src/cli/mod.rs`: clap parse（包含 `external_subcommand` 与 `--skip`）

  **Must NOT do**:
  - 不在 CI 里依赖真实 keyring。

  **Recommended Agent Profile**:
  - Category: `unspecified-high` — Reason: CI + Rust test infra 需要细致验证
  - Skills: []

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: [2,3,4] | Blocked By: []

  **References**:
  - Pattern: `src/config/store.rs` — 原子写入 + 0600 逻辑
  - API/Type: `src/cli/mod.rs` — clap CLI 定义

  **Acceptance Criteria**:
  - [ ] GitHub Actions 在 Ubuntu 上通过 fmt/clippy/test
  - [ ] `cargo test` 覆盖 store + cli 基础用例

  **QA Scenarios**:
  ```
  Scenario: CI gate passes locally
    Tool: Bash
    Steps:
      1) cargo fmt --all -- --check
      2) cargo clippy --all-targets -- -D warnings
      3) cargo test
    Expected: 全部退出码 0
    Evidence: .sisyphus/evidence/task-1-ci-gates.txt
  ```

  **Commit**: YES | Message: `chore(ci): add fmt clippy test gates` | Files: [Cargo.toml, .github/workflows/*, tests/*]

- [x] 2. 添加 fixture 程序（协议兼容 / 不兼容）用于行为回归

  **What to do**:
  - 在 workspace 内新增两个可执行 fixture（建议 `src/bin/`）：
    - `em_fixture_protocol_ok`: 支持 `--env-manager-protocol` 输出 Protocol v1.0 JSON；正常运行时打印若干 env 值（用于断言注入生效）。
    - `em_fixture_no_protocol`: 不支持协议（对该 flag 返回非 0 或输出非 JSON）；正常运行时同样打印 env 值。
  - 约束：fixture 不接触真实 keyring；只使用非 secret env（或 secret 用占位且不输出）。
  - Protocol OK 的 schema 设计为“可无交互运行”：所有 required 字段都有默认值（避免测试需要 TUI）。

  **Must NOT do**:
  - 不要在 fixture 输出中打印任何可能被视为 secret 的字段值（即使是测试用）。

  **Recommended Agent Profile**:
  - Category: `quick` — Reason: 小型 Rust bin + 明确输出契约
  - Skills: []

  **Parallelization**: Can Parallel: YES | Wave 1 | Blocks: [3,4] | Blocked By: []

  **References**:
  - Protocol: 计划中 Protocol v1.0 JSON schema（后续任务 6 会正式实现解析；fixture 先按该 schema 输出）

  **Acceptance Criteria**:
  - [ ] `cargo run --bin em_fixture_protocol_ok -- --env-manager-protocol` 输出合法 JSON
  - [ ] `cargo run --bin em_fixture_no_protocol -- --env-manager-protocol` 退出码非 0 或输出非 JSON（可用于 fallback 测试）

  **QA Scenarios**:
  ```
  Scenario: Protocol OK fixture prints protocol JSON
    Tool: Bash
    Steps: cargo run --bin em_fixture_protocol_ok -- --env-manager-protocol
    Expected: stdout 是 JSON，包含 version=1.0 program=... env_vars=[...]
    Evidence: .sisyphus/evidence/task-2-protocol-ok.json

  Scenario: No-protocol fixture fails protocol flag
    Tool: Bash
    Steps: cargo run --bin em_fixture_no_protocol -- --env-manager-protocol
    Expected: 退出码 !=0 或 stdout 不是 JSON
    Evidence: .sisyphus/evidence/task-2-no-protocol.txt
  ```

  **Commit**: YES | Message: `test(fixtures): add protocol and non-protocol binaries` | Files: [src/bin/*]

- [x] 3. 定义 `--skip/-s` 的“无 TUI”语义并实现可测试的运行路径

  **What to do**:
  - 明确定义并实现：`em --skip <cmd...>` 绝不进入 TUI。
  - 行为顺序（无 UI）：
    1) 若存在 profile：加载→解析 env（含 keyring 引用）→执行
    2) 否则尝试协议检测（`<cmd0> --env-manager-protocol`）：成功则用默认值构建临时配置（并可选择写入 profile）→执行
    3) 协议失败：stderr 输出一行 warning → 尝试 preset（若存在且 defaults 足够）→执行
    4) 若仍缺少 required：打印“缺少哪些变量（仅名称/类型）”并以非 0 退出
  - 注意：preset 匹配按 argv0 精确匹配。

  **Must NOT do**:
  - 不要在 `--skip` 路径里做交互输入（包括读取 stdin 作为 prompt）。

  **Recommended Agent Profile**:
  - Category: `unspecified-high` — Reason: 这是可测试性与 UX 的关键 contract
  - Skills: []

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: [4,5,6] | Blocked By: [1,2]

  **References**:
  - CLI: `src/cli/mod.rs`（已存在 `--skip`）
  - Store: `src/config/store.rs`
  - Paths: `src/paths.rs`（XDG sandbox 需可测）

  **Acceptance Criteria**:
  - [ ] `em --skip <cmd>` 在无 profile 且协议 OK 且 defaults 完整时可直接运行（无 TUI）
  - [ ] 协议失败时会 warning + fallback（不含 secret）

  **QA Scenarios**:
  ```
  Scenario: Skip runs protocol-ok via defaults
    Tool: Bash
    Steps:
      1) export XDG_CONFIG_HOME=$(mktemp -d)
      2) cargo run --bin em -- --skip em_fixture_protocol_ok
    Expected: 退出码 0；stdout 包含 fixture 打印的默认 env 值
    Evidence: .sisyphus/evidence/task-3-skip-protocol-ok.txt

  Scenario: Skip fails with missing required vars
    Tool: Bash
    Steps: 运行一个协议 schema 中 required 且无 default 的 fixture（或临时参数控制）
    Expected: 退出码 !=0；stderr 列出缺失变量名（不含值）
    Evidence: .sisyphus/evidence/task-3-skip-missing-required.txt
  ```

  **Commit**: YES | Message: `feat(cli): make --skip non-interactive auto-run` | Files: [src/main.rs, src/*]

- [x] 4. 为“不兼容协议”的程序提供 preset 配置文件 fixture，并用集成测试验证 fallback + 注入与执行

  **What to do**:
  - 新增一个 preset JSON fixture（放在项目内置 presets 目录，或测试中写入 `presets_dir()`），内容：
    - `name`: `fixture-no-protocol`
    - `commands`: ["em_fixture_no_protocol"]（按 argv0 精确匹配）
    - `env_vars`: 至少包含 string/number/boolean/enum/path 的代表字段
    - 不包含 secret（避免 keyring 依赖）
  - 集成测试：
    - 设置 `XDG_CONFIG_HOME` 到 temp
    - 将 preset 写入 `presets_dir()`
    - 运行 `em --skip em_fixture_no_protocol`
    - 断言 stderr 出现“protocol 检测失败 warning”（不含值）
    - 断言 stdout 中 fixture 输出包含 preset 提供的 env 值

  **Recommended Agent Profile**:
  - Category: `unspecified-high` — Reason: 路径隔离 + profile 写入 + assert_cmd 调用
  - Skills: []

  **Parallelization**: Can Parallel: YES | Wave 2 | Blocks: [5] | Blocked By: [1,2,3]

  **References**:
  - Paths: `src/paths.rs`（`presets_dir()`）
  - Preset loader: Phase 2 presets 系统（实现后需能加载该 fixture preset）

  **Acceptance Criteria**:
  - [ ] `cargo test` 中集成测试可稳定通过（不依赖桌面 keyring）

  **QA Scenarios**:
  ```
  Scenario: Skip runs no-protocol fixture via preset fallback
    Tool: Bash
    Steps: cargo test -q --test skip_runs_preset_fixture
    Expected: 退出码 0；测试断言通过
    Evidence: .sisyphus/evidence/task-4-integration-skip-preset.txt
  ```

  **Commit**: YES | Message: `test(integration): run with profile fixture` | Files: [tests/*]

- [x] 99. （最终新增）研究 2 个热门项目并生成内置 presets

  **What to do**:
  - 选择 2 个热门项目（决策固定）：**Docker** 与 **AWS CLI**。
  - 调研它们的常用环境变量：名称、类型（secret/enum/path/...）、说明、默认值（如有）。
  - 生成内置 presets JSON（存放在项目内的 presets 目录，供运行时加载）。
  - 为每个 env var 在 preset 中标记 `type` 与 `required`，并把 `AWS_SECRET_ACCESS_KEY` 等标成 `secret`。
  - 引用官方文档链接（写到 preset 的 metadata 或作为注释/README 记录；实现时按项目规范落地）。

  **Recommended Agent Profile**:
  - Category: `writing` — Reason: 需要资料核对与结构化输出
  - Skills: []

  **Parallelization**: Can Parallel: YES | Wave 8 | Blocks: [] | Blocked By: [Phase 2 presets 系统完成]

  **Acceptance Criteria**:
  - [ ] `em presets` 能列出 docker 与 aws 两个内置 preset
  - [ ] preset 的字段可被 loader 解析并在 TUI/skip 路径使用

  **QA Scenarios**:
  ```
  Scenario: List built-in presets includes docker/aws
    Tool: Bash
    Steps: cargo run --bin em -- presets
    Expected: 输出包含 docker 与 aws
    Evidence: .sisyphus/evidence/task-99-presets-list.txt
  ```

  **Commit**: YES | Message: `feat(presets): add docker and aws presets` | Files: [presets/*]

## Final Verification Wave (MANDATORY — after ALL implementation tasks)
> 4 review agents run in PARALLEL. ALL must APPROVE.
- [x] F1. Plan Compliance Audit — oracle
- [x] F2. Code Quality Review — unspecified-high
- [x] F3. Real Manual QA — unspecified-high (+ playwright if UI)
- [x] F4. Scope Fidelity Check — deep

## Commit Strategy
- 建议按 wave/模块拆分为原子提交（实现+测试同一提交），遵循 Conventional Commits：
  - `feat(cli): ...`, `feat(config): ...`, `feat(tui): ...`, `feat(protocol): ...`, `feat(presets): ...`, `chore(ci): ...`, `test(config): ...`
- 每个提交必须：不包含 secret；`cargo fmt/clippy/test` 通过。

## Success Criteria
- Phase 1（Linux MVP）可用：TUI 配置→保存→运行；`--skip` 复用 profile；secret 进 keyring；profile 0600。
- Phase 2 可用：presets 自动匹配并可管理。
- Phase 3 可用：protocol 检测/解析/校验+自动模式选择。
- Phase 4 可用：高级 TUI、模板变量、导入导出、团队模板（不含 secret）。
