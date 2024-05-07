mod node;
mod system;
mod window;

use eyre::Result;
use system::System;
use window::Window;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    Window::new(System::new()?)?;
    Ok(())
}
