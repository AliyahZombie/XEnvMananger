use clap::Parser;
use std::ffi::OsString;

#[derive(Debug, Parser)]
#[command(
    name = "em",
    version,
    about = "XEnvManager - environment variable manager"
)]
pub struct Cli {
    /// Use last saved config without opening the TUI.
    #[arg(short = 's', long = "skip")]
    pub skip: bool,

    #[arg(long = "presets", help = "Open the presets TUI")]
    pub presets: bool,

    #[arg(long = "preset-list", help = "List built-in preset definitions")]
    pub preset_list: bool,

    #[arg(long = "preset-dir", help = "Print the user presets directory path")]
    pub preset_dir: bool,

    #[arg(long = "preset-user", help = "List user preset files")]
    pub preset_user: bool,

    #[arg(
        long = "preset-init",
        value_name = "PROGRAM",
        help = "Initialize a user preset file from a built-in definition"
    )]
    pub preset_init: Option<String>,

    #[arg(
        long,
        help = "Optional second-level subcommand match for --preset-init"
    )]
    pub preset_subcommand: Option<String>,

    #[arg(long, help = "Include secret env vars as keyring references")]
    pub include_secrets: bool,

    #[arg(long, help = "Overwrite an existing preset file")]
    pub force: bool,

    #[arg(
        long = "keyring-set",
        value_name = "KEY",
        help = "Read a secret from stdin and store it in the keyring"
    )]
    pub keyring_set: Option<String>,

    #[arg(
        long = "keyring-delete",
        value_name = "KEY",
        help = "Delete a secret from the keyring"
    )]
    pub keyring_delete: Option<String>,

    #[arg(
        long = "keyring-has",
        value_name = "KEY",
        help = "Exit 0 if a keyring secret exists, otherwise exit 1"
    )]
    pub keyring_has: Option<String>,

    #[arg(
        long = "protocol",
        alias = "protocal",
        help = "Force using protocol detection (avoid presets)"
    )]
    pub protocol: bool,

    /// Program to run and arguments to pass through.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub run: Vec<OsString>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::ffi::OsString;

    #[test]
    fn parses_skip_with_program_named_like_old_subcommand() {
        let cli = Cli::try_parse_from(["em", "--skip", "list"]).expect("parse");
        assert!(cli.skip);
        assert_eq!(cli.run, vec![OsString::from("list")]);
    }

    #[test]
    fn parses_program_and_preserves_args() {
        let cli =
            Cli::try_parse_from(["em", "--skip", "myprog", "--flag", "value"]).expect("parse");
        assert!(cli.skip);
        let expected: Vec<OsString> = ["myprog", "--flag", "value"]
            .into_iter()
            .map(OsString::from)
            .collect();
        assert_eq!(cli.run, expected);
    }

    #[test]
    fn parses_program_without_skip() {
        let cli = Cli::try_parse_from(["em", "some_program", "arg1"]).expect("parse");
        assert!(!cli.skip);
        let expected: Vec<OsString> = ["some_program", "arg1"]
            .into_iter()
            .map(OsString::from)
            .collect();
        assert_eq!(cli.run, expected);
    }

    #[test]
    fn parses_presets_as_program_name() {
        let cli = Cli::try_parse_from(["em", "presets"]).expect("parse");
        assert!(!cli.skip);
        assert_eq!(cli.run, vec![OsString::from("presets")]);
    }

    #[test]
    fn parses_preset_management_flags() {
        let cli = Cli::try_parse_from(["em", "--preset-dir"]).expect("parse");
        assert!(cli.preset_dir);
        assert!(cli.run.is_empty());
    }

    #[test]
    fn parses_keyring_flags() {
        let cli = Cli::try_parse_from(["em", "--keyring-has", "abc"]).expect("parse");
        assert_eq!(cli.keyring_has.as_deref(), Some("abc"));
        assert!(cli.run.is_empty());
    }
}
