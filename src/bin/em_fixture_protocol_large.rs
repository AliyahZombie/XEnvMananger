use serde::Serialize;
use std::env;
use std::ffi::OsStr;

const PROTOCOL_FLAG: &str = "--env-manager-protocol";

#[derive(Serialize)]
struct ProtocolFixture {
    version: &'static str,
    program: &'static str,
    env_vars: Vec<EnvVarDefinition>,
}

#[derive(Serialize)]
struct EnvVarDefinition {
    name: String,
    #[serde(rename = "type")]
    kind: &'static str,
    default: serde_json::Value,
}

fn main() {
    if env::args_os().any(|arg| arg == OsStr::new(PROTOCOL_FLAG)) {
        print_protocol();
    }
}

fn print_protocol() {
    let env_vars = (0..5_000)
        .map(|idx| EnvVarDefinition {
            name: format!("EM_FIXTURE_LARGE_{idx:04}"),
            kind: "string",
            default: serde_json::Value::String(format!("value-{idx:04}")),
        })
        .collect();

    let payload = ProtocolFixture {
        version: "1.0",
        program: "em_fixture_protocol_large",
        env_vars,
    };

    serde_json::to_writer(std::io::stdout(), &payload)
        .expect("protocol fixture json should serialize");
}
