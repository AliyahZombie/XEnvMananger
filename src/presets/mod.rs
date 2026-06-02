//! Preset system (Phase 2).

use crate::config::model::EnvVarType;
use crate::config::model::StoredEnvVar;
use crate::config::model::StoredSecret;
use crate::paths::app_dirs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    #[serde(default)]
    pub r#match: Option<PresetMatch>,

    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub commands: Vec<String>,

    #[serde(default)]
    pub env_vars: BTreeMap<String, StoredEnvVar>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetMatch {
    pub program: String,

    #[serde(default)]
    pub subcommand: Option<String>,
}

#[derive(Debug, Error)]
pub enum PresetError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Deserialize)]
pub struct BuiltInPreset {
    pub id: String,

    #[serde(default)]
    pub name: Option<String>,

    pub env_vars: Vec<BuiltInEnvVar>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BuiltInEnvVar {
    pub name: String,

    #[serde(rename = "type")]
    pub ty: EnvVarType,

    pub required: bool,

    #[serde(default)]
    pub default: Option<serde_json::Value>,

    pub docs_url: String,
}

#[derive(Debug, Error)]
pub enum BuiltInPresetError {
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

const BUILT_IN_DOCKER_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/presets/docker.json"));
const BUILT_IN_AWS_JSON: &str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/presets/aws.json"));
const BUILT_IN_OPENCODE_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/presets/opencode.json"
));

pub fn built_in_presets() -> Result<Vec<BuiltInPreset>, BuiltInPresetError> {
    let docker: BuiltInPreset = serde_json::from_str(BUILT_IN_DOCKER_JSON)?;
    let aws: BuiltInPreset = serde_json::from_str(BUILT_IN_AWS_JSON)?;
    let opencode: BuiltInPreset = serde_json::from_str(BUILT_IN_OPENCODE_JSON)?;
    Ok(vec![docker, aws, opencode])
}

pub fn find_preset(program: &str, subcommand: Option<&str>) -> Result<Option<Preset>, PresetError> {
    if let Some(p) = find_user_preset(program, subcommand)? {
        return Ok(Some(p));
    }

    find_built_in_preset(program, subcommand)
}

fn find_user_preset(
    program: &str,
    subcommand: Option<&str>,
) -> Result<Option<Preset>, PresetError> {
    let dirs = app_dirs()?;
    let presets_dir = dirs.presets_dir();

    let entries = match fs::read_dir(presets_dir) {
        Ok(e) => e,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    let mut best: Option<(Preset, u8)> = None;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.extension() != Some(OsStr::new("json")) {
            continue;
        }

        let bytes = match fs::read(&path) {
            Ok(b) => b,
            Err(e) => return Err(e.into()),
        };

        let preset: Preset = match serde_json::from_slice(&bytes) {
            Ok(p) => p,
            Err(_) => continue,
        };

        let (matches, specificity) = preset_matches(&preset, &path, program, subcommand);
        if !matches {
            continue;
        }

        match &best {
            Some((_, best_score)) if *best_score >= specificity => {}
            _ => best = Some((preset, specificity)),
        }
    }

    Ok(best.map(|(p, _)| p))
}

fn find_built_in_preset(
    program: &str,
    subcommand: Option<&str>,
) -> Result<Option<Preset>, PresetError> {
    let built_ins = built_in_presets().map_err(|e| match e {
        BuiltInPresetError::Json(err) => PresetError::Json(err),
    })?;

    let Some(built_in) = built_ins.into_iter().find(|p| p.id == program) else {
        return Ok(None);
    };

    let mut env_vars: BTreeMap<String, StoredEnvVar> = BTreeMap::new();
    for v in built_in.env_vars {
        if v.ty == EnvVarType::Secret {
            let name = v.name;

            if !should_include_built_in_secret(&built_in.id, subcommand, &name) {
                continue;
            }

            let keyring_key = format!("{}:{}", built_in.id, name);
            env_vars.insert(
                name,
                StoredEnvVar::Secret(StoredSecret::Keyring {
                    required: v.required,
                    keyring_key,
                }),
            );
            continue;
        }
    }

    if env_vars.is_empty() {
        return Ok(None);
    }

    Ok(Some(Preset {
        r#match: Some(PresetMatch {
            program: built_in.id,
            subcommand: None,
        }),
        name: built_in.name,
        commands: Vec::new(),
        env_vars,
    }))
}

fn should_include_built_in_secret(program: &str, subcommand: Option<&str>, var_name: &str) -> bool {
    if program == "opencode" && var_name == "OPENCODE_SERVER_PASSWORD" {
        return subcommand == Some("web");
    }

    true
}

pub fn list_user_presets() -> Result<Vec<String>, PresetError> {
    let dirs = app_dirs()?;
    let presets_dir = dirs.presets_dir();

    let entries = match fs::read_dir(presets_dir) {
        Ok(e) => e,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.into()),
    };

    let mut out = Vec::new();
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension() != Some(OsStr::new("json")) {
            continue;
        }
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            out.push(stem.to_string());
        }
    }
    Ok(out)
}

pub fn delete_user_preset(stem: &str) -> Result<(), PresetError> {
    if stem.is_empty() || stem.contains('/') || stem.contains('\\') {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "invalid preset name").into());
    }

    let dirs = app_dirs()?;
    let path = dirs.presets_dir().join(format!("{stem}.json"));
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[derive(Debug, Error)]
pub enum PresetInitError {
    #[error("unknown built-in preset id: {0}")]
    UnknownBuiltIn(String),

    #[error("io error: {0}")]
    Io(#[from] io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("built-in preset error: {0}")]
    BuiltIn(#[from] BuiltInPresetError),

    #[error("preset already exists: {0}")]
    AlreadyExists(PathBuf),

    #[error("invalid preset file name")]
    InvalidFileName,
}

pub fn init_user_preset_from_builtin(
    built_in_id: &str,
    subcommand: Option<&str>,
    include_secrets: bool,
    overwrite: bool,
) -> Result<PathBuf, PresetInitError> {
    let built_ins = built_in_presets()?;
    let Some(built_in) = built_ins.into_iter().find(|p| p.id == built_in_id) else {
        return Err(PresetInitError::UnknownBuiltIn(built_in_id.to_string()));
    };

    let mut env_vars: BTreeMap<String, StoredEnvVar> = BTreeMap::new();
    for v in built_in.env_vars {
        if v.ty == EnvVarType::Secret {
            if include_secrets {
                env_vars.insert(
                    v.name.clone(),
                    StoredEnvVar::Secret(StoredSecret::Keyring {
                        required: v.required,
                        keyring_key: format!("{built_in_id}:{}", v.name),
                    }),
                );
            }
            continue;
        }

        let Some(default) = v.default.as_ref() else {
            continue;
        };

        if let Some(stored) = stored_env_var_from_default(&v.ty, default) {
            env_vars.insert(v.name, stored);
        }
    }

    let preset = Preset {
        r#match: Some(PresetMatch {
            program: built_in_id.to_string(),
            subcommand: subcommand.map(|s| s.to_string()),
        }),
        name: built_in.name,
        commands: Vec::new(),
        env_vars,
    };

    let file_name =
        user_preset_file_name(built_in_id, subcommand).ok_or(PresetInitError::InvalidFileName)?;
    write_user_preset_file(&preset, &file_name, overwrite)
}

pub fn init_user_preset_empty(
    program: &str,
    subcommand: Option<&str>,
    overwrite: bool,
) -> Result<PathBuf, PresetInitError> {
    let preset = Preset {
        r#match: Some(PresetMatch {
            program: program.to_string(),
            subcommand: subcommand.map(|s| s.to_string()),
        }),
        name: None,
        commands: Vec::new(),
        env_vars: BTreeMap::new(),
    };

    let file_name =
        user_preset_file_name(program, subcommand).ok_or(PresetInitError::InvalidFileName)?;
    write_user_preset_file(&preset, &file_name, overwrite)
}

fn stored_env_var_from_default(ty: &EnvVarType, v: &serde_json::Value) -> Option<StoredEnvVar> {
    match ty {
        EnvVarType::Secret => None,
        EnvVarType::String => v.as_str().map(|s| StoredEnvVar::String {
            value: s.to_string(),
        }),
        EnvVarType::Enum => v.as_str().map(|s| StoredEnvVar::Enum {
            value: s.to_string(),
        }),
        EnvVarType::Path => v.as_str().map(|s| StoredEnvVar::Path {
            value: s.to_string(),
        }),
        EnvVarType::Boolean => v.as_bool().map(|b| StoredEnvVar::Boolean { value: b }),
        EnvVarType::Number => v
            .as_i64()
            .or_else(|| v.as_u64().and_then(|n| i64::try_from(n).ok()))
            .map(|n| StoredEnvVar::Number { value: n }),
    }
}

fn user_preset_file_name(program: &str, subcommand: Option<&str>) -> Option<String> {
    let program = sanitize_file_component(program);
    if program.is_empty() {
        return None;
    }

    let mut base = program;
    if let Some(sub) = subcommand {
        let sub = sanitize_file_component(sub);
        if sub.is_empty() {
            return None;
        }
        base.push('.');
        base.push_str(&sub);
    }

    base.push_str(".json");
    Some(base)
}

fn sanitize_file_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    out.trim_matches('_').to_string()
}

fn write_user_preset_file(
    preset: &Preset,
    file_name: &str,
    overwrite: bool,
) -> Result<PathBuf, PresetInitError> {
    if file_name.is_empty() || file_name.contains('/') || file_name.contains('\\') {
        return Err(PresetInitError::InvalidFileName);
    }

    let dirs = app_dirs()?;
    let presets_dir = dirs.presets_dir();
    fs::create_dir_all(&presets_dir)?;

    let path: PathBuf = presets_dir.join(file_name);
    if path.exists() && !overwrite {
        return Err(PresetInitError::AlreadyExists(path));
    }

    atomic_write_json(&path, preset)?;
    Ok(path)
}

fn atomic_write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), io::Error> {
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "preset path has no parent"))?;

    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))?;
    }

    serde_json::to_writer_pretty(tmp.as_file_mut(), value)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    tmp.persist(path).map_err(|e| e.error)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

fn preset_matches(
    preset: &Preset,
    path: &std::path::Path,
    program: &str,
    subcommand: Option<&str>,
) -> (bool, u8) {
    if let Some(m) = &preset.r#match {
        if m.program != program {
            return (false, 0);
        }

        if let Some(wanted) = m.subcommand.as_deref() {
            return (subcommand == Some(wanted), 2);
        }

        return (true, 1);
    }

    if !preset.commands.is_empty() {
        return (preset.commands.iter().any(|c| c == program), 1);
    }

    let stem_matches = path
        .file_stem()
        .and_then(|s| s.to_str())
        .is_some_and(|stem| stem == program);
    (stem_matches, 0)
}
