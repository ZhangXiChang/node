mod gui;
mod window;

use eyre::Result;
use gui::GUI;

#[tokio::main]
async fn main() -> Result<()> {
    window::new(
        window::Info {
            app_name: Some("节点网络"),
            inner_size: Some(window::Size {
                width: 500.,
                height: 500. + 50.,
            }),
            resizable: Some(false),
            maximize_button: Some(false),
            ..Default::default()
        },
        GUI::new()?,
    )?;
    Ok(())
}
