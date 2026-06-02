//! Process execution with environment injection.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::process::{Command, ExitStatus, Stdio};

pub fn run_program(
    argv: &[OsString],
    env: &BTreeMap<String, String>,
) -> std::io::Result<ExitStatus> {
    if argv.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "argv must not be empty",
        ));
    }

    let mut cmd = Command::new(&argv[0]);
    if argv.len() > 1 {
        cmd.args(&argv[1..]);
    }

    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .envs(env);

    cmd.status()
}
