pub mod menu_bar;
pub mod message_bar;
pub mod title_bar;

use std::io::{stdout, Stdout};

use crossterm::{
    event::Event,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use eyre::Result;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Layout, Rect},
    Frame, Terminal,
};
use share::ArcMutex;

use super::System;

#[allow(unused)]
pub trait Componect {
    fn draw(&mut self, frame: &mut Frame, area: Rect) {}
    fn event(&mut self, system: ArcMutex<System>, event: &Event) -> Result<()> {
        Ok(())
    }
}

pub struct WidgetLayout {
    pub layout: Layout,
    pub widgets: Vec<(Box<dyn Componect>, usize)>,
    pub sub_widget_layout: Option<(Box<WidgetLayout>, usize)>,
}

pub struct Widget {
    term: Terminal<CrosstermBackend<Stdout>>,
    widget_layout: WidgetLayout,
}
impl Widget {
    pub fn new(widget_layout: WidgetLayout) -> Result<Self> {
        Ok(Self {
            term: Terminal::new(CrosstermBackend::new(stdout()))?,
            widget_layout,
        })
    }
    fn enter_alternate_screen() -> Result<()> {
        stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;
        Ok(())
    }
    fn leave_alternate_screen() -> Result<()> {
        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;
        Ok(())
    }
    fn poll(&mut self, system: ArcMutex<System>, event: &Option<Event>) -> Result<()> {
        self.term
            .draw(|frame| Self::draw_poll(&mut self.widget_layout, frame, frame.size()))?;
        if let Some(event) = event {
            Self::event_poll(&mut self.widget_layout, system, event)?;
        }
        Ok(())
    }
    fn draw_poll(widget_layout: &mut WidgetLayout, frame: &mut Frame, area: Rect) {
        let layout = widget_layout.layout.split(area);
        for (widget, i) in widget_layout.widgets.iter_mut() {
            widget.draw(frame, layout[*i]);
        }
        if let Some((widget_layout, i)) = &mut widget_layout.sub_widget_layout {
            Self::draw_poll(widget_layout, frame, layout[*i]);
        }
    }
    fn event_poll(
        widget_layout: &mut WidgetLayout,
        system: ArcMutex<System>,
        event: &Event,
    ) -> Result<()> {
        for (widget, _) in widget_layout.widgets.iter_mut() {
            widget.event(system.clone(), event)?;
        }
        if let Some((widget_layout, _)) = &mut widget_layout.sub_widget_layout {
            Self::event_poll(widget_layout, system, event)?;
        }
        Ok(())
    }
}
impl super::Componect for Widget {
    fn startup(&mut self, _system: ArcMutex<System>) -> Result<()> {
        Self::enter_alternate_screen()?;
        Ok(())
    }
    fn update(&mut self, system: ArcMutex<System>, event: &Option<Event>) -> Result<()> {
        self.poll(system, event)?;
        Ok(())
    }
    fn end(&mut self, _system: ArcMutex<System>) -> Result<()> {
        Self::leave_alternate_screen()?;
        Ok(())
    }
}
