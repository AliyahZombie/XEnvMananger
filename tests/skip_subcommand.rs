use std::collections::BTreeMap;
use std::env;
use std::ffi::OsString;
use std::process::Command;
use std::sync::{Mutex, OnceLock};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

struct EnvSnapshot {
    prev: Vec<(OsString, Option<OsString>)>,
}

impl EnvSnapshot {
    fn capture(keys: impl IntoIterator<Item = &'static str>) -> Self {
        let prev = keys
            .into_iter()
            .map(|k| (OsString::from(k), env::var_os(k)))
            .collect();
        Self { prev }
    }

    fn restore(self) {
        for (k, v) in self.prev {
            let key = k.to_string_lossy();
            match v {
                Some(val) => env::set_var(&*key, val),
                None => env::remove_var(&*key),
            }
        }
    }
}

fn with_sandboxed_app_dirs<T>(root: &std::path::Path, f: impl FnOnce() -> T) -> T {
    let lock = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
    let snapshot = EnvSnapshot::capture(["XDG_CONFIG_HOME", "HOME", "APPDATA", "LOCALAPPDATA"]);

    env::set_var("XDG_CONFIG_HOME", root);
    env::set_var("HOME", root);
    env::set_var("APPDATA", root);
    env::set_var("LOCALAPPDATA", root);

    let out = f();
    snapshot.restore();
    drop(lock);
    out
}

#[test]
fn skip_uses_subcommand_specific_profile_even_with_flags_before_subcommand() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let em_bin = env!("CARGO_BIN_EXE_em");

    with_sandboxed_app_dirs(tmp.path(), || {
        let mut env_vars: BTreeMap<String, em::config::model::StoredEnvVar> = BTreeMap::new();
        env_vars.insert(
            "EM_FIXTURE_MODE".to_string(),
            em::config::model::StoredEnvVar::String {
                value: "web".to_string(),
            },
        );

        let profile = em::config::model::Profile {
            program: "em_fixture_no_protocol.web".to_string(),
            source: em::config::model::ProfileSource::Manual,
            last_used: None,
            env_vars,
        };
        em::config::store::save_profile(&profile).expect("save_profile");

        let output = Command::new(em_bin)
            .arg("--skip")
            .arg("em_fixture_no_protocol")
            .arg("--verbose")
            .arg("web")
            .env("XDG_CONFIG_HOME", tmp.path())
            .env("HOME", tmp.path())
            .env("APPDATA", tmp.path())
            .env("LOCALAPPDATA", tmp.path())
            .output()
            .expect("run em");

        assert!(output.status.success(), "status: {:?}", output.status);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("EM_FIXTURE_MODE=web"), "stdout: {stdout}");
    });
}

#[test]
fn skip_encodes_subcommand_profile_key_before_saving_or_loading() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let em_bin = env!("CARGO_BIN_EXE_em");

    with_sandboxed_app_dirs(tmp.path(), || {
        let mut env_vars: BTreeMap<String, em::config::model::StoredEnvVar> = BTreeMap::new();
        env_vars.insert(
            "EM_FIXTURE_MODE".to_string(),
            em::config::model::StoredEnvVar::String {
                value: "encoded".to_string(),
            },
        );

        let profile = em::config::model::Profile {
            program: "em_fixture_no_protocol.%2E%2E%2F%2E%2E%2Fx".to_string(),
            source: em::config::model::ProfileSource::Manual,
            last_used: None,
            env_vars,
        };
        em::config::store::save_profile(&profile).expect("save_profile");

        let output = Command::new(em_bin)
            .arg("--skip")
            .arg("em_fixture_no_protocol")
            .arg("../../x")
            .env("XDG_CONFIG_HOME", tmp.path())
            .env("HOME", tmp.path())
            .env("APPDATA", tmp.path())
            .env("LOCALAPPDATA", tmp.path())
            .output()
            .expect("run em");

        assert!(output.status.success(), "status: {:?}", output.status);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("EM_FIXTURE_MODE=encoded"),
            "stdout: {stdout}"
        );

        let profiles_dir = em::paths::app_dirs().expect("app_dirs").profiles_dir();
        assert!(profiles_dir
            .join("em_fixture_no_protocol.%2E%2E%2F%2E%2E%2Fx.json")
            .exists());
        assert!(!tmp.path().join("x.json").exists());
    });
}
