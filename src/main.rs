use chrono::{Local,DateTime};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent};
use events::EventHandler;
use futures_util::{
    stream::{SplitSink, SplitStream, StreamExt},
    SinkExt,
};
use message::{Message, MessageList};
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Flex, Layout, Margin, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
    DefaultTerminal,
};
use std::{
    collections::VecDeque,
    env::{self},
    mem,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::protocol::Message as TungSteniteMsg;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tui_textarea::TextArea;
mod events;
mod message;
use events::Event;
use serde_json::Value;

const SEQUENCE_TIMEOUT: Duration = Duration::from_millis(500);
#[derive(Debug, PartialEq)]
pub enum ContentType {
    Text(String),
    Json(Value),
    Binary(Vec<u8>),
}

#[derive(Debug)]
struct App {
    textarea: TextArea<'static>,
    messages: MessageList,
    mode: TerminalMode,
    write: Option<SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, TungSteniteMsg>>,
    events: Option<EventHandler>,
    last_key_time: Option<Instant>,
    key_buffer: Vec<KeyCode>,
    popup: bool,
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
        let msg_list: MessageList = MessageList::new(VecDeque::new());
        Self {
            messages: msg_list,
            exit: false,
            mode: TerminalMode::NORMAL,
            textarea: TextArea::default(),
            write: None,
            events: None,
            last_key_time: None,
            key_buffer: Vec::new(),
            popup: false,
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
        if self.popup {
            self.show_msg_info_popup(frame, frame.area());
            return;
        }
        frame.render_widget(self, frame.area());
    }

    fn show_msg_info_popup(&mut self, frame: &mut ratatui::Frame, area: Rect) {
        let guard = self.messages.messages.read().unwrap();

        let selected = self.messages.state.selected().unwrap();
        let msg = guard.get(selected).unwrap();
        let content_type = detect_content_type(&msg.content);

        let content_length = match &content_type {
            ContentType::Text(msg) => msg.len(),
            ContentType::Json(msg) => calculate_json_size(&msg),
            ContentType::Binary(msg) => msg.len(),
        };

        let kind = match msg.kind {
            message::MessageKind::OUTGOING => "Outgoing".to_string(),
            message::MessageKind::INCOMING => "Incomming".to_string(),
        };

        let sel_max_ratio = format!("Info: {}/{}",selected+1,guard.len());
        let area_b = popup_area(frame.area(), 60, 30);
        
        let block = Block::bordered().title(sel_max_ratio);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(1),
            ])
            .split(area_b);

        frame.render_widget(Clear, area_b); // Add this to clear the area first not the terminal
        frame.render_widget(block, area_b);

        let message_type = Paragraph::new(vec![Line::from(vec![
            Span::styled("Message Type: ", Style::default().fg(Color::Yellow)),
            Span::from(kind),
        ])]);

        let type_str = match content_type {
            ContentType::Text { .. } => "Text",
            ContentType::Json { .. } => "JSON",
            ContentType::Binary { .. } => "Binary",
        };

        let cnt_type = Paragraph::new(vec![Line::from(vec![
            Span::styled("Content Type", Style::default().fg(Color::Yellow)),
            Span::from(type_str),
        ])]);

        let size_para = Paragraph::new(vec![Line::from(vec![
            Span::styled("Size: ", Style::default().fg(Color::Yellow)),
            Span::from(format!("{} bytes", content_length)),
        ])]);

        let date_para = Paragraph::new(vec![Line::from(vec![
            Span::styled("Time: ", Style::default().fg(Color::Yellow)),
            Span::from(format!("{} ", msg.time.to_string())),
        ])]);

        let preview_text = match &content_type {
            ContentType::Text(content) => content.chars().take(100).collect::<String>(),
            ContentType::Json(content) => serde_json::to_string_pretty(content)
                .unwrap_or_default()
                .chars()
                .take(100)
                .collect::<String>(),
            ContentType::Binary(content) => {
                format!("{:?}", &content.iter().take(20).collect::<Vec<_>>())
            }
        };

        let preview = Paragraph::new(vec![
            Line::from(vec![Span::styled(
                "Preview: ",
                Style::default().fg(Color::Yellow),
            )]),
            Line::from(vec![Span::from(preview_text + "...")]),
        ])
        .wrap(Wrap { trim: true });

        frame.render_widget(message_type, chunks[0]);
        frame.render_widget(cnt_type, chunks[1]);
        frame.render_widget(size_para, chunks[2]);
        frame.render_widget(date_para, chunks[3]);
        frame.render_widget(preview, chunks[4]);

        return;
    }

    async fn handle_events(&mut self, event: Event) -> Result<()> {
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
            KeyCode::Char('j') => {
                if self.messages.state.selected().unwrap() != self.messages.messages.read().unwrap().len() -1 {
                    self.messages.select_next()
                }
            },
            KeyCode::Char('k') => {
                if self.messages.state.selected().unwrap() > 0 {
                    self.messages.select_previous()
                }
            }
            KeyCode::Char('G') => self.messages.state.select_last(),
            KeyCode::Char('O') => self.popup = !self.popup,
            KeyCode::Enter => self.send_curr_inp().await.unwrap(),
            _ => {}
        }

        if let Some(last_time) = self.last_key_time {
            if last_time.elapsed() > SEQUENCE_TIMEOUT {
                self.key_buffer.clear();
            }
        }

        self.last_key_time = Some(Instant::now());
        self.key_buffer.push(keyevent.code);

        if self.key_buffer.len() > 2 {
            self.key_buffer.clear();
            return Ok(());
        }
        if self.key_buffer == [KeyCode::Char('g'), KeyCode::Char('g')] {
            self.messages.state.select_first();
            self.key_buffer.clear();
            return Ok(());
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
        if self.textarea.lines().join("").len() < 1 {
            return Ok(());
        };
        if let Some(ws) = &mut self.write {
            let msg = Message {
                kind: message::MessageKind::OUTGOING,
                content: self.textarea.lines().join(" "),
                time: Local::now(),
            };

            ws.send(msg.clone().content.into()).await.unwrap();
            let mut guard = self.messages.messages.write().unwrap();
            guard.push_front(msg);
            self.messages.state.select_next();
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
        let title = Paragraph::new("ZapPulse")
            .style(Style::default())
            .alignment(ratatui::layout::Alignment::Center);

        let status = Paragraph::new("🌐 Connected")
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
    messages: Arc<RwLock<VecDeque<Message>>>,
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
            lock.push_front(info);
            drop(lock);
        }
    });
    Ok(())
}

pub fn detect_content_type(input: &str) -> ContentType {
    if let Ok(parsed) = serde_json::from_str::<Value>(input) {
        return ContentType::Json(parsed);
    }

    let binary_indicators = ['\0', '\x1B'];

    let control_char_threshold = 0.1;
    let control_chars = input
        .chars()
        .filter(|&c| c.is_control() && !c.is_whitespace())
        .count();
    let control_ratio = control_chars as f64 / input.len() as f64;

    if input.chars().any(|c| binary_indicators.contains(&c))
        || control_ratio > control_char_threshold
    {
        return ContentType::Binary(input.as_bytes().to_vec());
    }
    ContentType::Text(input.to_string())
}

fn calculate_json_size(value: &Value) -> usize {
    match value {
        Value::Null => mem::size_of::<Value>(),
        Value::Bool(_) => mem::size_of::<bool>(),
        Value::Number(n) => n.to_string().len(),
        Value::String(s) => s.len(),
        Value::Array(arr) => arr.iter().map(|v| calculate_json_size(v)).sum(),
        Value::Object(map) => map
            .iter()
            .map(|(k, v)| k.len() + calculate_json_size(v))
            .sum(),
    }
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}
