# XEnvManager - `em`

> 在终端里轻松发现、配置、安全保存并自动注入 CLI 所需的环境变量。

**中文** | [English](./README.en.md)

## 为什么需要它？

很多终端工具真正的门槛不在命令本身，而是运行前那一堆繁杂的环境变量。

想象一下，你刚准备跑起一个酷炫的 CLI，却被这些琐事绊住了脚：

- 它到底需要哪些环境变量？翻遍了文档、Issue，甚至别人的 dotfiles 才找到。
- 变量名是找到了，可还得手动 `export`、改 shell rc，或者在命令前临时拼凑一长串 `FOO=... BAR=... program ...`。
- Token、密码、访问密钥这类敏感信息，一不小心就留在了 shell 历史记录或明文配置文件里。
- 同一个程序换个子命令，需要的环境变量就不一样了，切来切去不仅心累还容易出错。

`em` 的出现就是为了把这些麻烦事变简单。它把「查变量名 -> 配环境 -> 安全保存 secret -> 注入环境并运行」整个过程，丝滑地收拢进一个终端内的可视化编辑流程中。它能智能地从你保存的 profile、内置预设，甚至是目标程序主动提供的 XEnvManager 协议里读取环境变量结构（schema），然后直接在 TUI 界面里让你填值、保存、反复使用。

## 基本用法

```bash
em opencode web
```

敲下这行命令，就会弹出一个优雅的环境配置 TUI。注意，默认情况下 `em <program>` 只会打开编辑器，并不会立刻运行目标程序。你可以在界面里从容地检查有哪些变量、补上缺少的值，确认无误后按 `s` 单独保存配置（profile），或者按 `r` 保存并直接运行。

以后再跑同样的命令，就不需要再配置一遍啦：

```bash
em --skip opencode web
```

加上 `--skip` 参数后，`em` 会在后台默默加载你保存好的 profile，注入环境变量，然后直接启动目标程序。如果它发现对应的 profile 还不存在，就会乖乖退出并提示你。

## 安装与构建

你需要安装 Rust stable 工具链。本项目使用的是 Rust 2021 edition。

```bash
cargo build --release
./target/release/em --help

cargo run -- opencode web

cargo install --path .
```

特别提示：在 Linux 系统上，如果想把 secret 安全地存进系统密钥环，你需要一个处于 DBus 会话中且可用的 Secret Service 后端（比如 gnome-keyring 或 KWallet）。

## 命令速查

| 命令 | 作用 |
| --- | --- |
| `em <program> [args...]` | 为指定命令打开环境配置 TUI。 |
| `em --skip <program> [args...]` | 跃过配置界面，直接使用已保存的 profile 注入环境并运行目标程序。 |
| `em --protocol <program> [args...]` | 打开 TUI，并强制优先使用目标程序自身的协议 schema 来预填信息。 |
| `em --presets` | 打开用户预设管理的 TUI 界面。 |
| `em --preset-list` | 看看有哪些内置的预设 ID。 |
| `em --preset-user` | 列出你自己的用户预设文件。 |
| `em --preset-dir` | 打印出存用户预设的文件夹路径。 |
| `em --preset-init <program> [--preset-subcommand <sub>] [--include-secrets] [--force]` | 把内置预设导出成你的自定义预设，方便魔改。 |
| `em --keyring-set <KEY>` | 从 stdin 读取 secret 并塞进系统的密钥环里。 |
| `em --keyring-delete <KEY>` | 把存在密钥环里的 secret 删掉。 |
| `em --keyring-has <KEY>` | 检查 secret 在不在（存在则 exit 0，否则 exit 1）。 |

什么都不带直接运行 `em`，就能看到完整的帮助信息。

## 在 TUI 中游刃有余

| 按键 | 操作 |
| --- | --- |
| `↑` / `↓` | 穿梭在变量之间 |
| `Enter` | 编辑当前变量；布尔值直接切开关，secret 则会贴心地弹出带遮罩的输入框 |
| `s` | 保存 profile 并优雅退出 |
| `r` | 保存并一键运行（运行前会帮你检查必填项有没有落下） |
| `q` / `Esc` | 挥挥衣袖，不带走一片云彩（不保存直接退出） |

行首显眼的红色 `*` 在提醒你这是必填项。输入 secret 时，所有的字符都会被替换成 `*` 保护起来。界面顶部还会清晰地标出当前的数据来源到底是 `saved`、`preset`、`protocol` 还是 `empty`，这份变量清单的来历一目了然，绝不让你猜谜。

数据预填的优先级也很清晰：

- 默认情况：保存的 profile -> 内置预设 -> 目标协议 -> 啥也没有 (empty)
- 带上 `--protocol` 时：保存的 profile -> 目标协议 -> 内置预设 -> 啥也没有 (empty)

## Profile 与存储路径

`em` 很聪明，它会根据程序名甚至是二级子命令来分别保存你的 profile：

- 运行 `em opencode` -> 对应 profile 名 `opencode`
- 运行 `em opencode web` -> 对应 profile 名 `opencode.web`

这里的“二级子命令”，指的就是目标程序名后面跟着的第一个“非 flag”参数。为了安全，用作文件名的 key 都会经过严格编码。

配置文件的安身之所由系统习惯决定（基于 `directories::ProjectDirs::from("io", "xenvmanager", "em")`），绝不乱丢垃圾：

| 操作系统 | 典型的配置目录 |
| --- | --- |
| Linux | `$XDG_CONFIG_HOME/em`（通常是 `~/.config/em`） |
| macOS | `~/Library/Application Support/io.xenvmanager.em` |
| Windows | `%APPDATA%\xenvmanager\em\config` |

Profile 具体存放在 `<config_dir>/profiles/<key>.json`，你自己捏的预设存放在 `<config_dir>/presets/<name>.json`。在 Unix 系统上，这些文件写入时都会严格控制权限为 `0600`。

## 守护你的 Secret：Keyring

`em` 优先把 secret 交给系统的 DBus Secret Service 保管（Service 标识是 `io.xenvmanager.em`）。当系统的密钥环可用时，你的 profile 文件里只会留下一串引用 ID，真正的 token 和密码都锁在极其安全的系统密钥环里。

万一系统密钥环罢工了，TUI 会非常直白地警告你，并把选择权交给你：你可以选择以明文把 secret 记在 profile 文件里。虽然在 Unix 上文件权限依然被死死限制在 `0600`，但这终究比不上真正的密钥环安全，还请三思。

```bash
printf '%s' 's3cr3t' | em --keyring-set opencode:OPENCODE_SERVER_PASSWORD
em --keyring-has opencode:OPENCODE_SERVER_PASSWORD
em --keyring-delete opencode:OPENCODE_SERVER_PASSWORD
```

## 预设 (Presets)：站在巨人的肩膀上

预设其实就是一份为你准备好的“开箱即用”变量清单，专治各种“不知道这 CLI 到底要配啥”的症状。

目前内置了这些常客：

- Docker
- AWS CLI
- OpenCode

如果你自己写了用户预设，它的优先级永远高于内置预设。而且，精确匹配到子命令（`program + subcommand`）的预设，会比只匹配主程序（`program`）的预设优先被使用。

```bash
em --preset-list
em --preset-user
em --preset-dir
em --presets
em --preset-init opencode --preset-subcommand web --include-secrets
```

当 `em` 用内置预设帮你自动填表时，那些不是 secret 且带有默认值的变量，以及存放在密钥环里的 secret 引用，都会一并带上。比如，OpenCode 的 `OPENCODE_SERVER_PASSWORD` 就只会在你使用 `web` 子命令时才悄悄出现。

## 给 CLI 开发者：接入 XEnvManager

如果你恰好在维护一个 CLI 工具，并且也希望你的用户能免去翻文档的折磨，一眼看穿该配什么环境变量，那么实现 XEnvManager 协议绝对是个好主意。

接入后，当用户敲下：

```bash
em your-cli serve
```

`em` 会在背后偷偷探测一下：

```bash
your-cli serve --env-manager-protocol
```

只要你的程序在这个特殊模式下吐出一段 JSON schema，`em` 就能把变量名、类型、默认值、必填项统统吸纳，然后丝滑地展示在配置 TUI 里。

### 你的 CLI 适合接入吗？

如果你的 CLI 有以下特质，那简直是天作之合：

- 运行极度依赖 API key、token、endpoint、region、配置路径这些环境变量。
- 不同的子命令（比如 `serve`、`deploy`、`web`）需要的环境千差万别。
- 用户老是在 Issue 里问“到底要设什么环境变量”，或者总把密码明文写在不该写的地方。
- 你只想提供一个极轻量的“元数据发现入口”，绝不想改变 CLI 原本的任何运行逻辑。

### 协议基本法

请记住，协议只是用来发现 Schema 和默认值的，它**绝不是**真正的运行入口。当你的程序收到 `--env-manager-protocol` 时：

- stdout 必须、且只能输出合法的 JSON，别加多余的 log。
- 进程跑完必须乖乖返回成功（exit 0）。
- 动作要快，必须在 10 秒内完事。
- 绝对不要弹任何交互输入，更不要去修改用户的本地状态。
- `version` 字段焊死为 `"1.0"`。
- `program` 必须与用户传给 `em` 的主程序名完全一致。
- 保护好 secret，它的 `default` 推荐使用 `null`；空字符串会按未设置处理，非空默认值仍会被拒绝。

最精简的 JSON 长这样：

```json
{
  "version": "1.0",
  "program": "your-cli",
  "env_vars": [
    {
      "name": "YOUR_CLI_API_KEY",
      "type": "secret",
      "default": null
    },
    {
      "name": "YOUR_CLI_REGION",
      "type": "string",
      "default": "us-east-1"
    }
  ]
}
```

目前支持的类型有：`secret`、`string`、`number`、`boolean`、`enum` 和 `path`。

### 最佳实践建议

请把这个协议模式当成一个只读的 REST Endpoint：它必须极速响应、极度稳定、绝无副作用。
如果你的 CLI 拥有丰富的子命令，建议复用正常运行时的参数解析逻辑来决定返回哪些 schema。比如，当探测 `your-cli web --env-manager-protocol` 时，就只返回 `web` 模式专属的环境变量。

开发完成后，不妨这样验证一下你的心血：

```bash
your-cli web --env-manager-protocol
em --protocol your-cli web
em your-cli web
```

想了解完整的字段规则、失败怎么回退，或是想要抄一份 Rust 的实现示例？请移步 [protocol.md](./protocol.md)。

## 开发

```bash
cargo build --release
cargo run -- <program> [args...]
cargo test
```

我们的 CI 守门员会认真执行 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings` 以及 `cargo test`。

项目主要仰仗这些优秀的库：`clap`、`serde` / `serde_json`、`directories`、`dbus-secret-service`、`ratatui`、`crossterm`、`color-eyre`。

## 许可证

本项目基于 [MIT License](./LICENSE) 开源。
