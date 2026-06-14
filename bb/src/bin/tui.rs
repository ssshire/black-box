/// TUI binary — `cargo run --bin tui`
use std::sync::mpsc::{channel, TryRecvError};

use bb::app::{AppScreen, AppState, Message};
use bb::command_handler::Command;

use ratatui::{
    backend::TermwizBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Terminal,
};
use termwiz::input::{InputEvent, KeyCode, Modifiers};
use termwiz::terminal::Terminal as TermwizTerminal;

fn main() {
    let (cmd_tx, cmd_rx) = channel::<Command>();
    let mut state = AppState::new(cmd_tx);

    let backend = TermwizBackend::new().expect("failed to create termwiz backend");
    let mut terminal = Terminal::new(backend).expect("failed to create terminal");

    while state.running {
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
                    state.current_screen = AppScreen::MainMenu;
                    state.error_message = None;
                }
                _ => {}
            }
        }

        match cmd_rx.try_recv() {
            Ok(_) | Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                state.running = false;
            }
        }
    }
}

fn handle_input(state: &mut AppState, input: String) {
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() {
        return;
    }

    if state.current_screen == AppScreen::InChannel && !trimmed.starts_with('/') {
        let channel = state
            .current_channel
            .clone()
            .unwrap_or_else(|| "general".to_string());
        let msg = Message::new(&state.user_id, &trimmed, &channel);
        state.push_message(msg);
        let _ = state.command_tx.send(Command::SendMessage { content: trimmed });
        return;
    }

    match Command::parse(&trimmed) {
        Ok(cmd) => {
            state.error_message = None;
            match &cmd {
                Command::CreateChannel { name, size: _ } => {
                    state.current_channel = Some(name.clone());
                    state.current_screen = AppScreen::InChannel;
                    let sys = Message::new("system", &format!("Channel '{}' created.", name), name);
                    state.push_message(sys);
                }
                Command::JoinChannel { channel_name } => {
                    state.current_channel = Some(channel_name.clone());
                    state.current_screen = AppScreen::InChannel;
                    let sys = Message::new("system", &format!("Joined '{}'.", channel_name), channel_name);
                    state.push_message(sys);
                }
                Command::Help => {
                    state.current_screen = AppScreen::Help;
                }
                Command::Exit => {
                    state.running = false;
                }
                Command::SendMessage { content } => {
                    if let Some(ch) = state.current_channel.clone() {
                        let msg = Message::new(&state.user_id, content, &ch);
                        state.push_message(msg);
                    }
                }
            }
            let _ = state.command_tx.send(cmd);
        }
        Err(e) => {
            state.error_message = Some(e.to_string());
        }
    }
}

fn render(frame: &mut ratatui::Frame, state: &AppState) {
    match state.current_screen {
        AppScreen::MainMenu | AppScreen::CreateChannel => render_main_menu(frame, state),
        AppScreen::InChannel => render_channel(frame, state),
        AppScreen::Help => render_help(frame, state),
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

    let status = Paragraph::new(Span::styled(
        format!("  #{} — {}  |  Esc: menu  Ctrl-C: quit", channel_name, state.user_id),
        Style::default().fg(Color::DarkGray),
    ));
    frame.render_widget(status, chunks[0]);

    let items: Vec<ListItem> = state
        .messages
        .iter()
        .filter(|m| state.current_channel.as_deref() == Some(m.channel.as_str()))
        .map(|m| {
            let color = if m.user_id == "system" {
                Color::DarkGray
            } else if m.user_id == state.user_id {
                Color::Cyan
            } else {
                Color::White
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("[{}] ", m.timestamp),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!("{}: ", m.user_id),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(m.content.clone()),
            ]))
        })
        .collect();

    let messages = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(format!(" #{} ", channel_name)));
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