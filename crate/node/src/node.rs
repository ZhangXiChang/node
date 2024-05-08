use std::{net::SocketAddr, sync::Arc, time::Duration};

use eyre::{eyre, Result};
use protocol::{DataPacket, NodeInfo};
use quinn::{
    ClientConfig, Connection, ConnectionError, Endpoint, ServerConfig, TransportConfig, VarInt,
};
use rustls::RootCertStore;
use share_code::{lock::ArcMutex, x509::x509_dns_name_from_cert_der};
use uuid::Uuid;

#[derive(Default)]
pub struct NewNodeInfo {
    pub description: String,
}
#[derive(Clone)]
pub struct Node {
    endpoint: Endpoint,
    cert_der: ArcMutex<Vec<u8>>,
    name: ArcMutex<String>,
    uuid: ArcMutex<String>,
    description: ArcMutex<String>,
    root_node_connection: ArcMutex<Option<Connection>>,
}
impl Node {
    pub fn new(name: String, info: NewNodeInfo) -> Result<Self> {
        let rcgen::CertifiedKey { cert, key_pair } =
            rcgen::generate_simple_self_signed(vec![Uuid::new_v4().to_string()])?;
        Ok(Self {
            endpoint: Endpoint::server(
                ServerConfig::with_single_cert(
                    vec![rustls::Certificate(cert.der().to_vec())],
                    rustls::PrivateKey(key_pair.serialize_der()),
                )?
                .transport_config(Arc::new({
                    let mut a = TransportConfig::default();
                    a.keep_alive_interval(Some(Duration::from_secs(5)));
                    a
                }))
                .to_owned(),
                "0.0.0.0:0".parse()?,
            )?,
            cert_der: ArcMutex::new(cert.der().to_vec()),
            name: ArcMutex::new(name),
            uuid: ArcMutex::new(Uuid::new_v4().to_string()),
            description: ArcMutex::new(info.description),
            root_node_connection: ArcMutex::new(None),
        })
    }
    pub fn close(&self, code: u32, reason: Vec<u8>) {
        self.endpoint.close(VarInt::from_u32(code), &reason);
    }
    pub async fn connect_root_node(
        &self,
        root_node_socket_addr: SocketAddr,
        root_node_cert_der: Vec<u8>,
    ) -> Result<()> {
        *self.root_node_connection.lock() = Some(
            self.endpoint
                .connect_with(
                    ClientConfig::with_root_certificates({
                        let mut a = RootCertStore::empty();
                        a.add(&rustls::Certificate(root_node_cert_der.clone()))?;
                        a
                    }),
                    root_node_socket_addr,
                    &x509_dns_name_from_cert_der(root_node_cert_der)?,
                )?
                .await?,
        );
        if let Some(root_node_connection) = {
            let a = self.root_node_connection.lock().clone();
            a
        } {
            root_node_connection
                .open_uni()
                .await?
                .write_all(&rmp_serde::to_vec(&DataPacket::NodeInfo(NodeInfo {
                    name: {
                        let a = self.name.lock().clone();
                        a
                    },
                    uuid: {
                        let a = self.uuid.lock().clone();
                        a
                    },
                    description: {
                        let a = self.description.lock().clone();
                        a
                    },
                }))?)
                .await?;
            tokio::spawn({
                async move {
                    match async {
                        loop {
                            match root_node_connection.accept_uni().await {
                                Ok(mut read) => match rmp_serde::from_slice::<DataPacket>(
                                    &read.read_to_end(usize::MAX).await?,
                                )? {
                                    _ => (),
                                },
                                Err(err) => {
                                    log::info!(
                                        "[{}]连接关闭，原因：{}",
                                        root_node_connection.remote_address(),
                                        err
                                    );
                                    break;
                                }
                            }
                        }
                        #[allow(unreachable_code)]
                        eyre::Ok(())
                    }
                    .await
                    {
                        Ok(_) => (),
                        Err(err) => log::error!("{}", err),
                    }
                }
            });
        }
        Ok(())
    }
    pub fn close_root_node_connect(&self, code: u32, reason: Vec<u8>) {
        if let Some(root_node_connection) = {
            let a = self.root_node_connection.lock().clone();
            a
        } {
            root_node_connection.close(VarInt::from_u32(code), &reason)
        }
    }
    pub fn root_node_state(&self) -> Result<Option<ConnectionError>> {
        match {
            let a = self.root_node_connection.lock().clone();
            a
        } {
            Some(root_node_connection) => Ok(root_node_connection.close_reason()),
            None => Err(eyre!("根节点不存在")),
        }
    }
}
