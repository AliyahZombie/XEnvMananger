use serde::Serialize;
use std::env;
use std::ffi::OsStr;
use std::process;

const PROTOCOL_FLAG: &str = "--env-manager-protocol";
const PRINTABLE_ENV_VARS: [&str; 2] = ["EM_FIXTURE_REQUIRED", "EM_FIXTURE_MODE"];

#[derive(Serialize)]
struct ProtocolFixture<'a> {
    version: &'a str,
    program: &'a str,
    env_vars: Vec<EnvVarDefinition<'a>>,
}

#[derive(Serialize)]
struct EnvVarDefinition<'a> {
    name: &'a str,
    #[serde(rename = "type")]
    kind: &'a str,
    default: serde_json::Value,
}

fn main() {
    if env::args_os().any(|arg| arg == OsStr::new(PROTOCOL_FLAG)) {
        print_protocol();
        return;
    }

    print_env_values();
}

fn print_protocol() {
    let payload = ProtocolFixture {
        version: "1.0",
        program: "em_fixture_protocol_missing_required",
        env_vars: vec![
            EnvVarDefinition {
                name: "EM_FIXTURE_REQUIRED",
                kind: "string",
                default: serde_json::Value::Null,
            },
            EnvVarDefinition {
                name: "EM_FIXTURE_MODE",
                kind: "string",
                default: serde_json::Value::String("should-not-run".to_string()),
            },
        ],
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&payload).expect("protocol fixture json should serialize")
    );
}

fn print_env_values() {
    for key in PRINTABLE_ENV_VARS {
        let value = env::var(key).unwrap_or_else(|_| "<unset>".to_string());
        println!("{key}={value}");
    }
    process::exit(0);
}
