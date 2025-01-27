use chrono::{DateTime, Local};
use color_eyre::Result;
use crossterm::event::{self,KeyCode, KeyEvent};
use events::EventHandler;
use futures_util::{
    stream::{SplitSink, SplitStream, StreamExt},
    SinkExt,
};
use message::{Message, MessageList};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph, Widget},
    DefaultTerminal,
};
use std::{
    env::{self},
    sync::{Arc, RwLock},
};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::protocol::Message as TungSteniteMsg;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tui_textarea::TextArea;
mod events;
mod message;
use events::Event;

#[derive(Debug)]
struct App {
    textarea: TextArea<'static>,
    messages: MessageList,
    mode: TerminalMode,
    write: Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, TungSteniteMsg>>,
    events: Option<EventHandler>,
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
        let msgs = vec![Message {
            content: "Connected to websocket success".to_string(),
            kind: message::MessageKind::OUTGOING,
            time: Local::now(),
        }];

        let msg_list: MessageList = MessageList::new(msgs);
        Self {
            messages: msg_list,
            exit: false,
            mode: TerminalMode::NORMAL,
            textarea: TextArea::default(),
            write: None,
            events: None,
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

    let events = events::EventHandler::new();
    let (ws_stream, _) = connect_async(arguments[1].clone()).await.unwrap();
    let (write, read) = ws_stream.split();

    let terminal = ratatui::init();
    let mut app = App::default();

    app.write = Some(write);
    app.events = Some(events);

    if let Err(_e) = listen_messages(Some(read), Arc::clone(&app.messages.messages)).await {
        std::process::exit(1);
    }

    let result = app.run(terminal);
    let _ = result.await;
    ratatui::restore();
    Ok(())
}

impl App {
    async fn run(&mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let mut event = self.events.take().unwrap();
        while !self.exit {
            let task = event.next().await;
            self.handle_events(task.unwrap()).await.unwrap();
            terminal.draw(|frame| self.draw(frame))?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut ratatui::Frame) {
        frame.render_widget(self, frame.area());
    }

    async fn handle_events(&mut self,event:Event) -> Result<()> {
        match event {
            Event::Key(key_event) => {
                if let TerminalMode::INPUT = self.mode {
                    self.handle_msg_input(key_event)?;
                    return Ok(());
                }

                self.handle_key_events(key_event).await?;
            }
            _ => {}
        }
        Ok(())
    }

    // For handling all the global keybinds
    async fn handle_key_events(&mut self, keyevent: KeyEvent) -> Result<()> {
        match keyevent.code {
            KeyCode::Char('q') => self.exit = true,
            KeyCode::Char('i') => self.mode = TerminalMode::INPUT,
            KeyCode::Char('j') => self.messages.select_next(),
            KeyCode::Char('k') => self.messages.select_previous(),
            KeyCode::Enter => self.send_curr_inp().await.unwrap(),
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

    async fn send_curr_inp(&mut self) -> Result<()> {
        if self.textarea.lines().len() < 1 {
            return Ok(());
        };
        if let Some(ws) = &mut self.write {
            let msg = Message {
                kind: message::MessageKind::OUTGOING,
                content: self.textarea.lines().join(" "),
                time: DateTime::default(),
            };
            let mut guard = self.messages.messages.write().unwrap();
            guard.push(msg.clone());
            ws.send(msg.content.into()).await.unwrap();

            self.textarea.delete_line_by_head();
            drop(guard);
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

async fn listen_messages(
    reader: Option<SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
    messages: Arc<RwLock<Vec<Message>>>,
) -> Result<()> {
    let mut reader = reader.unwrap();
    tokio::spawn(async move {
        while let Some(Ok(msg)) = reader.next().await {
            let mut lock = messages.write().unwrap();
            let info = Message {
                content: msg.to_string(),
                time: Local::now(),
                kind: message::MessageKind::INCOMING,
            };
            lock.push(info);
            drop(lock);
        }
    });
    Ok(())
}
