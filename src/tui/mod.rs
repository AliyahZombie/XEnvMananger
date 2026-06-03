use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Terminal;
use std::io::{self, Write};
use std::time::Duration;

pub fn run() -> color_eyre::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_app(&mut terminal);

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

#[derive(Debug, Default)]
struct App {
    items: Vec<String>,
    selected: usize,
    confirm_delete: bool,
    status: Option<String>,
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> color_eyre::Result<()> {
    let mut app = App::default();
    refresh_items(&mut app)?;

    loop {
        terminal.draw(|f| draw(f.size(), f, &app))?;

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let evt = event::read()?;
        if let Event::Key(key) = evt {
            if handle_key(terminal, &mut app, key)? {
                break;
            }
        }
    }

    Ok(())
}

fn refresh_items(app: &mut App) -> color_eyre::Result<()> {
    let mut items = crate::presets::list_user_presets()?;
    items.sort();
    app.items = items;
    if app.selected >= app.items.len() {
        app.selected = app.items.len().saturating_sub(1);
    }
    Ok(())
}

fn draw(area: Rect, f: &mut ratatui::Frame<'_>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);

    let block = Block::default().title("User Presets").borders(Borders::ALL);

    let items: Vec<ListItem> = if app.items.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "(no user presets yet)",
            Style::default().fg(Color::DarkGray),
        )))]
    } else {
        app.items
            .iter()
            .map(|i| ListItem::new(i.as_str()))
            .collect()
    };

    let mut state = ListState::default();
    if !app.items.is_empty() {
        state.select(Some(app.selected));
    }

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    f.render_stateful_widget(list, chunks[0], &mut state);

    let help = if app.confirm_delete {
        Line::from(vec![
            Span::styled(
                "Delete this preset? ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::styled("y", Style::default().fg(Color::Green)),
            Span::raw("/"),
            Span::styled("n", Style::default().fg(Color::Red)),
            Span::raw(" "),
        ])
    } else {
        Line::from(vec![
            Span::raw("↑/↓ move  "),
            Span::styled("n", Style::default().fg(Color::Green)),
            Span::raw(" new  "),
            Span::styled("d", Style::default().fg(Color::Red)),
            Span::raw(" delete  "),
            Span::styled("q", Style::default().fg(Color::Yellow)),
            Span::raw(" quit"),
        ])
    };

    f.render_widget(
        Paragraph::new(help).block(Block::default().borders(Borders::ALL).title("Help")),
        chunks[1],
    );

    let status = app.status.clone().unwrap_or_default();
    f.render_widget(Paragraph::new(status), chunks[2]);
}

fn handle_key(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    key: KeyEvent,
) -> color_eyre::Result<bool> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Ok(true);
    }

    if app.confirm_delete {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                app.confirm_delete = false;
                if let Some(name) = app.items.get(app.selected).cloned() {
                    match crate::presets::delete_user_preset(&name) {
                        Ok(()) => app.status = Some(format!("Deleted preset: {name}")),
                        Err(e) => app.status = Some(format!("Delete failed: {e}")),
                    }
                    refresh_items(app)?;
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.confirm_delete = false;
            }
            _ => {}
        }
        return Ok(false);
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Ok(true),
        KeyCode::Up => {
            if app.selected > 0 {
                app.selected -= 1;
            }
            Ok(false)
        }
        KeyCode::Down => {
            if app.selected + 1 < app.items.len() {
                app.selected += 1;
            }
            Ok(false)
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if !app.items.is_empty() {
                app.confirm_delete = true;
            }
            Ok(false)
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            let status = create_preset_wizard(terminal)?;
            app.status = status;
            refresh_items(app)?;
            Ok(false)
        }
        _ => Ok(false),
    }
}

fn create_preset_wizard(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> color_eyre::Result<Option<String>> {
    suspend_terminal(terminal)?;

    let result = (|| -> color_eyre::Result<Option<String>> {
        println!("Create preset");
        println!("  1) from built-in template");
        println!("  2) empty preset");
        print!("Choice [1/2]: ");
        io::stdout().flush()?;
        let choice = read_line_trimmed()?;

        let status = match choice.as_str() {
            "1" => create_from_built_in()?,
            "2" => create_empty_preset()?,
            _ => Some("Cancelled".to_string()),
        };

        println!("\nPress Enter to return...");
        let _ = read_line_trimmed();
        Ok(status)
    })();

    resume_terminal(terminal)?;
    result
}

fn create_from_built_in() -> color_eyre::Result<Option<String>> {
    let mut built_ins = crate::presets::built_in_presets()?;
    built_ins.sort_by(|a, b| a.id.cmp(&b.id));

    println!("Built-in presets:");
    for p in &built_ins {
        println!("  - {}", p.id);
    }

    print!("Built-in id: ");
    io::stdout().flush()?;
    let program = read_line_trimmed()?;

    print!("Subcommand (optional): ");
    io::stdout().flush()?;
    let sub_input = read_line_trimmed()?;
    let sub = if sub_input.is_empty() {
        None
    } else {
        Some(sub_input.as_str())
    };

    print!("Include secrets as keyring refs? [y/N]: ");
    io::stdout().flush()?;
    let include_secrets = matches!(read_line_trimmed()?.as_str(), "y" | "Y");

    print!("Overwrite existing file? [y/N]: ");
    io::stdout().flush()?;
    let force = matches!(read_line_trimmed()?.as_str(), "y" | "Y");

    match crate::presets::init_user_preset_from_builtin(&program, sub, include_secrets, force) {
        Ok(path) => Ok(Some(format!("Created: {}", path.display()))),
        Err(e) => Ok(Some(format!("Create failed: {e}"))),
    }
}

fn create_empty_preset() -> color_eyre::Result<Option<String>> {
    print!("Program: ");
    io::stdout().flush()?;
    let program = read_line_trimmed()?;
    if program.is_empty() {
        return Ok(Some("Create failed: program is required".to_string()));
    }

    print!("Subcommand (optional): ");
    io::stdout().flush()?;
    let sub_input = read_line_trimmed()?;
    let sub = if sub_input.is_empty() {
        None
    } else {
        Some(sub_input.as_str())
    };

    print!("Overwrite existing file? [y/N]: ");
    io::stdout().flush()?;
    let force = matches!(read_line_trimmed()?.as_str(), "y" | "Y");

    match crate::presets::init_user_preset_empty(&program, sub, force) {
        Ok(path) => Ok(Some(format!("Created: {}", path.display()))),
        Err(e) => Ok(Some(format!("Create failed: {e}"))),
    }
}

fn read_line_trimmed() -> io::Result<String> {
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_string())
}

fn suspend_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> color_eyre::Result<()> {
    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

fn resume_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> color_eyre::Result<()> {
    enable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    Ok(())
}

#[derive(Debug)]
pub struct RunConfigAction {
    pub profile_to_save: Option<crate::config::model::Profile>,
    pub run_now: bool,
}

pub fn run_program_config(
    cmdline: &str,
    program_id: &str,
    subcommand: Option<&str>,
    program_key: &str,
    use_protocol: bool,
    spawn_argv: &[std::ffi::OsString],
) -> color_eyre::Result<RunConfigAction> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let res = run_config_app(
        &mut terminal,
        cmdline,
        program_id,
        subcommand,
        program_key,
        use_protocol,
        spawn_argv,
    );

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigOrigin {
    Saved,
    Preset,
    Protocol,
    Empty,
}

#[derive(Debug, Clone)]
struct ConfigItem {
    name: String,
    kind: crate::config::model::EnvVarType,
    required: bool,
    value: Option<crate::config::model::StoredEnvVar>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputKind {
    Plain,
    Secret,
}

#[derive(Debug, Clone)]
struct InputState {
    item_idx: usize,
    kind: InputKind,
    buf: String,
}

#[derive(Debug)]
struct ConfigApp {
    cmdline: String,
    program_key: String,
    origin: ConfigOrigin,
    items: Vec<ConfigItem>,
    selected: usize,
    input: Option<InputState>,
    status: Option<String>,
    action: Option<RunConfigAction>,
}

fn run_config_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    cmdline: &str,
    program_id: &str,
    subcommand: Option<&str>,
    program_key: &str,
    use_protocol: bool,
    spawn_argv: &[std::ffi::OsString],
) -> color_eyre::Result<RunConfigAction> {
    let keyring_available = crate::keyring::is_available();
    let (origin, items) = load_config_items(
        program_id,
        subcommand,
        program_key,
        use_protocol,
        spawn_argv,
        keyring_available,
    )?;
    let mut app = ConfigApp {
        cmdline: cmdline.to_string(),
        program_key: program_key.to_string(),
        origin,
        items,
        selected: 0,
        input: None,
        status: if keyring_available {
            None
        } else {
            Some(
                "Keyring unavailable. Secrets will be stored in the profile file (0600). Enable a Secret Service backend (e.g. gnome-keyring) and run under a DBus session.".to_string(),
            )
        },
        action: None,
    };

    loop {
        terminal.draw(|f| draw_config(f.size(), f, &app))?;
        if let Some(action) = app.action.take() {
            return Ok(action);
        }

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            if handle_config_key(&mut app, key)? {
                break;
            }
        }
    }

    Ok(RunConfigAction {
        profile_to_save: None,
        run_now: false,
    })
}

fn load_config_items(
    program_id: &str,
    subcommand: Option<&str>,
    program_key: &str,
    use_protocol: bool,
    spawn_argv: &[std::ffi::OsString],
    keyring_available: bool,
) -> color_eyre::Result<(ConfigOrigin, Vec<ConfigItem>)> {
    if let Some(items) = load_saved_items(program_key)? {
        return Ok((ConfigOrigin::Saved, items));
    }

    if use_protocol {
        if let Some(items) = load_protocol_items(program_id, spawn_argv, keyring_available) {
            return Ok((ConfigOrigin::Protocol, items));
        }

        if let Some(items) = load_preset_items(program_id, subcommand, keyring_available)? {
            return Ok((ConfigOrigin::Preset, items));
        }

        return Ok((ConfigOrigin::Empty, Vec::new()));
    }

    if let Some(items) = load_preset_items(program_id, subcommand, keyring_available)? {
        return Ok((ConfigOrigin::Preset, items));
    }

    if let Some(items) = load_protocol_items(program_id, spawn_argv, keyring_available) {
        return Ok((ConfigOrigin::Protocol, items));
    }

    Ok((ConfigOrigin::Empty, Vec::new()))
}

fn load_saved_items(program_key: &str) -> color_eyre::Result<Option<Vec<ConfigItem>>> {
    match crate::config::store::load_profile(program_key) {
        Ok(profile) => {
            let mut items = Vec::new();
            for (name, val) in profile.env_vars {
                let (kind, required) = stored_kind_required(&val);
                items.push(ConfigItem {
                    name,
                    kind,
                    required,
                    value: Some(val),
                });
            }
            items.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(Some(items))
        }
        Err(crate::config::store::ProfileStoreError::Io(e))
            if e.kind() == std::io::ErrorKind::NotFound =>
        {
            Ok(None)
        }
        Err(e) => Err(e.into()),
    }
}

fn load_protocol_items(
    program_id: &str,
    spawn_argv: &[std::ffi::OsString],
    keyring_available: bool,
) -> Option<Vec<ConfigItem>> {
    let (_, proto) =
        crate::protocol::detect_protocol(spawn_argv, program_id, Duration::from_secs(10));
    let proto = proto?;

    let mut items: Vec<ConfigItem> = Vec::new();
    for v in proto.vars {
        let value = if v.kind == crate::config::model::EnvVarType::Secret {
            // Secrets never carry a default value over the protocol; seed an
            // empty placeholder backed by keyring or plaintext storage.
            Some(secret_placeholder(
                program_id,
                &v.name,
                v.required,
                keyring_available,
            ))
        } else {
            v.default
        };

        items.push(ConfigItem {
            name: v.name,
            kind: v.kind,
            required: v.required,
            value,
        });
    }

    items.sort_by(|a, b| a.name.cmp(&b.name));
    Some(items)
}

fn load_preset_items(
    program_id: &str,
    subcommand: Option<&str>,
    keyring_available: bool,
) -> color_eyre::Result<Option<Vec<ConfigItem>>> {
    let preset = crate::presets::find_preset(program_id, subcommand)?;
    let Some(preset) = preset else {
        return Ok(None);
    };

    let mut items = Vec::new();
    for (name, val) in preset.env_vars {
        let normalized = match val {
            crate::config::model::StoredEnvVar::Secret(secret) => {
                crate::config::model::StoredEnvVar::Secret(normalize_secret_storage(
                    program_id,
                    &name,
                    secret,
                    keyring_available,
                ))
            }
            other => other,
        };

        let (kind, required) = stored_kind_required(&normalized);
        items.push(ConfigItem {
            name,
            kind,
            required,
            value: Some(normalized),
        });
    }
    items.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Some(items))
}

fn normalize_secret_storage(
    program_id: &str,
    var_name: &str,
    secret: crate::config::model::StoredSecret,
    keyring_available: bool,
) -> crate::config::model::StoredSecret {
    match secret {
        crate::config::model::StoredSecret::Keyring { required, .. } if !keyring_available => {
            crate::config::model::StoredSecret::Plain {
                required,
                value: String::new(),
            }
        }
        crate::config::model::StoredSecret::Keyring { required, .. } => {
            crate::config::model::StoredSecret::Keyring {
                required,
                keyring_key: format!("{program_id}:{var_name}"),
            }
        }
        crate::config::model::StoredSecret::Plain { required, value } => {
            crate::config::model::StoredSecret::Plain { required, value }
        }
    }
}

fn secret_placeholder(
    program_id: &str,
    var_name: &str,
    required: bool,
    keyring_available: bool,
) -> crate::config::model::StoredEnvVar {
    use crate::config::model::{StoredEnvVar, StoredSecret};
    if keyring_available {
        StoredEnvVar::Secret(StoredSecret::Keyring {
            required,
            keyring_key: format!("{program_id}:{var_name}"),
        })
    } else {
        StoredEnvVar::Secret(StoredSecret::Plain {
            required,
            value: String::new(),
        })
    }
}

fn stored_kind_required(
    v: &crate::config::model::StoredEnvVar,
) -> (crate::config::model::EnvVarType, bool) {
    match v {
        crate::config::model::StoredEnvVar::Secret(s) => {
            (crate::config::model::EnvVarType::Secret, s.required())
        }
        crate::config::model::StoredEnvVar::String { .. } => {
            (crate::config::model::EnvVarType::String, false)
        }
        crate::config::model::StoredEnvVar::Number { .. } => {
            (crate::config::model::EnvVarType::Number, false)
        }
        crate::config::model::StoredEnvVar::Boolean { .. } => {
            (crate::config::model::EnvVarType::Boolean, false)
        }
        crate::config::model::StoredEnvVar::Enum { .. } => {
            (crate::config::model::EnvVarType::Enum, false)
        }
        crate::config::model::StoredEnvVar::Path { .. } => {
            (crate::config::model::EnvVarType::Path, false)
        }
    }
}

fn draw_config(area: Rect, f: &mut ratatui::Frame<'_>, app: &ConfigApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .split(area);

    let origin = match app.origin {
        ConfigOrigin::Saved => "saved",
        ConfigOrigin::Preset => "preset",
        ConfigOrigin::Protocol => "protocol",
        ConfigOrigin::Empty => "empty",
    };

    let header = Line::from(vec![
        Span::styled("[XEnvManager] ", Style::default().fg(Color::LightBlue)),
        Span::raw(&app.cmdline),
        Span::raw(" "),
        Span::styled(
            format!("({origin})"),
            Style::default().fg(Color::LightGreen),
        ),
    ]);
    f.render_widget(Paragraph::new(header), chunks[0]);

    let mut state = ListState::default();
    if !app.items.is_empty() {
        state.select(Some(app.selected));
    }

    let items: Vec<ListItem> = if app.items.is_empty() {
        vec![ListItem::new(Line::from(Span::styled(
            "(no known env schema)",
            Style::default().fg(Color::DarkGray),
        )))]
    } else {
        app.items
            .iter()
            .map(|it| {
                let req = if it.required { "*" } else { " " };
                let val = config_value_summary(it);
                ListItem::new(Line::from(vec![
                    Span::styled(req, Style::default().fg(Color::Red)),
                    Span::raw(" "),
                    Span::styled(
                        it.name.clone(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        env_type_label(&it.kind),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw("  "),
                    Span::raw(val),
                ]))
            })
            .collect()
    };

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Environment"))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");
    f.render_stateful_widget(list, chunks[1], &mut state);

    let footer_lines = if let Some(input) = &app.input {
        let prompt = match input.kind {
            InputKind::Plain => "Enter value: ",
            InputKind::Secret => "Enter secret: ",
        };

        let shown = match input.kind {
            InputKind::Plain => input.buf.clone(),
            InputKind::Secret => "*".repeat(input.buf.chars().count()),
        };

        vec![
            Line::from(vec![
                Span::styled(prompt, Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(shown),
            ]),
            Line::from(Span::raw("Enter=confirm  Esc=cancel  Backspace=delete")),
        ]
    } else {
        vec![
            Line::from(vec![
                Span::raw("↑/↓ move  Enter edit  "),
                Span::styled("s", Style::default().fg(Color::LightGreen)),
                Span::raw(" save  "),
                Span::styled("r", Style::default().fg(Color::LightGreen)),
                Span::raw(" run  "),
                Span::styled("q", Style::default().fg(Color::Yellow)),
                Span::raw(" quit"),
            ]),
            Line::from(Span::raw(app.status.clone().unwrap_or_default())),
        ]
    };

    f.render_widget(
        Paragraph::new(footer_lines).block(Block::default().borders(Borders::ALL)),
        chunks[2],
    );
}

fn config_value_summary(it: &ConfigItem) -> String {
    match &it.value {
        None => "<unset>".to_string(),
        Some(crate::config::model::StoredEnvVar::Secret(s)) => match s {
            crate::config::model::StoredSecret::Keyring { keyring_key, .. } => {
                match crate::keyring::get_secret(keyring_key) {
                    Ok(Some(_)) => "[set]".to_string(),
                    Ok(None) => "[unset]".to_string(),
                    Err(_) => "[keyring unavailable]".to_string(),
                }
            }
            crate::config::model::StoredSecret::Plain { value, .. } => {
                if value.is_empty() {
                    "[unset]".to_string()
                } else {
                    "[set]".to_string()
                }
            }
        },
        Some(crate::config::model::StoredEnvVar::String { value }) => value.clone(),
        Some(crate::config::model::StoredEnvVar::Enum { value }) => value.clone(),
        Some(crate::config::model::StoredEnvVar::Path { value }) => value.clone(),
        Some(crate::config::model::StoredEnvVar::Number { value }) => value.to_string(),
        Some(crate::config::model::StoredEnvVar::Boolean { value }) => value.to_string(),
    }
}

fn handle_config_key(app: &mut ConfigApp, key: KeyEvent) -> color_eyre::Result<bool> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Ok(true);
    }

    if let Some(input) = &mut app.input {
        match key.code {
            KeyCode::Esc => {
                app.input = None;
            }
            KeyCode::Backspace => {
                input.buf.pop();
            }
            KeyCode::Enter => {
                let idx = input.item_idx;
                let buf = std::mem::take(&mut input.buf);
                let kind = input.kind;
                app.input = None;
                apply_input(app, idx, kind, buf)?;
            }
            KeyCode::Char(c) => input.buf.push(c),
            _ => {}
        }
        return Ok(false);
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Ok(true),
        KeyCode::Up => {
            if app.selected > 0 {
                app.selected -= 1;
            }
            Ok(false)
        }
        KeyCode::Down => {
            if app.selected + 1 < app.items.len() {
                app.selected += 1;
            }
            Ok(false)
        }
        KeyCode::Enter => {
            start_edit(app);
            Ok(false)
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            app.action = Some(RunConfigAction {
                profile_to_save: Some(build_profile(app)),
                run_now: false,
            });
            Ok(false)
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            if validate_required_for_run(app)? {
                app.action = Some(RunConfigAction {
                    profile_to_save: Some(build_profile(app)),
                    run_now: true,
                });
            }
            Ok(false)
        }
        _ => Ok(false),
    }
}

fn start_edit(app: &mut ConfigApp) {
    if app.items.is_empty() {
        return;
    }
    let idx = app.selected;
    let item = &app.items[idx];

    match item.kind {
        crate::config::model::EnvVarType::Boolean => {
            let next = match &item.value {
                Some(crate::config::model::StoredEnvVar::Boolean { value }) => !*value,
                _ => true,
            };
            app.items[idx].value =
                Some(crate::config::model::StoredEnvVar::Boolean { value: next });
        }
        crate::config::model::EnvVarType::Secret => {
            app.input = Some(InputState {
                item_idx: idx,
                kind: InputKind::Secret,
                buf: String::new(),
            });
        }
        _ => {
            let buf = match &item.value {
                Some(crate::config::model::StoredEnvVar::String { value }) => value.clone(),
                Some(crate::config::model::StoredEnvVar::Enum { value }) => value.clone(),
                Some(crate::config::model::StoredEnvVar::Path { value }) => value.clone(),
                Some(crate::config::model::StoredEnvVar::Number { value }) => value.to_string(),
                Some(crate::config::model::StoredEnvVar::Boolean { value }) => value.to_string(),
                _ => String::new(),
            };

            app.input = Some(InputState {
                item_idx: idx,
                kind: InputKind::Plain,
                buf,
            });
        }
    }
}

fn apply_input(
    app: &mut ConfigApp,
    idx: usize,
    kind: InputKind,
    buf: String,
) -> color_eyre::Result<()> {
    let item = app
        .items
        .get(idx)
        .ok_or_else(|| color_eyre::eyre::eyre!("invalid selection"))?
        .clone();

    match kind {
        InputKind::Secret => {
            let Some(crate::config::model::StoredEnvVar::Secret(secret)) = item.value else {
                return Ok(());
            };

            if buf.is_empty() {
                match secret {
                    crate::config::model::StoredSecret::Keyring { keyring_key, .. } => {
                        let _ = crate::keyring::delete_secret(&keyring_key);
                        app.status = Some(format!("Cleared secret: {}", item.name));
                    }
                    crate::config::model::StoredSecret::Plain { required, .. } => {
                        app.items[idx].value = Some(crate::config::model::StoredEnvVar::Secret(
                            crate::config::model::StoredSecret::Plain {
                                required,
                                value: String::new(),
                            },
                        ));
                        app.status = Some(format!("Cleared secret: {}", item.name));
                    }
                }
                return Ok(());
            }

            match secret {
                crate::config::model::StoredSecret::Keyring {
                    required,
                    keyring_key,
                } => match crate::keyring::set_secret(&keyring_key, &buf) {
                    Ok(()) => {
                        app.status = Some(format!("Saved secret: {}", item.name));
                        Ok(())
                    }
                    Err(_) => {
                        app.items[idx].value = Some(crate::config::model::StoredEnvVar::Secret(
                            crate::config::model::StoredSecret::Plain {
                                required,
                                value: buf,
                            },
                        ));
                        app.status = Some(
                            "Keyring unavailable. Saved secret in profile file (0600). Enable a Secret Service backend (e.g. gnome-keyring) and run under a DBus session.".to_string(),
                        );
                        Ok(())
                    }
                },
                crate::config::model::StoredSecret::Plain { required, .. } => {
                    app.items[idx].value = Some(crate::config::model::StoredEnvVar::Secret(
                        crate::config::model::StoredSecret::Plain {
                            required,
                            value: buf,
                        },
                    ));
                    app.status = Some(format!("Saved secret (plaintext): {}", item.name));
                    Ok(())
                }
            }
        }
        InputKind::Plain => {
            if buf.is_empty() {
                app.items[idx].value = None;
                return Ok(());
            }

            let stored = match item.kind {
                crate::config::model::EnvVarType::String => {
                    crate::config::model::StoredEnvVar::String { value: buf }
                }
                crate::config::model::EnvVarType::Enum => {
                    crate::config::model::StoredEnvVar::Enum { value: buf }
                }
                crate::config::model::EnvVarType::Path => {
                    crate::config::model::StoredEnvVar::Path { value: buf }
                }
                crate::config::model::EnvVarType::Number => {
                    let v: i64 = buf.parse()?;
                    crate::config::model::StoredEnvVar::Number { value: v }
                }
                crate::config::model::EnvVarType::Boolean => {
                    let v: bool = buf.parse()?;
                    crate::config::model::StoredEnvVar::Boolean { value: v }
                }
                crate::config::model::EnvVarType::Secret => return Ok(()),
            };

            app.items[idx].value = Some(stored);
            Ok(())
        }
    }
}

fn validate_required_for_run(app: &mut ConfigApp) -> color_eyre::Result<bool> {
    let mut missing = Vec::new();
    for it in &app.items {
        if !it.required {
            continue;
        }

        match &it.value {
            Some(crate::config::model::StoredEnvVar::Secret(s)) => match s {
                crate::config::model::StoredSecret::Keyring { keyring_key, .. } => {
                    match crate::keyring::get_secret(keyring_key) {
                        Ok(Some(_)) => {}
                        Ok(None) => missing.push(it.name.clone()),
                        Err(_) => {
                            app.status = Some(
                                "Keyring unavailable. Enable a Secret Service backend (e.g. gnome-keyring) and run under a DBus session, or re-save secrets as plaintext in the editor.".to_string(),
                            );
                            missing.push(it.name.clone());
                        }
                    }
                }
                crate::config::model::StoredSecret::Plain { value, .. } => {
                    if value.is_empty() {
                        missing.push(it.name.clone());
                    }
                }
            },
            Some(_) => {}
            None => missing.push(it.name.clone()),
        }
    }

    if missing.is_empty() {
        app.status = Some("Ready".to_string());
        return Ok(true);
    }

    missing.sort();
    app.status = Some(format!("Missing required: {}", missing.join(", ")));
    Ok(false)
}

fn build_profile(app: &ConfigApp) -> crate::config::model::Profile {
    use crate::config::model::{Profile, ProfileSource, StoredEnvVar};
    use std::collections::BTreeMap;

    let mut env_vars: BTreeMap<String, StoredEnvVar> = BTreeMap::new();
    for it in &app.items {
        if it.kind == crate::config::model::EnvVarType::Secret {
            if let Some(v) = &it.value {
                env_vars.insert(it.name.clone(), v.clone());
            }
            continue;
        }

        if let Some(v) = &it.value {
            env_vars.insert(it.name.clone(), v.clone());
        }
    }

    Profile {
        program: app.program_key.clone(),
        source: ProfileSource::Manual,
        last_used: None,
        env_vars,
    }
}

fn env_type_label(t: &crate::config::model::EnvVarType) -> &'static str {
    match t {
        crate::config::model::EnvVarType::Secret => "secret",
        crate::config::model::EnvVarType::String => "string",
        crate::config::model::EnvVarType::Number => "number",
        crate::config::model::EnvVarType::Boolean => "boolean",
        crate::config::model::EnvVarType::Enum => "enum",
        crate::config::model::EnvVarType::Path => "path",
    }
}
