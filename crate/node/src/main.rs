mod gui;
mod node;
mod system;
mod window;

use eyre::Result;
use gui::GUInterface;
use system::System;
use window::Window;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    Window::new(GUInterface::new(System::new()?)?)?;
    Ok(())
}
