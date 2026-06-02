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
    assert_eq!(profile.env_defaults.len(), 5_000);
    assert!(profile.missing_required.is_empty());
}
