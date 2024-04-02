use std::{
    net::SocketAddr,
    sync::{Arc, Mutex, MutexGuard},
};

use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use x509_parser::{
    certificate::X509Certificate,
    der_parser::asn1_rs::FromDer,
    extensions::{GeneralName, ParsedExtension},
};

#[derive(Serialize, Deserialize)]
pub enum DataPacket {
    Request(RequestDataPacket),
    Response(ResponseDataPacket),
    RegisterNode { name: String, cert: Vec<u8> },
    UnRegisterNode,
}
#[derive(Serialize, Deserialize)]
pub enum RequestDataPacket {
    GetRootNodeInfo,
    GetAllRegisteredNodeName,
    GetRegisteredNodeIPAddrAndCert {
        name: String,
        node_name: String,
    },
    HolePunching {
        node_name: String,
        ip_addr: SocketAddr,
    },
}
#[derive(Serialize, Deserialize)]
pub enum ResponseDataPacket {
    GetRootNodeInfo {
        name: String,
        description: String,
    },
    GetAllRegisteredNodeName {
        all_registered_node_name: Vec<String>,
    },
    GetRegisteredNodeIPAddrAndCert(Option<NodeIPAddrAndCert>),
    HolePunching,
}
#[derive(Serialize, Deserialize)]
pub struct NodeIPAddrAndCert {
    pub ip_addr: SocketAddr,
    pub cert: Vec<u8>,
}

pub fn x509_dns_name_from_der(cert_bytes: &[u8]) -> Result<String> {
    for x509extension in X509Certificate::from_der(cert_bytes)?
        .1
        .tbs_certificate
        .extensions()
    {
        if let ParsedExtension::SubjectAlternativeName(names) = x509extension.parsed_extension() {
            for name in names.general_names.iter() {
                if let GeneralName::DNSName(dns_name) = name {
                    return Ok(dns_name.to_string());
                }
            }
        }
    }
    return Err(eyre!("读取DnsName字段失败"));
}

pub struct ArcMutex<T> {
    arc_t: Arc<Mutex<T>>,
}
impl<T> ArcMutex<T> {
    pub fn new(t: T) -> Self {
        Self {
            arc_t: Arc::new(Mutex::new(t)),
        }
    }
    pub fn lock(&self) -> MutexGuard<T> {
        self.arc_t.lock().unwrap()
    }
}
impl<T> Clone for ArcMutex<T> {
    fn clone(&self) -> Self {
        Self {
            arc_t: self.arc_t.clone(),
        }
    }
}
