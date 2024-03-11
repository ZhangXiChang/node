use std::{fs::File, io::Read};

use anyhow::Result;
use clap::Parser;
use quinn::{Endpoint, ServerConfig};

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
    //解析命令行参数
    let cli_args = CLIArgs::parse();
    //加载根节点证书
    let mut cert = Vec::new();
    File::open(cli_args.cert_path)?.read_to_end(&mut cert)?;
    let mut key = Vec::new();
    File::open(cli_args.key_path)?.read_to_end(&mut key)?;
    println!("根节点证书加载成功");
    //创建根节点
    let endpoint = Endpoint::server(
        ServerConfig::with_single_cert(vec![rustls::Certificate(cert)], rustls::PrivateKey(key))?,
        "0.0.0.0:10270".parse()?,
    )?;
    println!("根节点创建成功");
    //接收连接
    while let Some(connecting) = endpoint.accept().await {
        tokio::spawn(async move {
            let connection = connecting.await?;
            println!("[{}]节点连接成功", connection.remote_address());
            //返回对象节点的外部IP地址
            let mut send = connection.open_uni().await?;
            send.write_all(connection.remote_address().to_string().as_bytes())
                .await?;
            send.finish().await?;
            println!(
                "[{}]节点断开连接，原因：{}",
                connection.remote_address(),
                connection.closed().await
            );
            anyhow::Ok(())
        });
    }
    Ok(())
}
