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
fn skip_runs_saved_profile() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let em = env!("CARGO_BIN_EXE_em");

    with_sandboxed_app_dirs(tmp.path(), || {
        let mut env_vars = std::collections::BTreeMap::new();
        env_vars.insert(
            "EM_FIXTURE_MODE".to_string(),
            em::config::model::StoredEnvVar::String {
                value: "saved".to_string(),
            },
        );
        env_vars.insert(
            "EM_FIXTURE_COLOR".to_string(),
            em::config::model::StoredEnvVar::Enum {
                value: "blue".to_string(),
            },
        );
        env_vars.insert(
            "EM_FIXTURE_FLAG".to_string(),
            em::config::model::StoredEnvVar::Boolean { value: true },
        );
        env_vars.insert(
            "EM_FIXTURE_NUMBER".to_string(),
            em::config::model::StoredEnvVar::Number { value: 7 },
        );

        let profile = em::config::model::Profile {
            program: "em_fixture_protocol_ok".to_string(),
            source: em::config::model::ProfileSource::Manual,
            last_used: None,
            env_vars,
        };
        em::config::store::save_profile(&profile).expect("save_profile");

        let output = Command::new(em)
            .arg("--skip")
            .arg("em_fixture_protocol_ok")
            .env("XDG_CONFIG_HOME", tmp.path())
            .env("HOME", tmp.path())
            .env("APPDATA", tmp.path())
            .env("LOCALAPPDATA", tmp.path())
            .output()
            .expect("run em");

        assert!(output.status.success(), "status: {:?}", output.status);

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("EM_FIXTURE_MODE=saved"), "stdout: {stdout}");
        assert!(stdout.contains("EM_FIXTURE_COLOR=blue"), "stdout: {stdout}");
        assert!(stdout.contains("EM_FIXTURE_FLAG=true"), "stdout: {stdout}");
        assert!(stdout.contains("EM_FIXTURE_NUMBER=7"), "stdout: {stdout}");
    });
}

#[test]
fn skip_fails_when_not_saved() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let em = env!("CARGO_BIN_EXE_em");

    with_sandboxed_app_dirs(tmp.path(), || {
        let output = Command::new(em)
            .arg("--skip")
            .arg("em_fixture_protocol_ok")
            .env("XDG_CONFIG_HOME", tmp.path())
            .env("HOME", tmp.path())
            .env("APPDATA", tmp.path())
            .env("LOCALAPPDATA", tmp.path())
            .output()
            .expect("run em");

        assert!(!output.status.success(), "status: {:?}", output.status);

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("[XEnvManager] em_fixture_protocol_ok has not saved configurations"),
            "stderr: {stderr}"
        );
    });
}

#[test]
fn skip_injects_plaintext_secret_from_profile() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let em = env!("CARGO_BIN_EXE_em");

    with_sandboxed_app_dirs(tmp.path(), || {
        let mut env_vars = std::collections::BTreeMap::new();
        env_vars.insert(
            "EM_FIXTURE_SECRET".to_string(),
            em::config::model::StoredEnvVar::Secret(em::config::model::StoredSecret::Plain {
                required: true,
                value: "s3cr3t".to_string(),
            }),
        );

        let profile = em::config::model::Profile {
            program: "em_fixture_protocol_ok".to_string(),
            source: em::config::model::ProfileSource::Manual,
            last_used: None,
            env_vars,
        };
        em::config::store::save_profile(&profile).expect("save_profile");

        let output = Command::new(em)
            .arg("--skip")
            .arg("em_fixture_protocol_ok")
            .env("XDG_CONFIG_HOME", tmp.path())
            .env("HOME", tmp.path())
            .env("APPDATA", tmp.path())
            .env("LOCALAPPDATA", tmp.path())
            .output()
            .expect("run em");

        assert!(output.status.success(), "status: {:?}", output.status);

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("EM_FIXTURE_SECRET=s3cr3t"),
            "stdout: {stdout}"
        );
    });
}
