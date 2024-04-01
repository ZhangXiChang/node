use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use eyre::Result;
use quinn::{ClientConfig, Connection, ServerConfig, TransportConfig};
use share::x509_dns_name_from_der;
use uuid::Uuid;

pub struct EndpointInfo {
    pub cert_dns_name: String,
    pub keep_alive_interval: Option<Duration>,
    pub socket_addr: SocketAddr,
}
impl Default for EndpointInfo {
    fn default() -> Self {
        Self {
            cert_dns_name: Uuid::new_v4().to_string(),
            keep_alive_interval: Some(Duration::from_secs(5)),
            socket_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
        }
    }
}
#[derive(Clone)]
pub struct Endpoint {
    endpoint: quinn::Endpoint,
    cert_der: Vec<u8>,
}
impl Endpoint {
    pub fn new(info: EndpointInfo) -> Result<Self> {
        //创建证书
        let cert = rcgen::Certificate::from_params(rcgen::CertificateParams::new(vec![
            info.cert_dns_name,
        ]))?;
        let cert_der = cert.serialize_der()?;
        let cert_key = cert.serialize_private_key_der();
        //创建节点
        let mut transport_config = TransportConfig::default();
        transport_config.keep_alive_interval(info.keep_alive_interval);
        let endpoint = quinn::Endpoint::server(
            ServerConfig::with_single_cert(
                vec![rustls::Certificate(cert_der.clone())],
                rustls::PrivateKey(cert_key),
            )?
            .transport_config(Arc::new(transport_config))
            .clone(),
            info.socket_addr,
        )?;
        Ok(Self { endpoint, cert_der })
    }
    pub fn cert_der(&self) -> Vec<u8> {
        self.cert_der.clone()
    }
    pub async fn accept(&self) -> Result<Option<Connection>> {
        if let Some(connecting) = self.endpoint.accept().await {
            return Ok(Some(connecting.await?));
        }
        Ok(None)
    }
    pub async fn connect(&self, socket_addr: SocketAddr, cert_der: Vec<u8>) -> Result<Connection> {
        let mut node_cert_store = rustls::RootCertStore::empty();
        node_cert_store.add(&rustls::Certificate(cert_der.clone()))?;
        Ok(self
            .endpoint
            .connect_with(
                ClientConfig::with_root_certificates(node_cert_store),
                socket_addr,
                &x509_dns_name_from_der(&cert_der)?,
            )?
            .await?)
    }
    pub async fn try_connect(&self, socket_addr: SocketAddr) -> Result<()> {
        self.endpoint.connect(socket_addr, "_")?.await?;
        Ok(())
    }
}
