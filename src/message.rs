use chrono::{DateTime, Local};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{
        palette::{material::{GREEN, RED}, tailwind::SLATE},
        Color, Stylize,
    },
    text::{Line, Span},
    widgets::{
        Block, Borders, HighlightSpacing, List, ListItem, ListState, StatefulWidget, Widget,
    },
};
use std::{
    fmt::{self, Display},
    sync::{Arc, RwLock},
};




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
    pub messages: Arc<RwLock<Vec<Message>>>,
    pub state: ListState,
}

const NORMAL_ROW_BG: Color = SLATE.c900;
const ALT_ROW_BG_COLOR: Color = SLATE.c950;

impl From<&Message> for ListItem<'_> {
    fn from(msg: &Message) -> Self {
        let terminal_width = match crossterm::terminal::size() {
            Ok((width, _)) => width as usize,
            Err(_) => 80,
        };

        let kind_width = 12;
        let time_width = 10;
        let content_width = terminal_width.saturating_sub(kind_width + time_width + 2);

        let status_txt = format!(
            "{:<kind_width$}",
            msg.kind.to_string(),
            kind_width = kind_width
        );
        let status = match msg.kind {
            MessageKind::OUTGOING => Span::styled(status_txt, RED.c900),
            MessageKind::INCOMING => Span::styled(status_txt, GREEN.c900),
        };

        let content = Span::styled(
            format!(
                "{:^content_width$}",
                msg.content,
                content_width = content_width
            ),
            SLATE.c100,
        );

        let time = Span::styled(
            format!(
                "{:<time_width$}",
                msg.time.format("%H:%M"),
                time_width = time_width
            ),
            SLATE.c100,
        );

        let line = Line::from(vec![status, content, time]);
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
        state.select(Some(0));
        MessageList {
            messages: Arc::new(RwLock::new(messages)),
            state,
        }
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
            .read()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, message)| {
                if let Some(selected_idx) = self.state.selected() {
                    if i == selected_idx {
                        ListItem::from(message).bg(ALT_ROW_BG_COLOR)
                    } else {
                        ListItem::from(message).bg(NORMAL_ROW_BG)
                    }
                } else {
                    ListItem::from(message).bg(ALT_ROW_BG_COLOR)
                }
            })
            .collect();

        let list = List::new(items)
            .block(Block::new().title("Messages").borders(Borders::ALL))
            .highlight_symbol(">")
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(list, area, buf, &mut self.state);
    }
}
