use std::collections::BTreeMap;
use std::process::Command;

#[test]
fn skip_uses_subcommand_specific_profile_even_with_flags_before_subcommand() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let em_bin = env!("CARGO_BIN_EXE_em");

    std::env::set_var("XDG_CONFIG_HOME", tmp.path());
    std::env::set_var("HOME", tmp.path());
    std::env::set_var("APPDATA", tmp.path());
    std::env::set_var("LOCALAPPDATA", tmp.path());

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
}
