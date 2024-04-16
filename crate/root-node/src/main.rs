use std::{fs::File, io::Read, sync::Arc, time::Duration};

use clap::Parser;
use eyre::Result;
use protocol::{DataPacket, NodeRegisterInfo, Request, Response};
use quinn::{Connection, Endpoint, ServerConfig, TransportConfig};
use share_code::lock::ArcMutex;

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
    register_info: NodeRegisterInfo,
    connection: Connection,
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
    let mut transport_config = TransportConfig::default();
    transport_config.keep_alive_interval(Some(Duration::from_secs(5)));
    let endpoint = Endpoint::server(
        ServerConfig::with_single_cert(
            vec![rustls::Certificate(cert_der)],
            rustls::PrivateKey(key_der),
        )?
        .transport_config(Arc::new(transport_config))
        .to_owned(),
        "0.0.0.0:10270".parse()?,
    )?;
    log::info!("根节点创建成功");
    //在线节点列表
    let register_node_list = ArcMutex::new(Vec::new());
    //接收连接
    while let Some(connecting) = endpoint.accept().await {
        let register_node_list = register_node_list.clone();
        tokio::spawn(async move {
            let connection = connecting.await?;
            log::info!("[{}]节点连接成功", connection.remote_address());
            loop {
                match connection.accept_bi().await {
                    Ok((mut send, mut recv)) => match rmp_serde::from_slice::<DataPacket>(
                        &recv.read_to_end(usize::MAX).await?,
                    )? {
                        DataPacket::Request(Request::RegisterNode(register_info)) => {
                            register_node_list.lock().push(Node {
                                register_info,
                                connection: connection.clone(),
                            });
                        }
                        DataPacket::Request(Request::UnregisterNode) => {
                            let mut register_node_list = register_node_list.lock();
                            for i in 0..register_node_list.len() {
                                if register_node_list[i].connection.stable_id()
                                    == connection.stable_id()
                                {
                                    register_node_list.remove(i);
                                }
                            }
                        }
                        DataPacket::Request(Request::RegisterNodeInfoList) => {
                            send.write_all(&rmp_serde::to_vec(&DataPacket::Response(
                                Response::RegisterNodeInfoList({
                                    let a = register_node_list
                                        .lock()
                                        .iter()
                                        .map(|v| v.register_info.node_info.clone())
                                        .collect();
                                    a
                                }),
                            ))?)
                            .await?;
                            send.finish().await?;
                        }
                        _ => (),
                    },
                    Err(err) => {
                        {
                            let mut register_node_list = register_node_list.lock();
                            for i in 0..register_node_list.len() {
                                if register_node_list[i].connection.stable_id()
                                    == connection.stable_id()
                                {
                                    register_node_list.remove(i);
                                }
                            }
                        }
                        log::info!(
                            "[{}]节点断开连接，原因：{}",
                            connection.remote_address(),
                            err
                        );
                        break;
                    }
                }
            }
            eyre::Ok(())
        });
    }
    Ok(())
}
