use std::{net::SocketAddr, sync::Arc, time::Duration};

use eyre::{eyre, Result};
use quinn::{ClientConfig, Connection, Endpoint, ServerConfig, TransportConfig, VarInt};
use rustls::{
    pki_types::{CertificateDer, PrivatePkcs8KeyDer},
    RootCertStore,
};
use serde::{Deserialize, Serialize};
use tool_code_rs::{lock::ArcMutex, x509::x509_dns_name_from_cert_der};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub enum DataPacket {
    NodeInfo(NodeInfo),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NodeInfo {
    pub uuid: String,
    pub name: String,
    pub description: String,
}

#[derive(Clone)]
pub struct PeerNode {
    info: ArcMutex<NodeInfo>,
    connection: Connection,
}
impl PeerNode {
    async fn new(connection: Connection, self_node_info: NodeInfo) -> Result<Self> {
        let (mut send, mut recv) = connection.open_bi().await?;
        send.write_all(&rmp_serde::to_vec(&DataPacket::NodeInfo(self_node_info))?)
            .await?;
        send.finish()?;
        Ok(Self {
            info: ArcMutex::new(rmp_serde::from_slice(&recv.read_to_end(usize::MAX).await?)?),
            connection,
        })
    }
}

#[derive(Clone)]
pub struct Node {
    info: ArcMutex<NodeInfo>,
    cert_der: ArcMutex<Vec<u8>>,
    endpoint: Endpoint,
}
impl Node {
    pub fn new() -> Result<Self> {
        let rcgen::CertifiedKey { cert, key_pair } =
            rcgen::generate_simple_self_signed(vec![Uuid::new_v4().to_string()])?;
        Ok(Self {
            info: ArcMutex::new(NodeInfo {
                uuid: Uuid::new_v4().to_string(),
                name: String::new(),
                description: String::new(),
            }),
            cert_der: ArcMutex::new(cert.der().to_vec()),
            endpoint: Endpoint::server(
                ServerConfig::with_single_cert(
                    vec![CertificateDer::from(cert)],
                    PrivatePkcs8KeyDer::from(key_pair.serialize_der()).into(),
                )?
                .transport_config(Arc::new({
                    let mut a = TransportConfig::default();
                    a.keep_alive_interval(Some(Duration::from_secs(5)));
                    a
                }))
                .clone(),
                "0.0.0.0:0".parse()?,
            )?,
        })
    }
    pub fn close(&self, code: u32, reason: &[u8]) {
        self.endpoint.close(VarInt::from_u32(code), reason);
    }
    pub async fn accept_peer_node(&self) -> Result<PeerNode> {
        if let Some(incoming) = self.endpoint.accept().await {
            Ok(PeerNode::new(incoming.accept()?.await?, {
                let a = self.info.lock().clone();
                a
            })
            .await?)
        } else {
            Err(eyre!("节点关闭"))
        }
    }
    pub async fn connect_peer_node(
        &self,
        socket_addr: SocketAddr,
        cert_der: Vec<u8>,
    ) -> Result<PeerNode> {
        Ok(PeerNode::new(
            {
                let server_name = x509_dns_name_from_cert_der(&cert_der)?;
                self.endpoint
                    .connect_with(
                        ClientConfig::with_root_certificates({
                            Arc::new({
                                let mut a = RootCertStore::empty();
                                a.add(CertificateDer::from(cert_der))?;
                                a
                            })
                        })?,
                        socket_addr,
                        &server_name,
                    )?
                    .await?
            },
            {
                let a = self.info.lock().clone();
                a
            },
        )
        .await?)
    }
    // pub async fn connect_hub_node(&self, socket_addr: SocketAddr, cert_der: Vec<u8>) -> Result<()> {
    //     let hub_node_connection = self
    //         .endpoint
    //         .connect_with(
    //             ClientConfig::with_root_certificates({
    //                 Arc::new({
    //                     let mut a = RootCertStore::empty();
    //                     a.add(CertificateDer::from(cert_der.clone()))?;
    //                     a
    //                 })
    //             })?,
    //             socket_addr,
    //             &x509_dns_name_from_cert_der(cert_der)?,
    //         )?
    //         .await?;
    //     let mut send = hub_node_connection.open_uni().await?;
    //     send.write_all(&rmp_serde::to_vec(&DataPacket::NodeInfo {
    //         uuid: {
    //             let a = self.info.lock().uuid.clone();
    //             a
    //         },
    //         name: {
    //             let a = self.info.lock().name.clone();
    //             a
    //         },
    //         description: {
    //             let a = self.info.lock().description.clone();
    //             a
    //         },
    //     })?)
    //     .await?;
    //     send.finish()?;
    //     tokio::spawn(async move {
    //         loop {
    //             match hub_node_connection.accept_uni().await {
    //                 Ok(mut read) => {
    //                     match rmp_serde::from_slice::<DataPacket>(&match read
    //                         .read_to_end(usize::MAX)
    //                         .await
    //                     {
    //                         Ok(data) => data,
    //                         Err(err) => {
    //                             log::error!(
    //                                 "[{}]读取数据报失败，原因：{}",
    //                                 hub_node_connection.remote_address(),
    //                                 err
    //                             );
    //                             break;
    //                         }
    //                     }) {
    //                         Ok(data_packet) => match data_packet {
    //                             _ => (),
    //                         },
    //                         Err(err) => {
    //                             log::error!(
    //                                 "[{}]解析数据报失败，原因：{}",
    //                                 hub_node_connection.remote_address(),
    //                                 err
    //                             );
    //                             break;
    //                         }
    //                     }
    //                 }
    //                 Err(err) => {
    //                     log::info!(
    //                         "[{}]连接关闭，原因：{}",
    //                         hub_node_connection.remote_address(),
    //                         err
    //                     );
    //                     break;
    //                 }
    //             }
    //         }
    //     });
    //     Ok(())
    // }
}
