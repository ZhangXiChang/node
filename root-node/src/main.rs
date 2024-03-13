use std::{fs::File, io::Read, sync::Arc, time::Duration};

use anyhow::Result;
use bincode::{Decode, Encode};
use clap::Parser;
use quinn::{Endpoint, ServerConfig, TransportConfig};
use tokio::sync::Mutex;

#[derive(Encode, Decode)]
enum DataPacket {
    RequestRegisterNode {
        node_name: String,
        node_cert: Vec<u8>,
    },
    RequestGetOnlineNode,
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

struct NodeInfo {
    name: String,
    cert: Vec<u8>,
    id: usize,
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
    //在线节点储存
    let online_node = Arc::new(Mutex::new(Vec::new()));
    //接收连接
    while let Some(connecting) = endpoint.accept().await {
        let online_node = online_node.clone();
        tokio::spawn(async move {
            let connection = connecting.await?;
            println!("[{}]节点连接成功", connection.remote_address());
            //应答
            loop {
                match connection.accept_uni().await {
                    Ok(mut recv) => {
                        let data_packet: DataPacket = bincode::decode_from_slice(
                            recv.read_to_end(usize::MAX).await?.as_slice(),
                            bincode::config::standard(),
                        )?
                        .0;
                        match data_packet {
                            DataPacket::RequestRegisterNode {
                                node_name,
                                node_cert,
                            } => {
                                let mut online_node = online_node.lock().await;
                                online_node.push(NodeInfo {
                                    name: node_name,
                                    cert: node_cert,
                                    id: connection.stable_id(),
                                });
                            }
                            DataPacket::RequestGetOnlineNode => {
                                let online_node = online_node.lock().await;
                                let mut send = connection.open_uni().await?;
                                for node_info in online_node.iter() {
                                    send.write_all(node_info.name.as_bytes()).await?;
                                }
                                send.finish().await?;
                            }
                        }
                    }
                    Err(err) => {
                        println!(
                            "[{}]节点断开连接，原因：{}",
                            connection.remote_address(),
                            err
                        );
                        let mut online_node = online_node.lock().await;
                        for i in 0..online_node.len() {
                            if online_node[i].id == connection.stable_id() {
                                online_node.remove(i);
                            }
                        }
                        break;
                    }
                }
            }
            anyhow::Ok(())
        });
    }
    Ok(())
}
