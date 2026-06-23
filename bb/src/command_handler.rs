#[derive(Debug, Clone)]
pub enum Command {
    CreateChannel { name: String, size: usize },
    JoinChannel { channel_name: String },
    LeaveChannel,
    SendMessage { content: String },
    ListChannels,
    Exit,
    Help,
}

impl Command {
    pub fn parse(input: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let parts: Vec<&str> = input.trim().split_whitespace().collect();

        if parts.is_empty() {
            return Err("Empty command".into());
        }

        match parts[0] {
            "/create" => {
                if parts.len() < 3 {
                    return Err("/create requires: name and size. Usage: /create <name> <size>".into());
                }
                let name = parts[1].to_string();
                let size = parts[2].parse::<usize>()?;
                if size == 0 {
                    return Err("/create requires a channel size of at least 1.".into());
                }
                Ok(Command::CreateChannel { name, size })
            }
            "/join" => {
                if parts.len() < 2 {
                    return Err("/join requires a channel name. Usage: /join <channel_name>".into());
                }
                let channel_name = parts[1].to_string();
                Ok(Command::JoinChannel { channel_name })
            }
            "/msg" | "/send" => {
                if parts.len() < 2 {
                    return Err("/msg requires a message. Usage: /msg <message>".into());
                }
                let content = parts[1..].join(" ");
                Ok(Command::SendMessage { content })
            }
            "/leave" => Ok(Command::LeaveChannel),
            "/list" | "/channels" => Ok(Command::ListChannels),
            "/exit" | "/quit" => Ok(Command::Exit),
            "/help" => Ok(Command::Help),
            _ => Err(format!("Unknown command: {}. Type /help for available commands.", parts[0]).into()),
        }
    }

    pub fn help_text() -> &'static str {
        r#"
    Available Commands:
        /create <name> <size>  - Create a new channel
        /join <channel>        - Join an existing channel
        /leave                 - Leave the current channel
        /list                  - Show all available channels
        /msg <message>         - Send a message (or just type without /)
        /help                  - Show this help message
        /exit or /quit         - Exit the program
        Ctrl-C                 - Force quit
        "#
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_create_command() {
        let cmd = Command::parse("/create general 50").unwrap();
        match cmd {
            Command::CreateChannel { name, size } => {
                assert_eq!(name, "general");
                assert_eq!(size, 50);
            }
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_parse_join_command() {
        let cmd = Command::parse("/join general").unwrap();
        match cmd {
            Command::JoinChannel { channel_name } => {
                assert_eq!(channel_name, "general");
            }
            _ => panic!("Wrong command type"),
        }
    }

    #[test]
    fn test_parse_leave_command() {
        let cmd = Command::parse("/leave").unwrap();
        assert!(matches!(cmd, Command::LeaveChannel));
    }

    #[test]
    fn test_parse_list_command() {
        let cmd = Command::parse("/list").unwrap();
        assert!(matches!(cmd, Command::ListChannels));

        let cmd2 = Command::parse("/channels").unwrap();
        assert!(matches!(cmd2, Command::ListChannels));
    }
}