use std::env;
use std::ffi::OsStr;
use std::process;

const PROTOCOL_FLAG: &str = "--env-manager-protocol";
const PRINTABLE_ENV_VARS: [&str; 4] = [
    "EM_FIXTURE_MODE",
    "EM_FIXTURE_COLOR",
    "EM_FIXTURE_FLAG",
    "EM_FIXTURE_NUMBER",
];

fn main() {
    if env::args_os().any(|arg| arg == OsStr::new(PROTOCOL_FLAG)) {
        eprintln!("protocol not supported by this fixture");
        process::exit(2);
    }

    for key in PRINTABLE_ENV_VARS {
        let value = env::var(key).unwrap_or_else(|_| "<unset>".to_string());
        println!("{key}={value}");
    }
}
