use std::{fs::File, io::Read, sync::Arc, time::Duration};

use anyhow::Result;
use clap::Parser;
use node_network::{DataPacket, NodeAddrAndCert, RequestDataPacket, ResponseDataPacket};
use quinn::{Connection, Endpoint, ServerConfig, TransportConfig};
use tokio::sync::Mutex;

#[derive(Parser)]
struct CLIArgs {
    ///证书文件路径
    #[arg(long)]
    cert_path: String,
    ///私钥文件路径
    #[arg(long)]
    key_path: String,
}

struct Node {
    name: String,
    connection: Connection,
    cert: Vec<u8>,
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
    //注册的节点列表
    let node_list = Arc::new(Mutex::new(Vec::new()));
    //接收连接
    while let Some(connecting) = endpoint.accept().await {
        let node_list = node_list.clone();
        tokio::spawn(async move {
            let connection = connecting.await?;
            println!("[{}]节点连接成功", connection.remote_address());
            //应答
            loop {
                match connection.accept_bi().await {
                    Ok((mut send, mut recv)) => match rmp_serde::from_slice::<DataPacket>(
                        &recv.read_to_end(usize::MAX).await?,
                    )? {
                        DataPacket::RegisterNode {
                            node_name,
                            node_cert,
                        } => {
                            let mut node_list = node_list.lock().await;
                            node_list.push(Node {
                                name: node_name,
                                connection: connection.clone(),
                                cert: node_cert,
                            });
                        }
                        DataPacket::Request(RequestDataPacket::GetRegisteredNodeNameList) => {
                            let node_list = node_list.lock().await;
                            let mut registered_node_name_list = Vec::new();
                            for node in node_list.iter() {
                                registered_node_name_list.push(node.name.clone());
                            }
                            send.write_all(&rmp_serde::to_vec(&DataPacket::Response(
                                ResponseDataPacket::GetRegisteredNodeNameList {
                                    registered_node_name_list,
                                },
                            ))?)
                            .await?;
                            send.finish().await?;
                        }
                        DataPacket::Request(RequestDataPacket::GetRegisteredNodeAddrAndCert {
                            node_name,
                        }) => {
                            let node_list = node_list.lock().await;
                            let mut node = None;
                            for i in node_list.iter() {
                                if i.name == node_name {
                                    node = Some(i);
                                }
                            }
                            if let Some(node) = node {
                                let mut node_send = node.connection.open_uni().await?;
                                node_send
                                    .write_all(&rmp_serde::to_vec(&DataPacket::Request(
                                        RequestDataPacket::HolePunching {
                                            addr: connection.remote_address(),
                                        },
                                    ))?)
                                    .await?;
                                node_send.finish().await?;
                                match node.connection.accept_uni().await {
                                    Ok(mut node_recv) => match rmp_serde::from_slice::<DataPacket>(
                                        &node_recv.read_to_end(usize::MAX).await?,
                                    )? {
                                        DataPacket::Response(ResponseDataPacket::HolePunching) => {
                                            let node_addr_and_cert = DataPacket::Response(
                                                ResponseDataPacket::GetRegisteredNodeAddrAndCert(
                                                    Ok(NodeAddrAndCert {
                                                        addr: node.connection.remote_address(),
                                                        cert: node.cert.clone(),
                                                    }),
                                                ),
                                            );
                                            send.write_all(&rmp_serde::to_vec(
                                                &node_addr_and_cert,
                                            )?)
                                            .await?;
                                            send.finish().await?;
                                        }
                                        _ => (),
                                    },
                                    Err(_) => (),
                                }
                            } else {
                                send.write_all(&rmp_serde::to_vec(&DataPacket::Response(
                                    ResponseDataPacket::GetRegisteredNodeAddrAndCert(Err(format!(
                                        "没有找到叫[{}]的节点等待连接",
                                        node_name
                                    ))),
                                ))?)
                                .await?;
                                send.finish().await?;
                            }
                        }
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
            let mut node_list = node_list.lock().await;
            for i in 0..node_list.len() {
                if node_list[i].connection.stable_id() == connection.stable_id() {
                    node_list.remove(i);
                }
            }
            anyhow::Ok(())
        });
    }
    Ok(())
}
