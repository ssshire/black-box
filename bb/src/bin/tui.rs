use ratatui::{backend::TermwizBackend, widgets::Paragraph, Terminal};
use std::{
    error::Error,
    thread,
    time::{Duration, Instant},
};

fn main() -> Result<(), Box<dyn Error>> {
    let backend = TermwizBackend::new()?;
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let now = Instant::now();
    while now.elapsed() < Duration::from_secs(5) {
        terminal.draw(|f| f.render_widget(Paragraph::new("welcome to blackbox..."), f.area()))?;
        thread::sleep(Duration::from_millis(250));
    }

    terminal.show_cursor()?;
    terminal.flush()?;
    Ok(())
}
