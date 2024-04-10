use std::{fs::File, io::Read, sync::Arc, time::Duration};

use clap::Parser;
use eyre::Result;
use log::{info, LevelFilter};
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
    //初始化日志
    env_logger::builder().filter_level(LevelFilter::Info).init();
    //解析命令行参数
    let cli_args = CLIArgs::parse();
    //加载根节点证书
    let mut cert = Vec::new();
    File::open(cli_args.cert_path)?.read_to_end(&mut cert)?;
    let mut key = Vec::new();
    File::open(cli_args.key_path)?.read_to_end(&mut key)?;
    info!("根节点证书加载成功");
    //创建根节点
    let mut transport_config = TransportConfig::default();
    transport_config.keep_alive_interval(Some(Duration::from_secs(5)));
    let endpoint = Endpoint::server(
        ServerConfig::with_single_cert(
            vec![rustls::Certificate(cert.clone())],
            rustls::PrivateKey(key.clone()),
        )?
        .transport_config(Arc::new(transport_config))
        .clone(),
        "0.0.0.0:10270".parse()?,
    )?;
    info!("根节点创建成功");
    //接收连接
    while let Some(connecting) = endpoint.accept().await {
        tokio::spawn(async move {
            let connection = connecting.await?;
            info!("[{}]节点连接成功", connection.remote_address());
            info!(
                "[{}]断开连接，原因：{}",
                connection.remote_address(),
                connection.closed().await
            );
            eyre::Ok(())
        });
    }
    Ok(())
}
