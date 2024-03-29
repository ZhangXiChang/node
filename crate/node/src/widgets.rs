use ratatui::{
    layout::{Alignment, Rect},
    style::{Modifier, Style, Stylize},
    widgets::{Block, Borders, List, ListDirection, ListState, Paragraph},
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

#[derive(Default)]
pub struct MenuBarInfo<'a> {
    pub title: &'a str,
    pub title_modifier: Modifier,
    pub items: Vec<&'a str>,
    pub items_state: Option<usize>,
}
pub struct MenuBar {
    title: String,
    title_modifier: Modifier,
    items: Vec<String>,
    items_state: ListState,
}
impl MenuBar {
    pub fn new(info: MenuBarInfo) -> Self {
        Self {
            title: info.title.to_string(),
            title_modifier: info.title_modifier,
            items: info.items.iter().map(|&s| s.to_string()).collect(),
            items_state: {
                let mut a = ListState::default();
                a.select(info.items_state);
                a
            },
        }
    }
    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_stateful_widget(
            List::new(self.items.clone())
                .block(
                    Block::new()
                        .borders(Borders::ALL)
                        .title(self.title.clone().add_modifier(self.title_modifier)),
                )
                .highlight_style(Style::new().add_modifier(Modifier::BOLD))
                .highlight_symbol(">> "),
            area,
            &mut self.items_state,
        );
    }
    pub fn set_title_modifier(&mut self, title_modifier: Modifier) {
        self.title_modifier = title_modifier;
    }
    pub fn set_items(&mut self, items: Vec<&str>) {
        self.items = items.iter().map(|&s| s.to_string()).collect();
    }
    pub fn selected(&self) -> Option<usize> {
        self.items_state.selected()
    }
    pub fn select(&mut self, index: Option<usize>) {
        self.items_state.select(index);
    }
    pub fn up_select(&mut self) {
        if let Some(index) = self.selected() {
            self.select(Some(index.saturating_sub(1)));
        }
    }
    pub fn down_select(&mut self) {
        if let Some(index) = self.selected() {
            self.select(Some(index.saturating_add(1).clamp(0, self.items.len() - 1)));
        }
    }
    pub fn items(&self) -> Vec<String> {
        self.items.clone()
    }
}

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
    pub fn draw(&self, frame: &mut Frame, area: Rect) {
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
    pub fn set_title_modifier(&mut self, title_modifier: Modifier) {
        self.title_modifier = title_modifier;
    }
    pub fn append(&mut self, msg: &str) {
        self.items.insert(0, msg.to_string());
    }
}
