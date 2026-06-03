//! Protocol support (Phase 3).

use crate::config::model::{EnvVarType, StoredEnvVar};
use serde::Deserialize;
use std::ffi::OsString;
use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

pub const PROTOCOL_FLAG: &str = "--env-manager-protocol";

#[derive(Debug, Clone)]
pub struct MissingVar {
    pub name: String,
    pub kind: EnvVarType,
}

/// A single env var as reported by a protocol probe.
///
/// `required` is read directly from the protocol output (defaulting to
/// `false`). `default` is the prefilled value, if any; `None` means the var
/// has no default and starts unset.
#[derive(Debug, Clone)]
pub struct ProtocolVar {
    pub name: String,
    pub kind: EnvVarType,
    pub required: bool,
    pub default: Option<StoredEnvVar>,
}

#[derive(Debug, Clone)]
pub struct ProtocolProfile {
    pub program: String,
    pub vars: Vec<ProtocolVar>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolDetectOutcome {
    Ok,
    Failed,
}

#[derive(Debug, Deserialize)]
struct ProtocolDoc {
    version: String,
    program: String,
    env_vars: Vec<ProtocolEnvVar>,
}

#[derive(Debug, Deserialize)]
struct ProtocolEnvVar {
    name: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    required: bool,
    #[serde(default)]
    default: serde_json::Value,
}

pub fn detect_protocol(
    argv: &[OsString],
    expected_program: &str,
    timeout: Duration,
) -> (ProtocolDetectOutcome, Option<ProtocolProfile>) {
    if argv.is_empty() {
        return (ProtocolDetectOutcome::Failed, None);
    }

    let mut cmd = Command::new(&argv[0]);
    if argv.len() > 1 {
        cmd.args(&argv[1..]);
    }
    cmd.arg(PROTOCOL_FLAG)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => return (ProtocolDetectOutcome::Failed, None),
    };

    let Some(mut stdout_pipe) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        return (ProtocolDetectOutcome::Failed, None);
    };
    let stdout_reader = thread::spawn(move || {
        let mut stdout = Vec::new();
        stdout_pipe.read_to_end(&mut stdout).map(|_| stdout)
    });

    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stdout_reader.join();
                    return (ProtocolDetectOutcome::Failed, None);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                let _ = stdout_reader.join();
                return (ProtocolDetectOutcome::Failed, None);
            }
        }
    };

    let stdout = match stdout_reader.join() {
        Ok(Ok(stdout)) => stdout,
        Ok(Err(_)) | Err(_) => return (ProtocolDetectOutcome::Failed, None),
    };

    if !status.success() {
        return (ProtocolDetectOutcome::Failed, None);
    }

    let doc: ProtocolDoc = match serde_json::from_slice(&stdout) {
        Ok(d) => d,
        Err(_) => return (ProtocolDetectOutcome::Failed, None),
    };

    if doc.version != "1.0" {
        return (ProtocolDetectOutcome::Failed, None);
    }
    if doc.program != expected_program {
        return (ProtocolDetectOutcome::Failed, None);
    }

    let mut vars: Vec<ProtocolVar> = Vec::new();

    for v in doc.env_vars {
        let kind = match parse_kind(&v.kind) {
            Some(k) => k,
            None => return (ProtocolDetectOutcome::Failed, None),
        };

        let default = if is_empty_default(&kind, &v.default) {
            None
        } else {
            match stored_from_default(kind.clone(), &v.default) {
                Ok(s) => Some(s),
                Err(()) => return (ProtocolDetectOutcome::Failed, None),
            }
        };

        vars.push(ProtocolVar {
            name: v.name,
            kind,
            required: v.required,
            default,
        });
    }

    vars.sort_by(|a, b| a.name.cmp(&b.name));

    (
        ProtocolDetectOutcome::Ok,
        Some(ProtocolProfile {
            program: expected_program.to_string(),
            vars,
        }),
    )
}

fn parse_kind(kind: &str) -> Option<EnvVarType> {
    match kind {
        "secret" => Some(EnvVarType::Secret),
        "string" => Some(EnvVarType::String),
        "number" => Some(EnvVarType::Number),
        "boolean" => Some(EnvVarType::Boolean),
        "enum" => Some(EnvVarType::Enum),
        "path" => Some(EnvVarType::Path),
        _ => None,
    }
}

/// Returns true when the protocol output carries no usable default for this
/// var, so it should start unset in the editor. A `null`/absent default has no
/// value; secrets additionally never carry a default, so `""` is treated as
/// empty too.
fn is_empty_default(kind: &EnvVarType, default: &serde_json::Value) -> bool {
    default.is_null() || (*kind == EnvVarType::Secret && default.as_str() == Some(""))
}

fn stored_from_default(kind: EnvVarType, default: &serde_json::Value) -> Result<StoredEnvVar, ()> {
    match kind {
        EnvVarType::Secret => Err(()),
        EnvVarType::String => match default {
            serde_json::Value::String(s) => Ok(StoredEnvVar::String { value: s.clone() }),
            _ => Err(()),
        },
        EnvVarType::Enum => match default {
            serde_json::Value::String(s) => Ok(StoredEnvVar::Enum { value: s.clone() }),
            _ => Err(()),
        },
        EnvVarType::Path => match default {
            serde_json::Value::String(s) => Ok(StoredEnvVar::Path { value: s.clone() }),
            _ => Err(()),
        },
        EnvVarType::Boolean => match default {
            serde_json::Value::Bool(b) => Ok(StoredEnvVar::Boolean { value: *b }),
            _ => Err(()),
        },
        EnvVarType::Number => match default {
            serde_json::Value::Number(n) => match n.as_i64() {
                Some(v) => Ok(StoredEnvVar::Number { value: v }),
                None => Err(()),
            },
            _ => Err(()),
        },
    }
}
