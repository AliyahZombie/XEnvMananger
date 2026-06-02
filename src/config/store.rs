use crate::config::model::Profile;
use crate::paths::app_dirs;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProfileStoreError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid program name")]
    InvalidProgram,
}

pub fn profile_path(program: &str) -> Result<PathBuf, ProfileStoreError> {
    if program.is_empty() {
        return Err(ProfileStoreError::InvalidProgram);
    }

    let dirs = app_dirs()?;
    Ok(dirs.profiles_dir().join(format!("{program}.json")))
}

pub fn load_profile(program: &str) -> Result<Profile, ProfileStoreError> {
    let path = profile_path(program)?;
    let bytes = fs::read(path)?;
    Ok(serde_json::from_slice(&bytes)?)
}

pub fn save_profile(profile: &Profile) -> Result<(), ProfileStoreError> {
    let path = profile_path(&profile.program)?;
    ensure_parent_dir(&path)?;

    // Atomic write: write to temp file in same directory, then persist.
    let dir = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "profile path has no parent"))?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))?;
    }

    serde_json::to_writer_pretty(tmp.as_file_mut(), profile)?;
    tmp.persist(&path).map_err(|e| e.error)?;

    // Best-effort: if file existed, re-apply permissions on Unix.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

pub fn list_profiles() -> Result<Vec<String>, ProfileStoreError> {
    let dirs = app_dirs()?;
    let profiles_dir = dirs.profiles_dir();
    let mut out = Vec::new();
    let entries = match fs::read_dir(profiles_dir) {
        Ok(e) => e,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(out),
        Err(e) => return Err(e.into()),
    };

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

    out.sort();
    Ok(out)
}

fn ensure_parent_dir(path: &Path) -> io::Result<()> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    fs::create_dir_all(parent)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::model::{Profile, ProfileSource, StoredEnvVar};
    use std::collections::BTreeMap;
    use std::env;
    use std::ffi::OsString;
    use std::fs;
    use std::sync::{Mutex, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    struct EnvRestore {
        prev: Vec<(OsString, Option<OsString>)>,
    }

    impl EnvRestore {
        fn capture(keys: impl IntoIterator<Item = &'static str>) -> Self {
            let prev = keys
                .into_iter()
                .map(|k| (OsString::from(k), env::var_os(k)))
                .collect();
            Self { prev }
        }

        fn restore(&self) {
            for (k, v) in &self.prev {
                let key = k.to_string_lossy();
                match v {
                    Option::Some(val) => env::set_var(&*key, val),
                    Option::None => env::remove_var(&*key),
                }
            }
        }
    }

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            self.restore();
        }
    }

    fn with_temp_config_root<T>(f: impl FnOnce() -> T) -> T {
        let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();

        let tmp = tempfile::TempDir::new().expect("tempdir");
        let restore = EnvRestore::capture(["XDG_CONFIG_HOME", "HOME", "APPDATA", "LOCALAPPDATA"]);

        env::set_var("XDG_CONFIG_HOME", tmp.path());
        env::set_var("HOME", tmp.path());
        env::set_var("APPDATA", tmp.path());
        env::set_var("LOCALAPPDATA", tmp.path());

        let out = f();
        drop(restore);
        out
    }

    fn test_profile(program: &str) -> Profile {
        let mut env_vars = BTreeMap::new();
        env_vars.insert(
            "FOO".to_string(),
            StoredEnvVar::String {
                value: "bar".to_string(),
            },
        );

        Profile {
            program: program.to_string(),
            source: ProfileSource::Manual,
            last_used: None,
            env_vars,
        }
    }

    #[test]
    fn profile_path_rejects_empty_program() {
        let err = profile_path("").unwrap_err();
        assert!(matches!(err, ProfileStoreError::InvalidProgram));
    }

    #[test]
    fn list_profiles_returns_empty_when_missing() {
        with_temp_config_root(|| {
            let out = list_profiles().expect("list_profiles");
            assert!(out.is_empty());
        });
    }

    #[test]
    fn save_then_load_roundtrips() {
        with_temp_config_root(|| {
            let profile = test_profile("prog_a");
            save_profile(&profile).expect("save_profile");

            let loaded = load_profile("prog_a").expect("load_profile");
            assert_eq!(loaded.program, profile.program);
            assert_eq!(loaded.last_used, profile.last_used);
            assert_eq!(loaded.env_vars.len(), profile.env_vars.len());

            match loaded.env_vars.get("FOO") {
                Some(StoredEnvVar::String { value }) => assert_eq!(value, "bar"),
                other => panic!("unexpected env var value: {other:?}"),
            }
        });
    }

    #[test]
    fn list_profiles_is_sorted_and_filters_non_json() {
        with_temp_config_root(|| {
            save_profile(&test_profile("b"))?;
            save_profile(&test_profile("a"))?;

            let dirs = crate::paths::app_dirs()?;
            let profiles_dir = dirs.profiles_dir();
            fs::create_dir_all(&profiles_dir)?;
            fs::write(profiles_dir.join("ignore.txt"), b"nope")?;

            let out = list_profiles()?;
            assert_eq!(out, vec!["a".to_string(), "b".to_string()]);
            Ok::<_, ProfileStoreError>(())
        })
        .expect("test should succeed");
    }

    #[test]
    fn save_writes_under_profiles_dir() {
        with_temp_config_root(|| {
            let profile = test_profile("prog_x");
            save_profile(&profile).expect("save_profile");

            let path = profile_path("prog_x").expect("profile_path");
            assert!(path.ends_with(Path::new("profiles").join("prog_x.json")));
            assert!(path.exists());
        });
    }

    #[test]
    fn list_profiles_uses_file_stem() {
        with_temp_config_root(|| {
            let dirs = crate::paths::app_dirs().expect("app_dirs");
            let profiles_dir = dirs.profiles_dir();
            fs::create_dir_all(&profiles_dir).expect("create_dir_all");
            fs::write(profiles_dir.join("z.json"), b"{}").expect("write dummy json file");
            fs::write(profiles_dir.join("x.json"), b"{}").expect("write dummy json file");
            fs::write(profiles_dir.join("x.json.bak"), b"{}").expect("write dummy backup file");

            let out = list_profiles().expect("list_profiles");
            assert_eq!(out, vec!["x".to_string(), "z".to_string()]);
        });
    }

    #[test]
    #[cfg(unix)]
    fn save_applies_0600_permissions_on_unix() {
        use std::os::unix::fs::PermissionsExt;

        with_temp_config_root(|| {
            let profile = test_profile("perm_test");
            save_profile(&profile).expect("save_profile");

            let path = profile_path("perm_test").expect("profile_path");
            let mode = fs::metadata(path).expect("metadata").permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        });
    }
}
