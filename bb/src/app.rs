use crate::net::ServerConnection;

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
    pub server_log: Vec<String>,
    /// Lines scrolled back from the bottom of `server_log`. 0 means "follow live".
    pub scroll_offset: usize,
    pub error_message: Option<String>,
    pub server: ServerConnection,
    pub running: bool,
}

impl AppState {
    pub fn new(server: ServerConnection) -> Self {
        Self {
            current_screen: AppScreen::MainMenu,
            current_channel: None,
            user_id: Self::generate_user_id(),
            input_buffer: String::new(),
            server_log: Vec::new(),
            scroll_offset: 0,
            error_message: None,
            server,
            running: true,
        }
    }

    /// Push a line received from the server into the scrollback for display.
    ///
    /// Channel membership is server-authoritative: a `/join` only actually
    /// switches the screen once the server confirms it with "joined #name",
    /// rather than assuming success the moment the command is sent.
    ///
    /// Lines prefixed "error: " are rejections (channel not found, full,
    /// etc.) — shown as a transient error like local parse failures, not
    /// kept in the permanent chat scrollback.
    pub fn push_server_line(&mut self, line: String) {
        if let Some(name) = line.strip_prefix("joined #") {
            self.current_channel = Some(name.trim().to_string());
            self.current_screen = AppScreen::InChannel;
            // Drop pre-join lobby chatter (welcome text, /list, /create
            // confirmations) — the channel view starts fresh from here.
            self.server_log.clear();
        }

        if line.starts_with("left #") {
            self.current_channel = None;
            self.current_screen = AppScreen::MainMenu;
            self.server_log.clear();
        }

        if let Some(msg) = line.strip_prefix("error: ") {
            self.error_message = Some(msg.to_string());
            return;
        }

        self.server_log.push(line);
        // Keep the last 200 lines in memory.
        if self.server_log.len() > 200 {
            self.server_log.remove(0);
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
