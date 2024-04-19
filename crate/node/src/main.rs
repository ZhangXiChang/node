mod node;
mod system;
mod widget;
mod window;

use eyre::Result;
use system::System;
use widget::Widget;
use window::Window;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    Window::new(Widget::new(System::new()?)?)?;
    Ok(())
}
