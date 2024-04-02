pub mod widget;

use std::time::Duration;

use crossterm::event::{self, Event};
use eyre::Result;
use share::ArcMutex;

#[allow(unused_variables)]
pub trait Componect {
    fn startup(&mut self, system: ArcMutex<System>) -> Result<()> {
        Ok(())
    }
    fn update(&mut self, system: ArcMutex<System>, event: &Option<Event>) -> Result<()> {
        Ok(())
    }
    fn end(&mut self, system: ArcMutex<System>) -> Result<()> {
        Ok(())
    }
}

pub struct System {
    is_run: bool,
    components: Vec<ArcMutex<Box<dyn Componect>>>,
}
impl System {
    pub fn new(components: Vec<ArcMutex<Box<dyn Componect>>>) -> Self {
        Self {
            is_run: true,
            components,
        }
    }
    pub fn run(self) -> Result<()> {
        let arc_self = ArcMutex::new(self);
        for component in {
            let a = arc_self.lock().components.clone();
            a
        }
        .iter_mut()
        {
            component.lock().startup(arc_self.clone())?;
        }
        while {
            let a = arc_self.lock().is_run;
            a
        } {
            let mut event = None;
            if event::poll(Duration::ZERO)? {
                event = Some(event::read()?);
            }
            for component in {
                let a = arc_self.lock().components.clone();
                a
            }
            .iter_mut()
            {
                component.lock().update(arc_self.clone(), &event)?;
            }
        }
        for component in {
            let a = arc_self.lock().components.clone();
            a
        }
        .iter_mut()
        {
            component.lock().end(arc_self.clone())?;
        }
        Ok(())
    }
    pub fn quit(&mut self) {
        self.is_run = false;
    }
}
