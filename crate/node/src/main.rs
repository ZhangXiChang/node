mod ui;

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use fltk::{
    app::App,
    prelude::{InputExt, WidgetExt},
};
use quinn::{Endpoint, ServerConfig, TransportConfig};

#[derive(Default)]
struct System {
    endpoint: Option<Endpoint>,
}
impl System {
    fn run(mut self) -> Result<()> {
        let app = App::default();
        let mut ui = ui::MainWindow::new();
        ui.root_node_connect_button.set_callback(move |_| {
            match self.create_endpoint(ui.node_name_text_input.value()) {
                Ok(_) => println!("节点创建成功"),
                Err(err) => println!("{}", err),
            }
        });
        ui.window.show();
        app.run()?;
        Ok(())
    }
    fn create_endpoint(&mut self, node_name: String) -> Result<()> {
        //创建证书
        let cert =
            rcgen::Certificate::from_params(rcgen::CertificateParams::new(
                vec![node_name.clone()],
            ))?;
        //配置连接
        let mut transport_config = TransportConfig::default();
        transport_config.keep_alive_interval(Some(Duration::from_secs(5)));
        //创建节点
        self.endpoint = Some(Endpoint::server(
            ServerConfig::with_single_cert(
                vec![rustls::Certificate(cert.serialize_der()?)],
                rustls::PrivateKey(cert.serialize_private_key_der()),
            )?
            .transport_config(Arc::new(transport_config))
            .clone(),
            "0.0.0.0:0".parse()?,
        )?);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    System::default().run()?;
    Ok(())
}
