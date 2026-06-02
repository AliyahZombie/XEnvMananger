use clap::{CommandFactory, Parser};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    em::keyring::init_default_credential_builder();

    let cli = em::cli::Cli::parse();

    let run_skip = cli.skip;
    let run_protocol = cli.protocol;

    if cli.presets {
        em::tui::run()?;
        return Ok(());
    }

    if cli.preset_list {
        let mut presets = em::presets::built_in_presets()?;
        presets.sort_by(|a, b| a.id.cmp(&b.id));
        for p in presets {
            println!("{}", p.id);
        }
        return Ok(());
    }

    if cli.preset_dir {
        let dirs = em::paths::app_dirs()?;
        println!("{}", dirs.presets_dir().display());
        return Ok(());
    }

    if cli.preset_user {
        let mut names = em::presets::list_user_presets()?;
        names.sort();
        for n in names {
            println!("{n}");
        }
        return Ok(());
    }

    if let Some(program) = cli.preset_init {
        let path = em::presets::init_user_preset_from_builtin(
            &program,
            cli.preset_subcommand.as_deref(),
            cli.include_secrets,
            cli.force,
        )?;
        println!("{}", path.display());
        return Ok(());
    }

    if let Some(key) = cli.keyring_set {
        let value = read_secret_from_stdin()?;
        em::keyring::set_secret(&key, &value)?;
        return Ok(());
    }

    if let Some(key) = cli.keyring_delete {
        em::keyring::delete_secret(&key)?;
        return Ok(());
    }

    if let Some(key) = cli.keyring_has {
        let exists = em::keyring::get_secret(&key)?.is_some();
        std::process::exit(if exists { 0 } else { 1 });
    }

    if cli.run.is_empty() {
        // When no args are provided, show help.
        let mut cmd = em::cli::Cli::command();
        cmd.print_help()?;
        println!();
        return Ok(());
    }

    run_external(run_skip, run_protocol, &cli.run)?;

    Ok(())
}

fn read_secret_from_stdin() -> color_eyre::Result<String> {
    use std::io::Read;

    let mut buf = String::new();
    std::io::stdin().read_to_string(&mut buf)?;
    let v = buf.trim_end_matches(&['\r', '\n'][..]).to_string();
    if v.is_empty() {
        return Err(color_eyre::eyre::eyre!("secret must not be empty"));
    }
    Ok(v)
}

fn run_external(run_skip: bool, run_protocol: bool, argv: &[OsString]) -> color_eyre::Result<()> {
    let program_id = sanitize_program_id(&argv[0])?;
    let spawn_argv = build_spawn_argv(argv)?;

    let cmdline = format_cmdline(argv);
    let subcommand = detect_subcommand(argv);
    let program_key = profile_program_key(&program_id, subcommand);

    if run_skip {
        let profile = match load_profile_if_exists(&program_key)? {
            Some(p) => p,
            None => {
                print_not_saved(&cmdline);
                std::process::exit(1);
            }
        };

        let (env, missing) = resolve_env_vars(&profile.env_vars);
        if !missing.is_empty() {
            print_missing(&missing);
            std::process::exit(1);
        }

        exec_and_exit(&spawn_argv, &env)?;
    }

    let action = em::tui::run_program_config(
        &cmdline,
        &program_id,
        subcommand,
        &program_key,
        run_protocol,
        &spawn_argv,
    )?;

    if let Some(profile) = action.profile_to_save {
        em::config::store::save_profile(&profile)?;
    }

    if action.run_now {
        let profile = load_profile_if_exists(&program_key)?.ok_or_else(|| {
            color_eyre::eyre::eyre!("internal error: expected profile to exist after save")
        })?;

        let (env, missing) = resolve_env_vars(&profile.env_vars);
        if !missing.is_empty() {
            print_missing(&missing);
            std::process::exit(1);
        }

        exec_and_exit(&spawn_argv, &env)?;
    }

    Ok(())
}

fn format_cmdline(argv: &[OsString]) -> String {
    argv.iter()
        .map(|a| a.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

fn profile_program_key(program: &str, subcommand: Option<&str>) -> String {
    let program = encode_profile_key_component(program);
    match subcommand {
        Some(s) => format!("{program}.{}", encode_profile_key_component(s)),
        None => program,
    }
}

fn encode_profile_key_component(component: &str) -> String {
    let mut out = String::with_capacity(component.len());

    for byte in component.bytes() {
        if is_profile_key_component_byte(byte) {
            out.push(char::from(byte));
        } else {
            out.push('%');
            out.push(nibble_to_hex(byte >> 4));
            out.push(nibble_to_hex(byte & 0x0f));
        }
    }

    out
}

fn is_profile_key_component_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_')
}

fn nibble_to_hex(nibble: u8) -> char {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    char::from(HEX[usize::from(nibble)])
}

fn print_not_saved(cmdline: &str) {
    eprintln!(
        "{red}[XEnvManager] {cmdline} has not saved configurations{reset}",
        red = ansi::LIGHT_RED,
        reset = ansi::RESET
    );
}

fn detect_subcommand(argv: &[OsString]) -> Option<&str> {
    argv.get(1..)?.iter().find_map(|a| {
        let s = a.to_str()?;
        if s.starts_with('-') {
            None
        } else {
            Some(s)
        }
    })
}

fn build_spawn_argv(argv: &[OsString]) -> color_eyre::Result<Vec<OsString>> {
    if argv.is_empty() {
        return Err(color_eyre::eyre::eyre!("argv must not be empty"));
    }

    let mut out = argv.to_vec();
    out[0] = resolve_spawn_argv0(&argv[0])?;
    Ok(out)
}

fn resolve_spawn_argv0(argv0: &OsString) -> color_eyre::Result<OsString> {
    let Some(name) = argv0.to_str() else {
        return Ok(argv0.clone());
    };

    if name.contains('/') || name.contains('\\') {
        return Ok(argv0.clone());
    }

    let current_exe = std::env::current_exe()?;
    let Some(dir) = current_exe.parent() else {
        return Ok(argv0.clone());
    };

    let candidate: PathBuf = dir.join(name);
    if candidate.is_file() {
        Ok(candidate.into_os_string())
    } else {
        Ok(argv0.clone())
    }
}

fn sanitize_program_id(argv0: &OsString) -> color_eyre::Result<String> {
    let Some(s) = argv0.to_str() else {
        return Err(color_eyre::eyre::eyre!("program name must be valid UTF-8"));
    };

    if s.is_empty() {
        return Err(color_eyre::eyre::eyre!("program name must not be empty"));
    }
    if s.contains('/') || s.contains('\\') {
        return Err(color_eyre::eyre::eyre!(
            "program name must not contain path separators"
        ));
    }
    if s == "." || s == ".." {
        return Err(color_eyre::eyre::eyre!("program name is not allowed"));
    }

    Ok(s.to_string())
}

fn load_profile_if_exists(
    program_id: &str,
) -> color_eyre::Result<Option<em::config::model::Profile>> {
    match em::config::store::load_profile(program_id) {
        Ok(p) => Ok(Some(p)),
        Err(em::config::store::ProfileStoreError::Io(e))
            if e.kind() == std::io::ErrorKind::NotFound =>
        {
            Ok(None)
        }
        Err(e) => Err(e.into()),
    }
}

fn resolve_env_vars(
    vars: &std::collections::BTreeMap<String, em::config::model::StoredEnvVar>,
) -> (BTreeMap<String, String>, Vec<em::protocol::MissingVar>) {
    let mut out: BTreeMap<String, String> = BTreeMap::new();
    let mut missing: Vec<em::protocol::MissingVar> = Vec::new();

    for (name, val) in vars {
        match val {
            em::config::model::StoredEnvVar::String { value } => {
                out.insert(name.clone(), value.clone());
            }
            em::config::model::StoredEnvVar::Number { value } => {
                out.insert(name.clone(), value.to_string());
            }
            em::config::model::StoredEnvVar::Boolean { value } => {
                out.insert(name.clone(), value.to_string());
            }
            em::config::model::StoredEnvVar::Enum { value } => {
                out.insert(name.clone(), value.clone());
            }
            em::config::model::StoredEnvVar::Path { value } => {
                out.insert(name.clone(), value.clone());
            }
            em::config::model::StoredEnvVar::Secret(secret) => match secret {
                em::config::model::StoredSecret::Keyring {
                    required,
                    keyring_key,
                } => match em::keyring::get_secret(keyring_key) {
                    Ok(Some(s)) => {
                        out.insert(name.clone(), s);
                    }
                    Ok(None) | Err(_) => {
                        if *required {
                            missing.push(em::protocol::MissingVar {
                                name: name.clone(),
                                kind: em::config::model::EnvVarType::Secret,
                            });
                        }
                    }
                },
                em::config::model::StoredSecret::Plain { required, value } => {
                    if value.is_empty() {
                        if *required {
                            missing.push(em::protocol::MissingVar {
                                name: name.clone(),
                                kind: em::config::model::EnvVarType::Secret,
                            });
                        }
                    } else {
                        out.insert(name.clone(), value.clone());
                    }
                }
            },
        }
    }

    missing.sort_by(|a, b| a.name.cmp(&b.name));
    (out, missing)
}

fn print_missing(missing: &[em::protocol::MissingVar]) {
    eprintln!("missing required environment variables:");
    for v in missing {
        eprintln!("{} ({})", v.name, env_type_label(&v.kind));
    }
}

fn env_type_label(t: &em::config::model::EnvVarType) -> &'static str {
    match t {
        em::config::model::EnvVarType::Secret => "secret",
        em::config::model::EnvVarType::String => "string",
        em::config::model::EnvVarType::Number => "number",
        em::config::model::EnvVarType::Boolean => "boolean",
        em::config::model::EnvVarType::Enum => "enum",
        em::config::model::EnvVarType::Path => "path",
    }
}

fn exec_and_exit(argv: &[OsString], env: &BTreeMap<String, String>) -> color_eyre::Result<()> {
    let status = em::executor::run_program(argv, env)?;
    std::process::exit(status.code().unwrap_or(1));
}

mod ansi {
    pub const RESET: &str = "\x1b[0m";
    pub const LIGHT_RED: &str = "\x1b[91m";
}
