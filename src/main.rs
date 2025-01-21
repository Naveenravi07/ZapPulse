use chrono::Local;
use color_eyre::Result;
use crossterm::event::{self, KeyCode, KeyEvent};
use message::{Message, MessageKind, MessageList};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph, Widget},
    DefaultTerminal,
};
mod message;
use tui_textarea::TextArea;

#[derive(Debug)]
struct App {
    textarea: TextArea<'static>,
    messages: MessageList,
    mode: TerminalMode,
    exit: bool,
}

#[derive(Debug)]
enum TerminalMode {
    INPUT,
    NORMAL,
}

impl Default for TerminalMode {
    fn default() -> Self {
        Self::NORMAL
    }
}

impl Default for App {
    fn default() -> Self {
        let mut msgs = vec![
            Message {
                content: "Connected to websocket success".to_string(),
                kind: message::MessageKind::OUTGOING,
                time: Local::now(),
            },
            Message {
                content: "UMBBBBBBBBBBBBBBBBBBBBBBBBBBB".to_string(),
                kind: message::MessageKind::OUTGOING,
                time: Local::now(),
            },
            Message {
                content: "HAHAHAHHAHAHAHAHHAHAHAHAHAHAH".to_string(),
                kind: message::MessageKind::OUTGOING,
                time: Local::now(),
            },
        ];

        if let Some(last_msg) = msgs.last_mut() {
            last_msg.content = "Disconnected from websocket".to_string();
            last_msg.kind = MessageKind::INCOMING;
        }

        let msg_list: MessageList = MessageList::new(msgs);
        Self{
            messages: msg_list,
            exit: false,
            mode: TerminalMode::NORMAL,
            textarea: TextArea::default()
        }
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let result = App::default().run(terminal);
    ratatui::restore();
    result
}

impl App {
    fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }

        Ok(())
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> Result<()> {
        let event = event::read()?;
        match event {
            event::Event::Key(key_event) => {
                if let TerminalMode::INPUT = self.mode {
                    self.handle_msg_input(key_event)?;
                    return Ok(());
                }

                self.handle_key_events(key_event)?;
            }
            _ => {}
        }
        Ok(())
    }

    // For handling all the global keybinds
    fn handle_key_events(&mut self, keyevent: KeyEvent) -> Result<()> {
        match keyevent.code {
            KeyCode::Char('q') => self.exit = true,
            KeyCode::Char('i') => self.mode = TerminalMode::INPUT,
            KeyCode::Char('j') => {
                self.messages.select_next();
            }
            KeyCode::Char('k') => self.messages.select_previous(),
            _ => {}
        }
        Ok(())
    }

    // For inserting into the editor
    fn handle_msg_input(&mut self, keyevent: KeyEvent) -> Result<()> {
        if let KeyCode::Esc = keyevent.code {
            self.mode = TerminalMode::NORMAL;
            return Ok(());
        }
        self.textarea.input(keyevent);
        return Ok(());
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(1),
                    Constraint::Min(2),
                    Constraint::Length(4),
                ]
                .as_ref(),
            )
            .split(area);

        ////// TOP
        let title = Paragraph::new("FUTURE-WS")
            .style(Style::default())
            .alignment(ratatui::layout::Alignment::Center);

        let status = Paragraph::new("Connected")
            .style(Style::default())
            .alignment(Alignment::Right);

        title.render(chunks[0], buf);
        status.render(chunks[0], buf);

        // MIDDLE
        self.messages.render(
            chunks[1].inner(Margin::new(0, 2)),
            buf,
        );

        ///// Bottom
        let bottom_border = Block::default().borders(Borders::ALL);
        bottom_border.render(chunks[2], buf);

        let input_placeholder = if let TerminalMode::NORMAL = self.mode {
            "Press 'i' to start editing "
        } else {
            "Press Esc to stop editing"
        };

        self.textarea.set_cursor_line_style(Style::default());
        self.textarea.set_placeholder_text(input_placeholder);

        let inner_bottom_area = chunks[2].inner(Margin::new(1, 1));
        self.textarea.render(inner_bottom_area, buf);
    }
}
