use ratatui::{
    layout::{Alignment, Rect},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Default)]
pub struct TitleBarInfo<'a> {
    pub title: &'a str,
}
pub struct TitleBar {
    title: String,
}
impl TitleBar {
    pub fn new(info: TitleBarInfo) -> Self {
        Self {
            title: info.title.to_string(),
        }
    }
    pub fn draw(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(
            Paragraph::new(self.title.clone())
                .block(Block::new().borders(Borders::ALL))
                .alignment(Alignment::Center),
            area,
        );
    }
}
