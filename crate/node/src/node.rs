use std::{
    fs::{create_dir_all, File},
    io::Read,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use eyre::{eyre, Result};
use quinn::{
    ClientConfig, Connection, ConnectionError, Endpoint, ServerConfig, TransportConfig, VarInt,
};
use rustls::RootCertStore;
use share_code::lock::ArcMutex;
use uuid::Uuid;

#[derive(Clone)]
pub struct Node {
    endpoint: Endpoint,
    root_node_connection: ArcMutex<Option<Connection>>,
}
impl Node {
    pub fn new() -> Result<Self> {
        Ok(Self {
            endpoint: Endpoint::server(
                {
                    let rcgen::CertifiedKey { cert, key_pair } =
                        rcgen::generate_simple_self_signed(vec![Uuid::new_v4().to_string()])?;
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
        })
    }
    pub fn close(&self, error_code: u32, reason: Vec<u8>) {
        self.endpoint.close(VarInt::from_u32(error_code), &reason);
    }
    pub async fn connect_root_node(
        &mut self,
        socket_addr: SocketAddr,
        dns_name: String,
    ) -> Result<()> {
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
            return Ok(root_node_connection.close_reason());
        }
        Err(eyre!("根节点连接不存在"))
    }
    pub async fn wait_root_node_disconnect(&self) -> Result<ConnectionError> {
        if let Some(root_node_connection) = {
            let a = self.root_node_connection.lock().clone();
            a
        } {
            return Ok(root_node_connection.closed().await);
        }
        Err(eyre!("根节点连接不存在"))
    }
}
