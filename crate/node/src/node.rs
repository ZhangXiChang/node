use std::{
    fs::{create_dir_all, File},
    io::Read,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use eyre::{eyre, Result};
use protocol::{DataPacket, NodeInfo, NodeRegisterInfo, Request, Response};
use quinn::{
    ClientConfig, Connection, ConnectionError, Endpoint, ServerConfig, TransportConfig, VarInt,
};
use rustls::RootCertStore;
use share_code::{lock::ArcMutex, x509::x509_dns_name_from_der};
use uuid::Uuid;

#[derive(Clone)]
pub struct Node {
    pub user_name: String,
    pub readme: String,
    uuid: String,
    cert_der: Vec<u8>,
    endpoint: Endpoint,
    root_node_connection: ArcMutex<Option<Connection>>,
    node_connection: ArcMutex<Option<Connection>>,
}
impl Node {
    pub fn new(user_name: String, readme: String) -> Result<Self> {
        let rcgen::CertifiedKey { cert, key_pair } =
            rcgen::generate_simple_self_signed(vec![Uuid::new_v4().to_string()])?;
        Ok(Self {
            user_name,
            readme,
            uuid: Uuid::new_v4().to_string(),
            cert_der: cert.der().to_vec(),
            endpoint: Endpoint::server(
                {
                    ServerConfig::with_single_cert(
                        vec![rustls::Certificate(cert.der().to_vec())],
                        rustls::PrivateKey(key_pair.serialize_der()),
                    )?
                    .transport_config(Arc::new({
                        let mut a = TransportConfig::default();
                        a.keep_alive_interval(Some(Duration::from_secs(5)));
                        a
                    }))
                    .to_owned()
                },
                "0.0.0.0:0".parse()?,
            )?,
            root_node_connection: ArcMutex::new(None),
            node_connection: ArcMutex::new(None),
        })
    }
    pub fn close(&self, error_code: u32, reason: Vec<u8>) {
        self.endpoint.close(VarInt::from_u32(error_code), &reason);
    }
    pub async fn connect_root_node(&self, socket_addr: SocketAddr, dns_name: String) -> Result<()> {
        *self.root_node_connection.lock() = Some(
            self.endpoint
                .connect_with(
                    ClientConfig::with_root_certificates({
                        let mut a = RootCertStore::empty();
                        let cert_dir_path = PathBuf::from("./certs/");
                        create_dir_all(cert_dir_path.clone())?;
                        for dir_entry in cert_dir_path.read_dir()? {
                            if let Ok(dir_entry) = dir_entry {
                                let path = dir_entry.path();
                                if let Some(extension) = path.extension() {
                                    if extension == "cer" {
                                        let mut cert_der = Vec::new();
                                        File::open(path)?.read_to_end(&mut cert_der)?;
                                        a.add(&rustls::Certificate(cert_der))?;
                                    }
                                }
                            }
                        }
                        a
                    }),
                    socket_addr,
                    &dns_name,
                )?
                .await?,
        );
        Ok(())
    }
    pub fn disconnect_root_node(&self, error_code: u32, reason: Vec<u8>) {
        if let Some(root_node_connection) = {
            let a = self.root_node_connection.lock().clone();
            a
        } {
            root_node_connection.close(VarInt::from_u32(error_code), &reason);
        }
    }
    pub fn root_node_is_disconnect(&self) -> Result<Option<ConnectionError>> {
        if let Some(root_node_connection) = {
            let a = self.root_node_connection.lock().clone();
            a
        } {
            Ok(root_node_connection.close_reason())
        } else {
            Err(eyre!("根节点连接不存在"))
        }
    }
    pub async fn wait_root_node_disconnect(&self) -> Result<ConnectionError> {
        if let Some(root_node_connection) = {
            let a = self.root_node_connection.lock().clone();
            a
        } {
            Ok(root_node_connection.closed().await)
        } else {
            Err(eyre!("根节点连接不存在"))
        }
    }
    pub async fn register_node(&self) -> Result<()> {
        if let Some(root_node_connection) = {
            let a = self.root_node_connection.lock().clone();
            a
        } {
            let (mut send, _) = root_node_connection.open_bi().await?;
            send.write_all(&rmp_serde::to_vec(&DataPacket::Request(
                Request::RegisterNode(NodeRegisterInfo {
                    node_info: NodeInfo {
                        user_name: self.user_name.clone(),
                        uuid: self.uuid.clone(),
                        readme: self.readme.clone(),
                    },
                    cert_der: self.cert_der.clone(),
                }),
            ))?)
            .await?;
            send.finish().await?;
        }
        Ok(())
    }
    pub async fn unregister_node(&self) -> Result<()> {
        if let Some(root_node_connection) = {
            let a = self.root_node_connection.lock().clone();
            a
        } {
            let (mut send, _) = root_node_connection.open_bi().await?;
            send.write_all(&rmp_serde::to_vec(&DataPacket::Request(
                Request::UnregisterNode,
            ))?)
            .await?;
            send.finish().await?;
        }
        Ok(())
    }
    pub async fn request_register_node_info_list(&self) -> Result<Vec<NodeInfo>> {
        if let Some(root_node_connection) = {
            let a = self.root_node_connection.lock().clone();
            a
        } {
            let (mut send, mut recv) = root_node_connection.open_bi().await?;
            send.write_all(&rmp_serde::to_vec(&DataPacket::Request(
                Request::RegisterNodeInfoList,
            ))?)
            .await?;
            send.finish().await?;
            match rmp_serde::from_slice::<DataPacket>(&recv.read_to_end(usize::MAX).await?)? {
                DataPacket::Response(Response::RegisterNodeInfoList(node_info_list)) => {
                    Ok(node_info_list)
                }
                _ => Err(eyre!("服务器返回了意料之外的数据包")),
            }
        } else {
            Err(eyre!("根节点连接不存在"))
        }
    }
    pub async fn accept(&self) -> Result<()> {
        if let Some(connecting) = self.endpoint.accept().await {
            *self.node_connection.lock() = Some(connecting.await?);
        }
        Ok(())
    }
    pub async fn connect_node(&self, uuid: String) -> Result<()> {
        if let Some(root_node_connection) = {
            let a = self.root_node_connection.lock().clone();
            a
        } {
            let (mut send, mut recv) = root_node_connection.open_bi().await?;
            send.write_all(&rmp_serde::to_vec(&DataPacket::Request(
                Request::ConnectNode(uuid),
            ))?)
            .await?;
            send.finish().await?;
            match rmp_serde::from_slice::<DataPacket>(&recv.read_to_end(usize::MAX).await?)? {
                DataPacket::Response(Response::ConnectNode(connect_node_info)) => {
                    if let Some(connect_node_info) = connect_node_info {
                        *self.node_connection.lock() = Some(
                            self.endpoint
                                .connect_with(
                                    ClientConfig::with_root_certificates({
                                        let mut a = RootCertStore::empty();
                                        a.add(&rustls::Certificate(
                                            connect_node_info.cert_der.clone(),
                                        ))?;
                                        a
                                    }),
                                    connect_node_info.socket_addr,
                                    &x509_dns_name_from_der(connect_node_info.cert_der)?,
                                )?
                                .await?,
                        );
                        Ok(())
                    } else {
                        Err(eyre!("没有找到节点"))
                    }
                }
                _ => Err(eyre!("服务器返回了意料之外的数据包")),
            }
        } else {
            Err(eyre!("根节点连接不存在"))
        }
    }
}
