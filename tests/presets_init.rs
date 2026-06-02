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
fn presets_init_writes_user_preset_file() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let em_bin = env!("CARGO_BIN_EXE_em");

    with_sandboxed_app_dirs(tmp.path(), || {
        let presets_dir = em::paths::app_dirs().expect("app_dirs").presets_dir();
        let expected_path = presets_dir.join("opencode.web.json");

        let output = Command::new(em_bin)
            .arg("presets")
            .arg("init")
            .arg("opencode")
            .arg("--subcommand")
            .arg("web")
            .arg("--include-secrets")
            .env("XDG_CONFIG_HOME", tmp.path())
            .env("HOME", tmp.path())
            .env("APPDATA", tmp.path())
            .env("LOCALAPPDATA", tmp.path())
            .output()
            .expect("run em");

        assert!(output.status.success(), "status: {:?}", output.status);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert_eq!(stdout.trim(), expected_path.display().to_string());
        assert!(expected_path.exists());

        let preset = em::presets::find_preset("opencode", Some("web"))
            .expect("find_preset")
            .expect("preset");
        let m = preset.r#match.expect("match");
        assert_eq!(m.program, "opencode");
        assert_eq!(m.subcommand.as_deref(), Some("web"));
        assert!(preset.env_vars.contains_key("OPENCODE_SERVER_USERNAME"));
        assert!(preset.env_vars.contains_key("OPENCODE_SERVER_PASSWORD"));

        let output = Command::new(em_bin)
            .arg("presets")
            .arg("user")
            .env("XDG_CONFIG_HOME", tmp.path())
            .env("HOME", tmp.path())
            .env("APPDATA", tmp.path())
            .env("LOCALAPPDATA", tmp.path())
            .output()
            .expect("run em");
        assert!(output.status.success(), "status: {:?}", output.status);

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.lines().any(|l| l.trim() == "opencode.web"),
            "stdout: {stdout}"
        );
    });
}
