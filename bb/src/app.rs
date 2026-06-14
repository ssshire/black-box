use std::sync::mpsc::Sender;
use chrono::Local;

use crate::command_handler::Command;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub user_id: String,
    pub content: String,
    pub timestamp: String,
    pub channel: String,
}

impl Message {
    pub fn new(user_id: &str, content: &str, channel: &str) -> Self {
        Self {
            user_id: user_id.to_string(),
            content: content.to_string(),
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            channel: channel.to_string(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppScreen {
    MainMenu,
    CreateChannel,
    InChannel,
    Help,
}

pub struct AppState {
    pub current_screen: AppScreen,
    pub current_channel: Option<String>,
    pub user_id: String,
    pub input_buffer: String,
    pub messages: Vec<Message>,
    pub error_message: Option<String>,
    pub command_tx: Sender<Command>,
    pub running: bool,
}

impl AppState {
    pub fn new(command_tx: Sender<Command>) -> Self {
        Self {
            current_screen: AppScreen::MainMenu,
            current_channel: None,
            user_id: Self::generate_user_id(),
            input_buffer: String::new(),
            messages: Vec::new(),
            error_message: None,
            command_tx,
            running: true,
        }
    }

    /// Push a message into local state for display.
    pub fn push_message(&mut self, msg: Message) {
        self.messages.push(msg);
        // Keep the last 200 messages in memory.
        if self.messages.len() > 200 {
            self.messages.remove(0);
        }
    }

    /// Clear the input buffer and return its contents.
    pub fn take_input(&mut self) -> String {
        std::mem::take(&mut self.input_buffer)
    }

    fn generate_user_id() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        format!("user_{}", rng.gen_range(10000..99999))
    }
}