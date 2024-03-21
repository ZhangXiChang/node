use std::{fs::File, io::Read, sync::Arc, time::Duration};

use anyhow::Result;
use clap::Parser;
use quinn::{Connection, Endpoint, ServerConfig, TransportConfig};
use share::{DataPacket, NodeIPAddrAndCert, RequestDataPacket, ResponseDataPacket};
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
    cert: Vec<u8>,
    connection: Connection,
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
    let register_node_list = Arc::new(Mutex::new(Vec::new()));
    //接收连接
    while let Some(connecting) = endpoint.accept().await {
        let register_node_list = register_node_list.clone();
        tokio::spawn(async move {
            let connection = connecting.await?;
            println!("[{}]节点连接成功", connection.remote_address());
            //应答
            loop {
                match connection.accept_bi().await {
                    Ok((mut send, mut recv)) => match rmp_serde::from_slice::<DataPacket>(
                        &recv.read_to_end(usize::MAX).await?,
                    )? {
                        DataPacket::Request(RequestDataPacket::GetRootNodeInfo) => {
                            send.write_all(&rmp_serde::to_vec(&DataPacket::Response(
                                ResponseDataPacket::GetRootNodeInfo {
                                    root_node_name: "北方通信".to_string(),
                                    root_node_description: "欢迎使用北方通信".to_string(),
                                },
                            ))?)
                            .await?;
                            send.finish().await?;
                        }
                        DataPacket::RegisterNode { node_name, cert } => {
                            let mut register_node_list = register_node_list.lock().await;
                            register_node_list.push(Node {
                                name: node_name,
                                cert: cert,
                                connection: connection.clone(),
                            });
                        }
                        DataPacket::Request(RequestDataPacket::GetAllRegisteredNodeName) => {
                            let mut all_registered_node_name = Vec::new();
                            {
                                let register_node_list = register_node_list.lock().await;
                                for register_node in register_node_list.iter() {
                                    all_registered_node_name.push(register_node.name.clone());
                                }
                            }
                            send.write_all(&rmp_serde::to_vec(&DataPacket::Response(
                                ResponseDataPacket::GetAllRegisteredNodeName {
                                    all_registered_node_name,
                                },
                            ))?)
                            .await?;
                            send.finish().await?;
                        }
                        DataPacket::Request(
                            RequestDataPacket::GetRegisteredNodeIPAddrAndCert { node_name },
                        ) => {
                            let mut node = None;
                            let register_node_list = register_node_list.lock().await;
                            for i in register_node_list.iter() {
                                if i.name == node_name {
                                    node = Some(i);
                                    break;
                                }
                            }
                            if let Some(node) = node {
                                let (mut node_send, mut node_recv) =
                                    node.connection.open_bi().await?;
                                node_send
                                    .write_all(&rmp_serde::to_vec(&DataPacket::Request(
                                        RequestDataPacket::HolePunching {
                                            ip_addr: connection.remote_address(),
                                        },
                                    ))?)
                                    .await?;
                                node_send.finish().await?;
                                match rmp_serde::from_slice::<DataPacket>(
                                    &node_recv.read_to_end(usize::MAX).await?,
                                )? {
                                    DataPacket::Response(ResponseDataPacket::HolePunching) => {
                                        let node_ip_addr_and_cert = DataPacket::Response(
                                            ResponseDataPacket::GetRegisteredNodeIPAddrAndCert(
                                                Some(NodeIPAddrAndCert {
                                                    ip_addr: node.connection.remote_address(),
                                                    cert: node.cert.clone(),
                                                }),
                                            ),
                                        );
                                        send.write_all(&rmp_serde::to_vec(&node_ip_addr_and_cert)?)
                                            .await?;
                                        send.finish().await?;
                                    }
                                    _ => (),
                                }
                            } else {
                                send.write_all(&rmp_serde::to_vec(&DataPacket::Response(
                                    ResponseDataPacket::GetRegisteredNodeIPAddrAndCert(None),
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
            let mut register_node_list = register_node_list.lock().await;
            for i in 0..register_node_list.len() {
                if register_node_list[i].connection.stable_id() == connection.stable_id() {
                    register_node_list.remove(i);
                }
            }
            anyhow::Ok(())
        });
    }
    Ok(())
}
