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
    Window::new(GUInterface::new(System::new()?)?)?;
    Ok(())
}
