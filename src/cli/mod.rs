use clap::{Parser, Subcommand};
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

    #[arg(
        long = "protocol",
        alias = "protocal",
        help = "Force using protocol detection (avoid presets)"
    )]
    pub protocol: bool,

    #[command(subcommand)]
    pub cmd: Option<Cmd>,
}

#[derive(Debug, Subcommand)]
pub enum Cmd {
    /// List configured programs.
    List,

    /// List built-in preset definitions.
    Presets {
        #[command(subcommand)]
        cmd: Option<PresetsCmd>,
    },

    Keyring {
        #[command(subcommand)]
        cmd: KeyringCmd,
    },

    /// Show stored profile details.
    Show { program: OsString },

    /// Edit (open TUI) for an existing program.
    Edit { program: OsString },

    /// Delete a stored profile.
    Delete { program: OsString },

    /// Reset a profile to defaults.
    Reset { program: OsString },

    /// Fallback: treat unknown subcommand as a program to run (and its args).
    #[command(external_subcommand)]
    Run(Vec<OsString>),
}

#[derive(Debug, Subcommand)]
pub enum PresetsCmd {
    #[command(about = "List built-in preset definitions")]
    List,

    #[command(about = "Print the user presets directory path")]
    Dir,

    #[command(about = "List user preset files")]
    User,

    #[command(about = "Initialize a user preset file from a built-in definition")]
    Init {
        program: String,

        #[arg(long, help = "Optional second-level subcommand match (e.g. web)")]
        subcommand: Option<String>,

        #[arg(long, help = "Include secret env vars as keyring references")]
        include_secrets: bool,

        #[arg(long, help = "Overwrite an existing preset file")]
        force: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum KeyringCmd {
    Set { key: String },

    Delete { key: String },

    Has { key: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::ffi::OsString;

    #[test]
    fn parses_skip_with_known_subcommand() {
        let cli = Cli::try_parse_from(["em", "--skip", "list"]).expect("parse");
        assert!(cli.skip);
        assert!(matches!(cli.cmd, Some(Cmd::List)));
    }

    #[test]
    fn parses_external_subcommand_as_run_and_preserves_args() {
        let cli =
            Cli::try_parse_from(["em", "--skip", "myprog", "--flag", "value"]).expect("parse");
        assert!(cli.skip);

        match cli.cmd {
            Some(Cmd::Run(args)) => {
                let expected: Vec<OsString> = ["myprog", "--flag", "value"]
                    .into_iter()
                    .map(OsString::from)
                    .collect();
                assert_eq!(args, expected);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_external_subcommand_without_skip() {
        let cli = Cli::try_parse_from(["em", "some_program", "arg1"]).expect("parse");
        assert!(!cli.skip);

        match cli.cmd {
            Some(Cmd::Run(args)) => {
                let expected: Vec<OsString> = ["some_program", "arg1"]
                    .into_iter()
                    .map(OsString::from)
                    .collect();
                assert_eq!(args, expected);
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn parses_presets_as_known_subcommand() {
        let cli = Cli::try_parse_from(["em", "presets"]).expect("parse");
        assert!(!cli.skip);
        assert!(matches!(cli.cmd, Some(Cmd::Presets { cmd: None })));
    }

    #[test]
    fn parses_presets_subcommand() {
        let cli = Cli::try_parse_from(["em", "presets", "dir"]).expect("parse");
        assert!(matches!(
            cli.cmd,
            Some(Cmd::Presets {
                cmd: Some(PresetsCmd::Dir)
            })
        ));
    }

    #[test]
    fn parses_keyring_subcommands() {
        let cli = Cli::try_parse_from(["em", "keyring", "has", "abc"]).expect("parse");
        assert!(matches!(
            cli.cmd,
            Some(Cmd::Keyring {
                cmd: KeyringCmd::Has { .. }
            })
        ));
    }
}
