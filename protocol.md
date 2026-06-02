# XEnvManager Protocol Integration

This document describes how a program can integrate with `em` by implementing the XEnvManager protocol.

## What the protocol is used for

The protocol is **not** the runtime launch path anymore.

Current `em` behavior is:

- `em <program> <args...>`: open the environment configuration UI
- `em --skip <program> <args...>`: run using the last saved profile only
- `em --protocol <program> <args...>`: prefer protocol over preset for this launch and use that schema to prefill the UI

So the protocol is now a **schema/default discovery mechanism** for the config editor.

## When `em` calls the protocol

`em` calls the target program with:

```bash
<program> <args...> --env-manager-protocol
```

Important details:

- the original argv is preserved
- `--env-manager-protocol` is appended at the end
- the process must exit successfully
- stdout must contain valid JSON
- stderr is ignored by `em`
- the probe times out after **10 seconds**

If the probe fails, `em` falls back to the remaining sources for that launch.

## When the protocol result is used

The current source order for the config editor is:

1. saved profile for the current program/subcommand key
2. preset, if one matches
3. protocol probe
4. empty editor if none of the above yields anything

If `--protocol` is passed, the order becomes:

1. saved profile for the current program/subcommand key
2. protocol probe
3. preset fallback if protocol fails
4. empty editor

So by default, protocol is automatically used when there is no saved profile and no preset.

## Profile key and subcommands

`em` stores profiles per program or per second-level subcommand.

Examples:

- `em opencode` -> profile key `opencode`
- `em opencode web` -> profile key `opencode.web`

The detected subcommand is the first non-flag argument after the program name.

That means your protocol implementation may receive different argv such as:

```bash
opencode web --env-manager-protocol
opencode --verbose web --env-manager-protocol
```

If your env schema depends on a subcommand, inspect your argv exactly the same way your normal app does.

## JSON format

The current protocol version is `1.0`.

Your program must print JSON in this shape:

```json
{
  "version": "1.0",
  "program": "your_program_name",
  "env_vars": [
    {
      "name": "EXAMPLE_TOKEN",
      "type": "secret",
      "default": null
    },
    {
      "name": "EXAMPLE_MODE",
      "type": "string",
      "default": "dev"
    }
  ]
}
```

### Top-level fields

- `version`: must be exactly `"1.0"`
- `program`: must exactly match the executable name `em` expects for the target program
- `env_vars`: list of env var definitions

If `version` or `program` does not match, protocol detection is treated as failed.

## Supported env var types

`type` may be one of:

- `secret`
- `string`
- `number`
- `boolean`
- `enum`
- `path`

## Meaning of `default`

`default` serves two roles:

- non-null: a default value that is prefilled into the config UI
- null: marks the variable as required and initially missing

### Required fields

If you want an env var to be required, set:

```json
{ "name": "API_KEY", "type": "string", "default": null }
```

### Optional fields with defaults

Examples:

```json
{ "name": "MODE", "type": "string", "default": "dev" }
{ "name": "DEBUG", "type": "boolean", "default": true }
{ "name": "PORT", "type": "number", "default": 3000 }
{ "name": "CONFIG_DIR", "type": "path", "default": "/tmp/app" }
```

## Secret handling rules

Secrets should normally be declared with `default: null`.

Example:

```json
{ "name": "PASSWORD", "type": "secret", "default": null }
```

Current implementation does **not** accept a non-null default for `secret`. If you emit one, protocol detection fails.

When the UI edits a secret:

- if OS keyring is available, `em` stores the secret in keyring and only saves a key reference in the profile
- if keyring is unavailable, `em` shows guidance and allows the secret to be stored directly in the profile file as plaintext

Profile files are written with `0600` permissions on Unix, but plaintext-in-profile is still less secure than a working keyring backend.

## How `em` uses protocol output

After a successful probe:

- variables with non-null defaults become prefilled values
- variables with `default: null` become required fields in the editor
- the user can edit values, save the profile, or save-and-run

`em` does not immediately execute the target just because protocol detection succeeded.

## Failure behavior

Protocol detection is considered failed if any of these happen:

- process spawn fails
- process exit code is non-zero
- stdout is not valid JSON
- `version != "1.0"`
- `program` does not match expected executable name
- an env var has an unsupported `type`
- a non-secret default has the wrong JSON type
- a secret uses a non-null default
- the probe exceeds 10 seconds

When protocol detection fails, `em` just continues with its normal fallback behavior instead of crashing.

## Minimal Rust example

```rust
use serde::Serialize;
use std::env;
use std::ffi::OsStr;

const PROTOCOL_FLAG: &str = "--env-manager-protocol";

#[derive(Serialize)]
struct ProtocolDoc<'a> {
    version: &'a str,
    program: &'a str,
    env_vars: Vec<EnvVar<'a>>,
}

#[derive(Serialize)]
struct EnvVar<'a> {
    name: &'a str,
    #[serde(rename = "type")]
    kind: &'a str,
    default: serde_json::Value,
}

fn main() {
    if env::args_os().any(|arg| arg == OsStr::new(PROTOCOL_FLAG)) {
        let doc = ProtocolDoc {
            version: "1.0",
            program: "myapp",
            env_vars: vec![
                EnvVar {
                    name: "MYAPP_TOKEN",
                    kind: "secret",
                    default: serde_json::Value::Null,
                },
                EnvVar {
                    name: "MYAPP_MODE",
                    kind: "string",
                    default: serde_json::Value::String("dev".to_string()),
                },
            ],
        };

        println!("{}", serde_json::to_string_pretty(&doc).unwrap());
        return;
    }

    // normal program flow
}
```

## Integration checklist

- support `--env-manager-protocol`
- print JSON to stdout only
- return `version: "1.0"`
- return the correct `program` name
- use `default: null` for required fields
- use `default: null` for secrets
- finish within 10 seconds
- keep protocol output side-effect free

## Recommended design

Treat protocol mode as a read-only schema endpoint for your CLI.

That usually means:

- do not open network connections unless they are cheap and deterministic
- do not require interactive input
- do not mutate user state
- keep the schema stable for a given argv shape

The cleaner this mode is, the more predictable the `em` onboarding experience will be.
