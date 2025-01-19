use chrono::{DateTime, Local};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    text::Text,
    widgets::{Block, Borders, List, ListItem, ListState, Widget},
};
use std::fmt::{self, Display};

pub enum MessageKind {
    OUTGOING,
    INCOMING,
}

impl Display for MessageKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageKind::OUTGOING => write!(f, "^"),
            MessageKind::INCOMING => write!(f, "V"),
        }
    }
}

pub struct Message {
    pub content: String,
    pub kind: MessageKind,
    pub time: DateTime<Local>,
}

pub struct MessageList {
    pub messages: Vec<Message>,
    pub state: ListState,
}

impl MessageList {
    pub fn new(messages: Vec<Message>) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        MessageList { messages, state }
    }

    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.messages.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.messages.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

impl Widget for &MessageList {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self
            .messages
            .iter()
            .map(|msg| {
                let layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(2),
                        Constraint::Percentage(94),
                        Constraint::Percentage(3),
                    ])
                    .split(area);

                let type_width = layout[0].width as usize;
                let content_width = layout[1].width as usize;
                let time_width = layout[2].width as usize;

                let line = format!(
                    "{:<type_width$}{:^content_width$}{:>time_width$}",
                    msg.kind,
                    msg.content,
                    msg.time.format("%H:%M"),
                    type_width = type_width,
                    content_width = content_width,
                    time_width = time_width,
                );

                ListItem::new(Text::raw(format!("{}\n", line)))
            })
            .collect();

        List::new(items)
            .block(Block::default().title("Messages").borders(Borders::ALL))
            .render(area, buf);
    }
}




