# XEnvManager - `em`

> Discover, configure, securely store, and seamlessly inject the environment variables your CLIs need, right from the comfort of your terminal.

[中文](./README.md) | **English**

## Why does this exist?

For many terminal tools, the real hurdle isn't the command itself—it's the tedious setup required before the command can even run.

You try to fire up a cool new CLI, only to find yourself lost in the env-var maze:

- Exactly which environment variables does this program even care about? You often have to dig through docs, GitHub issues, or someone else's dotfiles just to find out.
- Once you finally track down the names, what's next? Manually `export`ing them, muddying up your shell rc files, or piecing together a massive `FOO=... BAR=... program ...` one-liner?
- Where do sensitive tokens, passwords, and access keys go without accidentally leaking into your shell history or plaintext configs?
- And what if the exact same program demands completely different variables the moment you switch subcommands? It's a recipe for confusion.

`em` was built to make this entire flow effortless. It transforms the whole "discover variables -> configure values -> secure secrets -> run command" dance into a smooth, terminal-native visual editing experience. It intelligently prefills the editor using your saved profiles, built-in presets, or even a schema actively exposed by the target program itself via the XEnvManager protocol.

## Quick Start

```bash
em opencode web
```

Typing this pops open a sleek environment configuration TUI. Note that by default, `em <program>` simply opens the editor—it doesn't blindly run the target program right away. Take your time to review the variables, fill in the blanks, and when you're ready, just press `s` to safely stash the profile, or press `r` to save and immediately execute the command.

The next time you need to run the exact same environment, there's no need to repeat the process:

```bash
em --skip opencode web
```

By adding the `--skip` flag, `em` skips the UI, quietly loads your saved profile in the background, injects the environment, and fires up the target program. If it can't find a matching profile, it will gracefully fail and let you know.

## Installation & Building

You'll need a stable Rust toolchain installed. We're using the Rust 2021 edition here.

```bash
cargo build --release
./target/release/em --help

cargo run -- opencode web

cargo install --path .
```

Heads up for Linux users: If you want the peace of mind of storing secrets in your system's keyring, make sure you have an active Secret Service backend (like gnome-keyring or KWallet) running within a DBus session.

## Command Cheat Sheet

| Command | What it does |
| --- | --- |
| `em <program> [args...]` | Fire up the environment config TUI for your target command. |
| `em --skip <program> [args...]` | Skip the UI: inject the saved profile's environment and run the target program directly. |
| `em --protocol <program> [args...]` | Open the TUI, explicitly favoring the target program's own protocol schema for prefilling. |
| `em --presets` | Launch the TUI to manage your user presets. |
| `em --preset-list` | Take a peek at all available built-in preset IDs. |
| `em --preset-user` | List out your personal user preset files. |
| `em --preset-dir` | Print the exact directory where your user presets live. |
| `em --preset-init <program> [--preset-subcommand <sub>] [--include-secrets] [--force]` | Clone a built-in preset into a user preset so you can customize it. |
| `em --keyring-set <KEY>` | Read a secret from stdin and lock it securely in the system keyring. |
| `em --keyring-delete <KEY>` | Wipe a specific secret from the keyring. |
| `em --keyring-has <KEY>` | Check if a secret exists (exits 0 if found, 1 if not). |

Just run `em` completely bare, and it will print out the full help manual.

## Navigating the TUI

| Key | Action |
| --- | --- |
| `↑` / `↓` | Glide up and down your variables |
| `Enter` | Edit the currently selected variable. Booleans toggle instantly; secrets pop open a secure, masked prompt |
| `s` | Save your profile and exit cleanly |
| `r` | Save and immediately run (it'll helpfully double-check that you haven't missed any required fields first) |
| `q` / `Esc` | Bail out without saving |

A bright red `*` is your hint that a variable is strictly required. When you type in a secret, everything is safely masked with `*`. The very top header always keeps you informed about where this list of variables came from—whether it's `saved`, `preset`, `protocol`, or completely `empty`. No guessing required.

Here's how `em` decides who wins when filling out the form:

- Default behavior: Saved Profile -> Built-in Preset -> Program Protocol -> Empty
- When you pass `--protocol`: Saved Profile -> Program Protocol -> Built-in Preset -> Empty

## Profiles & Where They Live

`em` is smart enough to save distinct profiles based on the program, or even down to the specific subcommand:

- Running `em opencode` yields the profile `opencode`
- Running `em opencode web` yields the profile `opencode.web`

That "second-level subcommand" simply refers to the very first non-flag argument you provide right after the main program name. And don't worry about weird characters—keys used for filenames are properly sanitized and encoded.

We respect your system's layout. The configuration directory is determined via `directories::ProjectDirs::from("io", "xenvmanager", "em")`, sticking to standard OS conventions:

| OS | Typical config directory |
| --- | --- |
| Linux | `$XDG_CONFIG_HOME/em` (usually `~/.config/em`) |
| macOS | `~/Library/Application Support/io.xenvmanager.em` |
| Windows | `%APPDATA%\xenvmanager\em\config` |

Your profiles hang out in `<config_dir>/profiles/<key>.json`, while any presets you forge yourself live in `<config_dir>/presets/<name>.json`. Everything is standard JSON, and on Unix systems, `em` strictly locks down these files with `0600` permissions.

## Guarding Your Secrets

By default, `em` hands your secrets off to the DBus Secret Service (under the identifier `io.xenvmanager.em`). When your system keyring is up and running, your local profile file only stores a harmless reference ID; the actual sensitive tokens and passwords are locked away securely inside the system keyring.

If the keyring happens to be unavailable or broken, the TUI will give you a clear heads-up. It gives you the choice to fallback to storing secrets as plaintext right inside the profile file. While `em` still locks the file down with `0600` permissions on Unix, writing plaintext to disk is obviously less secure than a dedicated keyring, so proceed with caution.

```bash
printf '%s' 's3cr3t' | em --keyring-set opencode:OPENCODE_SERVER_PASSWORD
em --keyring-has opencode:OPENCODE_SERVER_PASSWORD
em --keyring-delete opencode:OPENCODE_SERVER_PASSWORD
```

## Presets: Standing on the Shoulders of Giants

Presets are essentially "batteries-included" checklists of variables for popular CLI tools. They exist entirely to save you from having to embark on a documentation scavenger hunt just to get started.

We currently bundle these built-in presets:

- Docker
- AWS CLI
- OpenCode

If you craft your own user presets, they will always override the built-in ones. Furthermore, precision matters: a highly specific `program + subcommand` preset will always take priority over a broader `program`-only preset.

```bash
em --preset-list
em --preset-user
em --preset-dir
em --presets
em --preset-init opencode --preset-subcommand web --include-secrets
```

When a built-in preset is used to automatically fill out the TUI, it intelligently populates non-secret variables with their default values, while bringing in secret variables as secure keyring references. For instance, OpenCode's `OPENCODE_SERVER_PASSWORD` will only magically appear if you specifically target the `web` subcommand.

## For CLI Developers: Integrating with XEnvManager

If you're the maintainer of a CLI tool and you'd love to spare your users the pain of hunting through READMEs just to figure out what environment variables to set, you should absolutely implement the XEnvManager protocol.

Once you're hooked up, a user simply types:

```bash
em your-cli serve
```

Behind the scenes, `em` fires off a quick probe:

```bash
your-cli serve --env-manager-protocol
```

If your program responds by outputting a JSON schema for that specific mode, `em` slurps it up and instantly populates the configuration TUI with all the variable names, data types, default values, and required flags. It's like magic for your users.

### Is your CLI a good fit?

This protocol is a massive quality-of-life upgrade if your CLI:

- Heavily relies on API keys, tokens, backend endpoints, AWS regions, or custom config paths.
- Demands totally different configurations depending on the subcommand (e.g., `serve` vs `deploy` vs `web`).
- Has a user base that frequently opens issues asking, "Wait, what's the env var for this again?"
- Wants to offer a seamless, zero-friction discovery mechanism without altering a single byte of your core runtime logic.

### Protocol Ground Rules

Remember, this protocol is strictly for discovering schemas and default values—it is **not** an execution pathway. When your program detects the `--env-manager-protocol` flag:

- It must print only valid JSON to `stdout`—no extraneous logs, please!
- The process must exit cleanly with a success code.
- It needs to be snappy; the probe must complete within 10 seconds.
- Absolutely do not open interactive prompts or alter the user's local state in any way.
- The `version` field must be hardcoded to `"1.0"`.
- The `program` field must precisely match the target name the user handed to `em`.
- Keep secrets safe: use `default: null` for secrets. Empty strings are treated as unset for compatibility, while non-empty defaults are still rejected.

Here's the minimal JSON blueprint:

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

Currently supported types are: `secret`, `string`, `number`, `boolean`, `enum`, and `path`.

### Design Best Practices

Treat the protocol mode exactly like a read-only REST endpoint: it should be lightning fast, rock-solid, and entirely side-effect free.
If your CLI utilizes complex subcommands, we highly recommend reusing your existing argument parsing logic to determine the schema. For example, intercepting `your-cli web --env-manager-protocol` should ideally return a schema tailored exclusively for the `web` mode.

Once you've integrated it, give it a quick spin to ensure everything lines up:

```bash
your-cli web --env-manager-protocol
em --protocol your-cli web
em your-cli web
```

For the exhaustive list of field rules, error fallback behaviors, and a complete Rust reference implementation, dive into [protocol.md](./protocol.md).

## Development

```bash
cargo build --release
cargo run -- <program> [args...]
cargo test
```

Our CI pipeline will rigorously check your code by running `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`.

This project heavily relies on these fantastic crates: `clap`, `serde` / `serde_json`, `directories`, `dbus-secret-service`, `ratatui`, `crossterm`, and `color-eyre`.

## License

This project is open source under the [MIT License](./LICENSE).
