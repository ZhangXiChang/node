mod app;
mod window;

use eyre::Result;

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
        app::App::new()?,
    )?;
    Ok(())
}
