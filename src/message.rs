use chrono::{DateTime, Local};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{palette::tailwind::SLATE, Color, Stylize},
    widgets::{HighlightSpacing, List, ListItem, ListState, StatefulWidget, Widget},
};
use std::fmt::{self, Display};



#[derive(Debug, Clone)]
pub enum MessageKind {
    OUTGOING,
    INCOMING,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub content: String,
    pub kind: MessageKind,
    pub time: DateTime<Local>,
}

#[derive(Default, Debug)]
pub struct MessageList {
    pub messages: Vec<Message>,
    pub state: ListState,
}


const NORMAL_ROW_BG: Color = SLATE.c950;
const ALT_ROW_BG_COLOR: Color = SLATE.c900;


const fn alternate_colors(i: usize) -> Color {
    if i % 2 == 0 {
        NORMAL_ROW_BG
    } else {
        ALT_ROW_BG_COLOR
    }
}

impl From<&Message> for ListItem<'_> {
    fn from(msg: &Message) -> Self {
        let line = format!("{}{}{}", msg.kind, msg.content, msg.time.format("%H:%M"));
        ListItem::new(line)
    }
}


impl Display for MessageKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MessageKind::OUTGOING => write!(f, "⬆"),
            MessageKind::INCOMING => write!(f, "⬇"),
        }
    }
}


impl MessageList {
    pub fn new(messages: Vec<Message>) -> Self {
        let mut state = ListState::default();
        *state.offset_mut() = 0;
        state.select(Some(1));
        MessageList { messages, state }
    }

    pub fn select_next(&mut self) {
        self.state.select_next();
    }
    pub fn select_previous(&mut self) {
        self.state.select_previous();
    }
}


impl Widget for &mut MessageList {

    fn render(self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self
            .messages
            .iter()
            .enumerate()
            .map(|(i, message)| {
                let color = alternate_colors(i);
                ListItem::from(message).bg(color)
            })
            .collect();

        let list = List::new(items)
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(list, area, buf, &mut self.state);
    }
}
