use std::{fs::File, io::Read, sync::Arc, time::Duration};

use anyhow::Result;
use clap::Parser;
use quinn::{Endpoint, ServerConfig, TransportConfig};
use tokio::sync::Mutex;

enum Message {
    Undefined,
    RequestOnlineNode,
}
impl From<Vec<u8>> for Message {
    fn from(value: Vec<u8>) -> Self {
        match value[0] {
            0x00 => Message::RequestOnlineNode,
            _ => Message::Undefined,
        }
    }
}

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
    let mut transport_config = TransportConfig::default();
    transport_config.keep_alive_interval(Some(Duration::from_secs(5)));
    let endpoint = Endpoint::server(
        ServerConfig::with_single_cert(vec![rustls::Certificate(cert)], rustls::PrivateKey(key))?
            .transport_config(Arc::new(transport_config))
            .clone(),
        "0.0.0.0:10270".parse()?,
    )?;
    println!("根节点创建成功");
    //在线节点
    let online_connection = Arc::new(Mutex::new(Vec::new()));
    //接收连接
    while let Some(connecting) = endpoint.accept().await {
        let online_connection = online_connection.clone();
        tokio::spawn(async move {
            let connection = connecting.await?;
            {
                let mut online_connection = online_connection.lock().await;
                online_connection.push(connection.clone());
            }
            println!("[{}]节点连接成功", connection.remote_address());
            //应答
            loop {
                match connection.accept_uni().await {
                    Ok(mut recv) => match Message::from(recv.read_to_end(usize::MAX).await?) {
                        Message::RequestOnlineNode => match connection.open_uni().await {
                            Ok(mut send) => {
                                let online_connection = online_connection.lock().await;
                                for connection in online_connection.iter() {
                                    connection
                                        .peer_identity()
                                        .unwrap()
                                        .downcast::<rustls::Certificate>()
                                        .unwrap();
                                    send.write_all("".as_bytes()).await?;
                                }
                                send.finish().await?;
                            }
                            Err(_) => (),
                        },
                        _ => (),
                    },
                    Err(err) => {
                        println!(
                            "[{}]节点断开连接，原因：{}",
                            connection.remote_address(),
                            err
                        );
                        break;
                    }
                }
            }
            {
                let mut online_connection = online_connection.lock().await;
                for i in 0..online_connection.len() {
                    if online_connection[i].remote_address() == connection.remote_address() {
                        online_connection.remove(i);
                    }
                }
            }
            anyhow::Ok(())
        });
    }
    Ok(())
}
