use ratatui::{
    layout::Rect,
    style::{Modifier, Stylize},
    widgets::{Block, Borders, List, ListDirection},
    Frame,
};

use crate::system::widget;

#[derive(Default)]
pub struct MessageBarInfo<'a> {
    pub title: &'a str,
    pub title_modifier: Modifier,
    pub items: Vec<&'a str>,
}
pub struct MessageBar {
    title: String,
    title_modifier: Modifier,
    items: Vec<String>,
}
impl MessageBar {
    pub fn new(info: MessageBarInfo) -> Self {
        Self {
            title: info.title.to_string(),
            title_modifier: info.title_modifier,
            items: info.items.iter().map(|&s| s.to_string()).collect(),
        }
    }
    pub fn set_title_modifier(&mut self, title_modifier: Modifier) {
        self.title_modifier = title_modifier;
    }
    pub fn append(&mut self, msg: &str) {
        self.items.insert(0, msg.to_string());
    }
}
impl widget::Componect for MessageBar {
    fn draw(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_widget(
            List::new(self.items.clone())
                .direction(ListDirection::BottomToTop)
                .block(
                    Block::new()
                        .borders(Borders::ALL)
                        .title(self.title.clone().add_modifier(self.title_modifier)),
                ),
            area,
        );
    }
}
