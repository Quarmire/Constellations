use ratatui::crossterm::event::{EnableMouseCapture, DisableMouseCapture};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{enable_raw_mode, disable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;
use std::io;
use tui_textarea::TextArea;
use crossterm::event::{read, Event, KeyCode};

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

    let backend = CrosstermBackend::new(stdout);

    let mut term = Terminal::new(backend)?;

    // Create an empty `TextArea` instance which manages the editor state
    let mut textarea = TextArea::default();

    // Event loop
    loop {
        term.draw(|f| {
            // Get `ratatui::layout::Rect` where the editor should be rendered
            let rect = f.area();
            // Render the textarea in terminal screen
            f.render_widget(&textarea, rect);
        })?;

        if let Event::Key(key) = read()? {
            // Your own key mapping to break the event loop
            if key.code == KeyCode::Esc {
                break;
            }
            // `TextArea::input` can directly handle key events from backends and update the editor state
            textarea.input(key);
        }
    }

    disable_raw_mode()?;
    execute!(
        term.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    term.show_cursor()?;

    // Get text lines as `&[String]`
    println!("Lines: {:?}", textarea.lines());
    Ok(())
}