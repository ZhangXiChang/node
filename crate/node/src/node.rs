use std::{net::SocketAddr, sync::Arc, time::Duration};

use eyre::Result;
use protocol::DataPacket;
use quinn::{ClientConfig, Connection, Endpoint, ServerConfig, TransportConfig};
use rustls::RootCertStore;
use share_code::{lock::ArcMutex, x509::x509_dns_name_from_cert_der};
use uuid::Uuid;

#[derive(Clone)]
pub struct Node {
    endpoint: Endpoint,
    cert_der: ArcMutex<Vec<u8>>,
    name: ArcMutex<String>,
    uuid: ArcMutex<String>,
    root_node_connection: ArcMutex<Option<Connection>>,
}
impl Node {
    pub fn new(name: String) -> Result<Self> {
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
            root_node_connection: ArcMutex::new(None),
        })
    }
    pub async fn register_node(
        &self,
        root_node_socket_addr: SocketAddr,
        root_node_cert_der: Vec<u8>,
    ) -> Result<()> {
        let root_node_connection = self
            .endpoint
            .connect_with(
                ClientConfig::with_root_certificates({
                    let mut a = RootCertStore::empty();
                    a.add(&rustls::Certificate(root_node_cert_der.clone()))?;
                    a
                }),
                root_node_socket_addr,
                &x509_dns_name_from_cert_der(root_node_cert_der)?,
            )?
            .await?;
        root_node_connection
            .open_uni()
            .await?
            .write_all(&rmp_serde::to_vec(&DataPacket::NodeInfo {
                name: {
                    let a = self.name.lock().clone();
                    a
                },
                uuid: {
                    let a = self.uuid.lock().clone();
                    a
                },
            })?)
            .await?;
        tokio::spawn({
            let root_node_connection = root_node_connection.clone();
            async move {
                match async {
                    loop {
                        match rmp_serde::from_slice::<DataPacket>(
                            &root_node_connection
                                .accept_uni()
                                .await?
                                .read_to_end(usize::MAX)
                                .await?,
                        )? {
                            _ => (),
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
        {
            *self.root_node_connection.lock() = Some(root_node_connection);
        }
        Ok(())
    }
}
