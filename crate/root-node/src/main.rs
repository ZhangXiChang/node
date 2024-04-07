use std::{fs::File, io::Read, sync::Arc, time::Duration};

use clap::Parser;
use eyre::Result;
use log::{info, LevelFilter};
use protocol::{DataPacket, NodeIPAddrAndCert, RequestDataPacket, ResponseDataPacket};
use quinn::{Connection, Endpoint, ServerConfig, TransportConfig};
use share::ArcMutex;

#[derive(Parser)]
struct CLIArgs {
    ///证书文件路径
    #[arg(long)]
    cert_path: String,
    ///私钥文件路径
    #[arg(long)]
    key_path: String,
}

#[derive(Clone)]
struct Node {
    name: String,
    cert: Vec<u8>,
    connection: Connection,
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
    //节点列表
    let online_node_list = ArcMutex::new(Vec::new());
    //接收连接
    while let Some(connecting) = endpoint.accept().await {
        let online_node_list = online_node_list.clone();
        tokio::spawn(async move {
            let connection = connecting.await?;
            info!("[{}]节点连接成功", connection.remote_address());
            //应答
            loop {
                match connection.accept_bi().await {
                    Ok((mut send, mut recv)) => {
                        match rmp_serde::from_slice::<DataPacket>(
                            &recv.read_to_end(usize::MAX).await?,
                        )? {
                            DataPacket::Request(RequestDataPacket::GetRootNodeInfo) => {
                                send.write_all(&rmp_serde::to_vec(&DataPacket::Response(
                                    ResponseDataPacket::GetRootNodeInfo {
                                        name: "北方通信".to_string(),
                                        description: "欢迎使用北方通信".to_string(),
                                    },
                                ))?)
                                .await?;
                                send.finish().await?;
                            }
                            DataPacket::RegisterNode {
                                self_name,
                                self_cert,
                            } => {
                                online_node_list.lock().push(Node {
                                    name: self_name,
                                    cert: self_cert,
                                    connection: connection.clone(),
                                });
                            }
                            DataPacket::UnRegisterNode => {
                                for i in {
                                    let a = online_node_list.lock().len();
                                    0..a
                                } {
                                    if {
                                        let a = online_node_list.lock()[i].connection.stable_id();
                                        a
                                    } == connection.stable_id()
                                    {
                                        online_node_list.lock().remove(i);
                                    }
                                }
                            }
                            DataPacket::Request(RequestDataPacket::GetOnlineNodeNameList) => {
                                let mut online_node_name_list = Vec::new();
                                {
                                    for online_node in {
                                        let a = online_node_list.lock().clone();
                                        a
                                    } {
                                        online_node_name_list.push(online_node.name);
                                    }
                                }
                                send.write_all(&rmp_serde::to_vec(&DataPacket::Response(
                                    ResponseDataPacket::GetOnlineNodeNameList {
                                        online_node_name_list,
                                    },
                                ))?)
                                .await?;
                                send.finish().await?;
                            }
                            DataPacket::Request(
                                RequestDataPacket::GetRegisteredNodeIPAddrAndCert {
                                    self_name,
                                    object_name,
                                },
                            ) => {
                                let mut object_node = None;
                                for online_node in {
                                    let a = online_node_list.lock().clone();
                                    a
                                } {
                                    if online_node.name == object_name {
                                        object_node = Some(online_node);
                                        break;
                                    }
                                }
                                if let Some(object_node) = object_node {
                                    let (mut object_node_send, mut object_node_recv) =
                                        object_node.connection.open_bi().await?;
                                    object_node_send
                                        .write_all(&rmp_serde::to_vec(&DataPacket::Request(
                                            RequestDataPacket::HolePunching {
                                                object_name: self_name,
                                                object_socket_addr: connection.remote_address(),
                                            },
                                        ))?)
                                        .await?;
                                    object_node_send.finish().await?;
                                    match rmp_serde::from_slice::<DataPacket>(
                                        &object_node_recv.read_to_end(usize::MAX).await?,
                                    )? {
                                        DataPacket::Response(ResponseDataPacket::HolePunching) => {
                                            let node_ip_addr_and_cert_data_packet =
                                            DataPacket::Response(
                                                ResponseDataPacket::GetRegisteredNodeIPAddrAndCert(
                                                    Some(NodeIPAddrAndCert {
                                                        socket_addr: object_node.connection.remote_address(),
                                                        cert: object_node.cert.clone(),
                                                    }),
                                                ),
                                            );
                                            send.write_all(&rmp_serde::to_vec(
                                                &node_ip_addr_and_cert_data_packet,
                                            )?)
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
                        }
                    }
                    Err(err) => {
                        info!(
                            "[{}]节点断开连接，原因：{}",
                            connection.remote_address(),
                            err
                        );
                        break;
                    }
                }
            }
            for i in {
                let a = online_node_list.lock().len();
                0..a
            } {
                if {
                    let a = online_node_list.lock()[i].connection.stable_id();
                    a
                } == connection.stable_id()
                {
                    online_node_list.lock().remove(i);
                }
            }
            eyre::Ok(())
        });
    }
    Ok(())
}
