use crate::net::ServerConnection;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppScreen {
    Login,
    Register,
    MainMenu,
    CreateChannel,
    InChannel,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuthField {
    Username,
    Email,
    Password,
}

pub struct AppState {
    pub current_screen: AppScreen,
    pub current_channel: Option<String>,
    pub display_name: String,
    pub input_buffer: String,
    pub server_log: Vec<String>,
    /// Lines scrolled back from the bottom of `server_log`. 0 means "follow live".
    pub scroll_offset: usize,
    pub error_message: Option<String>,
    pub server: ServerConnection,
    pub running: bool,
    pub auth_username: String,
    pub auth_email: String,
    pub auth_password: String,
    pub auth_field: AuthField,
}

impl AppState {
    pub fn new(server: ServerConnection) -> Self {
        Self {
            current_screen: AppScreen::Login,
            current_channel: None,
            display_name: Self::generate_user_id(),
            input_buffer: String::new(),
            server_log: Vec::new(),
            scroll_offset: 0,
            error_message: None,
            server,
            running: true,
            auth_username: String::new(),
            auth_email: String::new(),
            auth_password: String::new(),
            auth_field: AuthField::Username,
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
            self.scroll_offset = 0;
        }

        if line.starts_with("left #") {
            self.current_channel = None;
            self.current_screen = AppScreen::MainMenu;
            self.server_log.clear();
            self.scroll_offset = 0;
        }

        if let Some(username) = line
            .strip_prefix("registered and logged in as ")
            .or_else(|| line.strip_prefix("logged in as "))
        {
            self.display_name = username.trim().to_string();
            self.current_screen = AppScreen::MainMenu;
            self.auth_username.clear();
            self.auth_email.clear();
            self.auth_password.clear();
            self.auth_field = AuthField::Username;
            return;
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

    /// Cycle to the next auth field. Login skips the email field entirely
    /// since `/login` only takes username + password.
    pub fn next_auth_field(&mut self) {
        let is_register = self.current_screen == AppScreen::Register;
        self.auth_field = match self.auth_field {
            AuthField::Username if is_register => AuthField::Email,
            AuthField::Username => AuthField::Password,
            AuthField::Email => AuthField::Password,
            AuthField::Password => AuthField::Username,
        };
    }

    pub fn auth_field_mut(&mut self) -> &mut String {
        match self.auth_field {
            AuthField::Username => &mut self.auth_username,
            AuthField::Email => &mut self.auth_email,
            AuthField::Password => &mut self.auth_password,
        }
    }

    /// Build the `/login` or `/register` command from the current auth fields.
    pub fn build_auth_command(&self) -> String {
        match self.current_screen {
            AppScreen::Register => format!(
                "/register {} {} {}",
                self.auth_username, self.auth_email, self.auth_password
            ),
            _ => format!("/login {} {}", self.auth_username, self.auth_password),
        }
    }

    fn generate_user_id() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        format!("user_{}", rng.gen_range(10000..99999))
    }
}