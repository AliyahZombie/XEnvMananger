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
      "required": true,
      "default": null
    },
    {
      "name": "EXAMPLE_MODE",
      "type": "string",
      "required": false,
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

## Meaning of `required`

`required` is an explicit boolean on every env var:

- it is optional in the JSON and defaults to `false`
- `true` marks the variable as required: the editor flags it and refuses to run until it has a value
- `false` (or omitted) marks the variable as optional

`required` is independent of `default`. A field can be required with a default, required with no default, optional with a default, or optional with no default — all four combinations are valid.

## Meaning of `default`

`default` is the value prefilled into the config UI:

- non-null: prefilled as the starting value
- null (or omitted): the variable starts unset

`default` no longer controls whether a field is required — use `required` for that.

### Required fields

To make an env var required, set `required: true`:

```json
{ "name": "API_KEY", "type": "string", "required": true, "default": null }
```

You may still pair a required field with a default if you want a starting value:

```json
{ "name": "REGION", "type": "string", "required": true, "default": "us-east-1" }
```

### Optional fields with defaults

Examples:

```json
{ "name": "MODE", "type": "string", "required": false, "default": "dev" }
{ "name": "DEBUG", "type": "boolean", "required": false, "default": true }
{ "name": "PORT", "type": "number", "required": false, "default": 3000 }
{ "name": "CONFIG_DIR", "type": "path", "required": false, "default": "/tmp/app" }
```

## Secret handling rules

Secrets never carry a value over the protocol, so their `default` must be `null`
(an empty string is accepted as an equivalent unset value). Any non-empty secret
default makes protocol detection fail. Use `required` to mark whether the secret
must be provided.

Example:

```json
{ "name": "PASSWORD", "type": "secret", "required": true, "default": null }
```

When the UI edits a secret:

- if OS keyring is available, `em` stores the secret in keyring and only saves a key reference in the profile
- if keyring is unavailable, `em` shows guidance and allows the secret to be stored directly in the profile file as plaintext

Profile files are written with `0600` permissions on Unix, but plaintext-in-profile is still less secure than a working keyring backend.

## How `em` uses protocol output

After a successful probe:

- variables with non-null defaults become prefilled values
- variables with `required: true` become required fields in the editor
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
- a secret uses a non-empty default
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
    required: bool,
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
                    required: true,
                    default: serde_json::Value::Null,
                },
                EnvVar {
                    name: "MYAPP_MODE",
                    kind: "string",
                    required: false,
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
- set `required: true` on fields that must be provided (defaults to `false` when omitted)
- use `default: null` for secrets; empty string is accepted only as an unset compatibility value
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
