use std::{fs::File, io::Read, sync::Arc, time::Duration};

use clap::Parser;
use eyre::Result;
use protocol::DataPacket;
use quinn::{Endpoint, ServerConfig, TransportConfig};

#[derive(Parser)]
struct CLIArgs {
    ///证书文件路径
    #[arg(long)]
    cert_path: String,
    ///私钥文件路径
    #[arg(long)]
    key_path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    //解析命令行参数
    let cli_args = CLIArgs::parse();
    //加载根节点证书
    let mut cert_der = Vec::new();
    File::open(cli_args.cert_path)?.read_to_end(&mut cert_der)?;
    let mut key_der = Vec::new();
    File::open(cli_args.key_path)?.read_to_end(&mut key_der)?;
    log::info!("根节点证书加载成功");
    //创建根节点
    let endpoint = Endpoint::server(
        ServerConfig::with_single_cert(
            vec![rustls::Certificate(cert_der)],
            rustls::PrivateKey(key_der),
        )?
        .transport_config(Arc::new({
            let mut a = TransportConfig::default();
            a.keep_alive_interval(Some(Duration::from_secs(5)));
            a
        }))
        .to_owned(),
        "0.0.0.0:10270".parse()?,
    )?;
    log::info!("根节点创建成功");
    //接收连接
    while let Some(connecting) = endpoint.accept().await {
        tokio::spawn(async move {
            match async {
                let connection = connecting.await?;
                loop {
                    match rmp_serde::from_slice::<DataPacket>(
                        &connection
                            .accept_uni()
                            .await?
                            .read_to_end(usize::MAX)
                            .await?,
                    )? {
                        DataPacket::NodeInfo { name, uuid } => {
                            log::info!("节点名称：{} 节点UUID：{}", name, uuid)
                        }
                    }
                }
                #[allow(unreachable_code)]
                eyre::Ok(())
            }
            .await
            {
                Ok(_) => (),
                Err(err) => log::error!("{}", err),
            }
        });
    }
    Ok(())
}
