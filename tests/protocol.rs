use std::ffi::OsString;
use std::time::Duration;

#[test]
fn protocol_detection_handles_large_stdout() {
    let fixture = env!("CARGO_BIN_EXE_em_fixture_protocol_large");
    let argv = vec![OsString::from(fixture)];

    let (outcome, profile) =
        em::protocol::detect_protocol(&argv, "em_fixture_protocol_large", Duration::from_secs(5));

    assert_eq!(outcome, em::protocol::ProtocolDetectOutcome::Ok);
    let profile = profile.expect("profile");
    assert_eq!(profile.vars.len(), 5_000);
    // The large fixture omits `required`, so every var defaults to optional
    // and carries a string default.
    assert!(profile.vars.iter().all(|v| !v.required));
    assert!(profile.vars.iter().all(|v| v.default.is_some()));
}

#[test]
fn protocol_detection_reads_explicit_required_flag() {
    let fixture = env!("CARGO_BIN_EXE_em_fixture_protocol_missing_required");
    let argv = vec![OsString::from(fixture)];

    let (outcome, profile) = em::protocol::detect_protocol(
        &argv,
        "em_fixture_protocol_missing_required",
        Duration::from_secs(5),
    );

    assert_eq!(outcome, em::protocol::ProtocolDetectOutcome::Ok);
    let profile = profile.expect("profile");

    let by_name = |name: &str| {
        profile
            .vars
            .iter()
            .find(|v| v.name == name)
            .unwrap_or_else(|| panic!("missing var {name}"))
    };

    // Required string with a null default: required, no prefilled value.
    let required = by_name("EM_FIXTURE_REQUIRED");
    assert_eq!(required.kind, em::config::model::EnvVarType::String);
    assert!(required.required);
    assert!(required.default.is_none());

    // Required secret: required, never carries a default.
    let secret = by_name("EM_FIXTURE_SECRET");
    assert_eq!(secret.kind, em::config::model::EnvVarType::Secret);
    assert!(secret.required);
    assert!(secret.default.is_none());

    // Optional string with a default: not required, value prefilled.
    let mode = by_name("EM_FIXTURE_MODE");
    assert!(!mode.required);
    assert!(mode.default.is_some());
}
