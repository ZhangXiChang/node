mod system;
mod window;

use eyre::Result;
use system::System;
use window::Window;

const ICON_FILE_DATA: &[u8] = include_bytes!("../assets/icon/node_icon.png");
const ICON_WIDTH: f32 = 512.;
const ICON_HEIGHT: f32 = 512.;
const FONT_FILE_DATA: &[u8] = include_bytes!("../assets/fonts/SourceHanSansCN-Bold.otf");

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    Window::new(System::new()?)?;
    Ok(())
}
