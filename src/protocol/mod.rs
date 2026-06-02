//! Protocol support (Phase 3).

use crate::config::model::{EnvVarType, StoredEnvVar};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const PROTOCOL_FLAG: &str = "--env-manager-protocol";

#[derive(Debug, Clone)]
pub struct MissingVar {
    pub name: String,
    pub kind: EnvVarType,
}

#[derive(Debug, Clone)]
pub struct ProtocolProfile {
    pub program: String,
    pub env_defaults: BTreeMap<String, StoredEnvVar>,
    pub missing_required: Vec<MissingVar>,
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

    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    return (ProtocolDetectOutcome::Failed, None);
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return (ProtocolDetectOutcome::Failed, None);
            }
        }
    };

    let mut stdout = Vec::new();
    if let Some(mut out) = child.stdout.take() {
        if out.read_to_end(&mut stdout).is_err() {
            return (ProtocolDetectOutcome::Failed, None);
        }
    }

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

    let mut env_defaults: BTreeMap<String, StoredEnvVar> = BTreeMap::new();
    let mut missing_required: Vec<MissingVar> = Vec::new();

    for v in doc.env_vars {
        let kind = match parse_kind(&v.kind) {
            Some(k) => k,
            None => return (ProtocolDetectOutcome::Failed, None),
        };

        if v.default.is_null() {
            missing_required.push(MissingVar { name: v.name, kind });
            continue;
        }

        let stored = match stored_from_default(kind, &v.default) {
            Ok(s) => s,
            Err(()) => return (ProtocolDetectOutcome::Failed, None),
        };

        env_defaults.insert(v.name, stored);
    }

    missing_required.sort_by(|a, b| a.name.cmp(&b.name));

    (
        ProtocolDetectOutcome::Ok,
        Some(ProtocolProfile {
            program: expected_program.to_string(),
            env_defaults,
            missing_required,
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
