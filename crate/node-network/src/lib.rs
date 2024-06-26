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
        let mut send = connection.open_uni().await?;
        send.write_all(&rmp_serde::to_vec(&self_node_info)?).await?;
        send.finish()?;
        Ok(Self {
            info: ArcMutex::new(rmp_serde::from_slice(
                &connection
                    .accept_uni()
                    .await?
                    .read_to_end(usize::MAX)
                    .await?,
            )?),
            connection,
        })
    }
    pub fn remote_address(&self) -> SocketAddr {
        self.connection.remote_address()
    }
    pub fn close(&self, code: u32, reason: &[u8]) {
        self.connection.close(VarInt::from_u32(code), reason);
    }
}

#[derive(Clone)]
pub struct Node {
    info: ArcMutex<NodeInfo>,
    cert_der: Arc<Vec<u8>>,
    endpoint: Endpoint,
    hub_node: ArcMutex<Option<PeerNode>>,
}
impl Node {
    pub fn new(socket_addr: SocketAddr, cert_der: Vec<u8>, key_pair_der: Vec<u8>) -> Result<Self> {
        rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .map_err(|_| eyre!("初始化CryptoProvider失败"))?;
        Ok(Self {
            info: ArcMutex::new(NodeInfo {
                uuid: Uuid::new_v4().to_string(),
                name: String::new(),
                description: String::new(),
            }),
            cert_der: Arc::new(cert_der.clone()),
            endpoint: Endpoint::server(
                ServerConfig::with_single_cert(
                    vec![CertificateDer::from(cert_der)],
                    PrivatePkcs8KeyDer::from(key_pair_der).into(),
                )?
                .transport_config(Arc::new({
                    let mut a = TransportConfig::default();
                    a.keep_alive_interval(Some(Duration::from_secs(5)));
                    a
                }))
                .clone(),
                socket_addr,
            )?,
            hub_node: ArcMutex::new(None),
        })
    }
    pub fn new_from_new_cert(socket_addr: SocketAddr) -> Result<Self> {
        let rcgen::CertifiedKey { cert, key_pair } =
            rcgen::generate_simple_self_signed(vec![Uuid::new_v4().to_string()])?;
        Self::new(socket_addr, cert.der().to_vec(), key_pair.serialize_der())
    }
    pub fn set_name(&self, name: String) {
        self.info.lock().name = name;
    }
    pub fn set_description(&self, description: String) {
        self.info.lock().description = description;
    }
    pub fn close(&self, code: u32, reason: &[u8]) {
        self.endpoint.close(VarInt::from_u32(code), reason);
    }
    pub fn close_hub_node(&self, code: u32, reason: &[u8]) {
        if let Some(peer_node) = {
            let a = self.hub_node.lock().clone();
            a
        } {
            peer_node.close(code, reason);
        }
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
    pub async fn accept_peer_node_as_hub_node(&self) -> Result<()> {
        loop {
            let peer_node = self.accept_peer_node().await?;
            tokio::spawn(async move {
                log::info!("[{}]连接成功", peer_node.remote_address());
            });
        }
    }
    pub async fn access_hub_node(&self, socket_addr: SocketAddr, cert_der: Vec<u8>) -> Result<()> {
        *self.hub_node.lock() = Some(
            PeerNode::new(
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
            .await?,
        );
        Ok(())
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
