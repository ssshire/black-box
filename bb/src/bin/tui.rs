/// TUI binary — `cargo run --bin tui`
use std::sync::mpsc::TryRecvError;

use bb::app::{AppScreen, AppState, AuthField};
use bb::command_handler::Command;
use bb::net::{self, ServerEvent};

use ratatui::{
    backend::TermwizBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Terminal,
};
use termwiz::input::{InputEvent, KeyCode, KeyEvent, Modifiers};
use termwiz::terminal::Terminal as TermwizTerminal;

const SERVER_ADDR: &str = "127.0.0.1:8080";

fn main() {
    let server = match net::connect(SERVER_ADDR) {
        Ok(conn) => conn,
        Err(e) => {
            eprintln!("failed to connect to blackbox server at {}: {}", SERVER_ADDR, e);
            eprintln!("make sure `cargo run --bin tcp` is running first.");
            std::process::exit(1);
        }
    };
    let mut state = AppState::new(server);

    let backend = TermwizBackend::new().expect("failed to create termwiz backend");
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");

    while state.running {
        // termwiz's BufferedTerminal only learns about a terminal resize when
        // explicitly told to check — SIGWINCH is out-of-band on Unix, so the
        // surface (and therefore frame.area()) would otherwise stay frozen
        // at the size the app launched with.
        let _ = terminal.backend_mut().buffered_terminal_mut().check_for_resize();

        terminal
            .draw(|frame| render(frame, &state))
            .expect("draw failed");

        let event = terminal
            .backend_mut()
            .buffered_terminal_mut()
            .terminal()
            .poll_input(Some(std::time::Duration::from_millis(16)))
            .unwrap_or(None);

        if let Some(InputEvent::Key(key)) = event {
            if key.key == KeyCode::Char('c') && key.modifiers == Modifiers::CTRL {
                state.running = false;
                break;
            }

            if matches!(state.current_screen, AppScreen::Login | AppScreen::Register) {
                handle_auth_key(&mut state, key);
            } else {
                match key.key {
                    KeyCode::Char(c) => {
                        state.error_message = None;
                        state.input_buffer.push(c);
                    }
                    KeyCode::Backspace => {
                        state.input_buffer.pop();
                    }
                    KeyCode::Enter => {
                        let input = state.take_input();
                        handle_input(&mut state, input);
                    }
                    KeyCode::Escape => {
                        state.error_message = None;
                        if state.current_screen == AppScreen::InChannel {
                            // Actually leave server-side — the screen only
                            // switches back once "left #..." is confirmed
                            // (see AppState::push_server_line).
                            if let Err(e) = state.server.send_line("/leave") {
                                state.error_message = Some(format!("lost connection to server: {}", e));
                                state.running = false;
                            }
                        } else {
                            state.current_screen = AppScreen::MainMenu;
                        }
                    }
                    KeyCode::UpArrow if state.current_screen == AppScreen::InChannel => {
                        state.scroll_offset = state.scroll_offset.saturating_add(1);
                    }
                    KeyCode::DownArrow if state.current_screen == AppScreen::InChannel => {
                        state.scroll_offset = state.scroll_offset.saturating_sub(1);
                    }
                    KeyCode::PageUp if state.current_screen == AppScreen::InChannel => {
                        state.scroll_offset = state.scroll_offset.saturating_add(10);
                    }
                    KeyCode::PageDown if state.current_screen == AppScreen::InChannel => {
                        state.scroll_offset = state.scroll_offset.saturating_sub(10);
                    }
                    _ => {}
                }
            }
        }

        loop {
            match state.server.events.try_recv() {
                Ok(ServerEvent::Line(line)) => state.push_server_line(line),
                Ok(ServerEvent::Disconnected) | Err(TryRecvError::Disconnected) => {
                    state.error_message = Some("disconnected from server.".to_string());
                    state.running = false;
                    break;
                }
                Err(TryRecvError::Empty) => break,
            }
        }
    }
}

fn handle_input(state: &mut AppState, input: String) {
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() {
        return;
    }

    state.error_message = None;
    state.scroll_offset = 0;

    // Plain text (no leading slash) is a chat message, not a local command —
    // forward it as-is and let the server validate (it auto-prefixes with
    // /msg when the client is in a channel). Local parsing only drives the
    // Help screen and quitting — channel membership is decided by the
    // server's response (see AppState::push_server_line), not guessed here,
    // so a failed /join can't strand the UI on a channel that never joined.
    if trimmed.starts_with('/') {
        match Command::parse(&trimmed) {
            Ok(Command::Help) => state.current_screen = AppScreen::Help,
            Ok(Command::Exit) => state.running = false,
            Ok(_) => {}
            Err(e) => state.error_message = Some(e.to_string()),
        }
    }

    if let Err(e) = state.server.send_line(&trimmed) {
        state.error_message = Some(format!("lost connection to server: {}", e));
        state.running = false;
    }
}

/// Routes key input to whichever auth field is focused, cycles fields with
/// Tab, toggles between Login/Register with Ctrl-R, and submits on Enter.
fn handle_auth_key(state: &mut AppState, key: KeyEvent) {
    if key.key == KeyCode::Char('r') && key.modifiers == Modifiers::CTRL {
        state.current_screen = match state.current_screen {
            AppScreen::Register => AppScreen::Login,
            _ => AppScreen::Register,
        };
        state.error_message = None;
        return;
    }

    match key.key {
        KeyCode::Char(c) => {
            state.error_message = None;
            state.auth_field_mut().push(c);
        }
        KeyCode::Backspace => {
            state.auth_field_mut().pop();
        }
        KeyCode::Tab => state.next_auth_field(),
        KeyCode::Enter => {
            let missing_email = state.current_screen == AppScreen::Register && state.auth_email.is_empty();
            if state.auth_username.is_empty() || state.auth_password.is_empty() || missing_email {
                state.error_message = Some("all fields are required.".to_string());
                return;
            }
            let cmd = state.build_auth_command();
            if let Err(e) = state.server.send_line(&cmd) {
                state.error_message = Some(format!("lost connection to server: {}", e));
                state.running = false;
            }
        }
        _ => {}
    }
}

fn render(frame: &mut ratatui::Frame, state: &AppState) {
    match state.current_screen {
        AppScreen::Login | AppScreen::Register => render_auth(frame, state),
        AppScreen::MainMenu | AppScreen::CreateChannel => render_main_menu(frame, state),
        AppScreen::InChannel => render_channel(frame, state),
        AppScreen::Help => render_help(frame, state),
    }
}

fn render_auth(frame: &mut ratatui::Frame, state: &AppState) {
    let area = frame.area();
    let is_register = state.current_screen == AppScreen::Register;

    let mut constraints = vec![Constraint::Length(3)];
    if is_register {
        constraints.push(Constraint::Length(3));
    }
    constraints.push(Constraint::Length(3));
    constraints.push(Constraint::Length(1));
    constraints.push(Constraint::Min(1));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    let field_style = |field: AuthField| {
        if state.auth_field == field {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Gray)
        }
    };

    let username = Paragraph::new(format!("> {}", state.auth_username))
        .block(Block::default().borders(Borders::ALL).title(" username "))
        .style(field_style(AuthField::Username));
    frame.render_widget(username, chunks[0]);

    let mut idx = 1;
    if is_register {
        let email = Paragraph::new(format!("> {}", state.auth_email))
            .block(Block::default().borders(Borders::ALL).title(" email "))
            .style(field_style(AuthField::Email));
        frame.render_widget(email, chunks[idx]);
        idx += 1;
    }

    let masked_password: String = state.auth_password.chars().map(|_| '*').collect();
    let password = Paragraph::new(format!("> {}", masked_password))
        .block(Block::default().borders(Borders::ALL).title(" password "))
        .style(field_style(AuthField::Password));
    frame.render_widget(password, chunks[idx]);
    idx += 1;

    let hint = if is_register {
        "Tab: next field  Enter: register  Ctrl-R: back to login  Ctrl-C: quit"
    } else {
        "Tab: next field  Enter: log in  Ctrl-R: register instead  Ctrl-C: quit"
    };
    let hint_widget = Paragraph::new(Span::styled(format!("  {}", hint), Style::default().fg(Color::DarkGray)));
    frame.render_widget(hint_widget, chunks[idx]);
    idx += 1;

    if let Some(err) = &state.error_message {
        let err_widget = Paragraph::new(Span::styled(
            format!("  ✗ {}", err),
            Style::default().fg(Color::Red),
        ));
        frame.render_widget(err_widget, chunks[idx]);
    }
}

fn render_main_menu(frame: &mut ratatui::Frame, state: &AppState) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area);

    let banner_text = vec![
        Line::from(Span::styled(
            "  welcome to blackbox",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "  /create <name> <size>  — create a channel",
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            "  /join <name>           — join a channel",
            Style::default().fg(Color::Gray),
        )),
        Line::from(Span::styled(
            "  /help                  — show all commands",
            Style::default().fg(Color::Gray),
        )),
    ];
    let banner = Paragraph::new(banner_text)
        .block(Block::default().borders(Borders::ALL).title(" blackbox "));
    frame.render_widget(banner, chunks[0]);

    let input = Paragraph::new(format!("> {}", state.input_buffer))
        .block(Block::default().borders(Borders::ALL).title(" command "))
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(input, chunks[1]);

    if let Some(err) = &state.error_message {
        let err_widget = Paragraph::new(Span::styled(
            format!("  ✗ {}", err),
            Style::default().fg(Color::Red),
        ));
        frame.render_widget(err_widget, chunks[2]);
    } else if let Some(last) = state.server_log.last() {
        let info_widget = Paragraph::new(Span::styled(
            format!("  {}", last),
            Style::default().fg(Color::DarkGray),
        ));
        frame.render_widget(info_widget, chunks[2]);
    }
}

fn render_channel(frame: &mut ratatui::Frame, state: &AppState) {
    let area = frame.area();
    let channel_name = state.current_channel.as_deref().unwrap_or("unknown");

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);

    let scroll_hint = if state.scroll_offset > 0 {
        "  |  ↓ PgDn: back to live"
    } else {
        ""
    };
    let status = Paragraph::new(Span::styled(
        format!(
            "  #{} — {}  |  Esc: leave  Ctrl-C: quit  |  ↑↓ PgUp/PgDn: scroll{}",
            channel_name, state.display_name, scroll_hint
        ),
        Style::default().fg(Color::DarkGray),
    ));
    frame.render_widget(status, chunks[0]);

    // Tail the log to whatever fits in the visible area, clamped by scroll_offset
    // so 0 always tracks the live bottom and scrolling back never reads out of bounds.
    let view_height = chunks[1].height.saturating_sub(2) as usize;
    let total = state.server_log.len();
    let max_offset = total.saturating_sub(view_height);
    let offset = state.scroll_offset.min(max_offset);
    let end = total - offset;
    let start = end.saturating_sub(view_height);

    let items: Vec<ListItem> = state.server_log[start..end]
        .iter()
        .map(|line| {
            let color = if line.starts_with("you: ") || line.contains(&format!("{}:", state.display_name)) {
                Color::Cyan
            } else if line.starts_with("***") {
                Color::DarkGray
            } else {
                Color::White
            };
            ListItem::new(Line::from(Span::styled(line.clone(), Style::default().fg(color))))
        })
        .collect();

    let title = if offset > 0 {
        format!(" #{} (scrolled back {}) ", channel_name, offset)
    } else {
        format!(" #{} ", channel_name)
    };
    let messages = List::new(items).block(Block::default().borders(Borders::ALL).title(title));
    frame.render_widget(messages, chunks[1]);

    let input = Paragraph::new(format!("> {}", state.input_buffer))
        .block(Block::default().borders(Borders::ALL).title(" message "))
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(input, chunks[2]);

    if let Some(err) = &state.error_message {
        let err_widget = Paragraph::new(Span::styled(
            format!("  ✗ {}", err),
            Style::default().fg(Color::Red),
        ));
        frame.render_widget(err_widget, chunks[3]);
    }
}

fn render_help(frame: &mut ratatui::Frame, _state: &AppState) {
    let area = frame.area();
    let help = Paragraph::new(Command::help_text())
        .block(Block::default().borders(Borders::ALL).title(" help "))
        .wrap(Wrap { trim: false })
        .style(Style::default().fg(Color::Gray));
    frame.render_widget(help, area);
}