use chrono::{Date, DateTime, Local};
use color_eyre::Result;
use crossbeam_channel::Sender;
use crossterm::event::{self, KeyCode, KeyEvent};
use message::{Message, MessageList};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph, Widget},
    DefaultTerminal,
};
use std::env::{self};
mod message;
mod websocket;
use tui_textarea::TextArea;

#[derive(Debug)]
struct App {
    textarea: TextArea<'static>,
    messages: MessageList,
    mode: TerminalMode,
    tx: Option<Sender<Message>>,
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
        let msgs = vec![
            Message {
                content: "Connected to websocket success".to_string(),
                kind: message::MessageKind::OUTGOING,
                time: Local::now(),
            },
            Message {
                content: "Got msg 192917929 from shastri".to_string(),
                kind: message::MessageKind::OUTGOING,
                time: Local::now(),
            },
            Message {
                content: "Disconnected from websocket".to_string(),
                kind: message::MessageKind::OUTGOING,
                time: Local::now(),
            },
        ];

        let msg_list: MessageList = MessageList::new(msgs);
        Self {
            messages: msg_list,
            exit: false,
            mode: TerminalMode::NORMAL,
            textarea: TextArea::default(),
            tx: None,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let arguments: Vec<String> = env::args().collect();
    if arguments.len() != 2 {
        eprintln!("\n Error occured. Program requires one argument ");
        eprintln!("\n Usage cargo run <url> ");
        std::process::exit(1);
    }

    let (tx, rx) = crossbeam_channel::unbounded::<Message>();
    let (tx_cl, rx_cl) = (tx.clone(), rx.clone());

    websocket::start_websocket(arguments[1].to_string(), tx_cl, rx_cl)
        .await
        .unwrap();

    let terminal = ratatui::init();
    let mut app = App::default();

    app.tx = Some(tx);

    let result = app.run(terminal);
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
            KeyCode::Char('j') => self.messages.select_next(),
            KeyCode::Char('k') => self.messages.select_previous(),
            KeyCode::Enter => self.send_curr_inp()?,
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

    fn send_curr_inp(&mut self) -> Result<()> {
        if self.textarea.lines().len() < 1 {
            return Ok(());
        };

        if let Some(tx) = &self.tx {
            tx.send(Message {
                kind: message::MessageKind::OUTGOING,
                content: self.textarea.lines().join(" "),
                time: DateTime::default(),
            })
            .unwrap();
            self.textarea.delete_line_by_head();
        }
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
        let title = Paragraph::new("Kyu-Tui")
            .style(Style::default())
            .alignment(ratatui::layout::Alignment::Center);

        let status = Paragraph::new("Connected")
            .style(Style::default())
            .alignment(Alignment::Right);

        title.render(chunks[0], buf);
        status.render(chunks[0], buf);

        // MIDDLE
        self.messages
            .render(chunks[1].inner(Margin::new(0, 1)), buf);

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
