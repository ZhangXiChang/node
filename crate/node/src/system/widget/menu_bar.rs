use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style, Stylize},
    widgets::{Block, Borders, List, ListState},
    Frame,
};
use share::ArcMutex;

use crate::system::widget;

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
impl widget::Componect for MenuBar {
    fn draw(&mut self, frame: &mut Frame, area: Rect) {
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
    fn event(
        &mut self,
        system: ArcMutex<crate::system::System>,
        event: &Event,
    ) -> eyre::Result<()> {
        match event {
            Event::Key(key) => match key.kind {
                KeyEventKind::Press => match key.code {
                    KeyCode::Up => self.up_select(),
                    KeyCode::Down => self.down_select(),
                    KeyCode::Enter => {
                        if let Some(selected) = self.selected() {
                            match selected {
                                0 => (),
                                1 => (),
                                2 => system.lock().quit(),
                                _ => (),
                            }
                        }
                    }
                    _ => (),
                },
                _ => (),
            },
            _ => (),
        }
        Ok(())
    }
}
