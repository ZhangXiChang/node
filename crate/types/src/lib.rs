use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum DataPacket {
    Request(RequestDataPacket),
    Response(ResponseDataPacket),
    RegisterNode {
        node_name: String,
        node_cert: Vec<u8>,
    },
}
#[derive(Serialize, Deserialize)]
pub enum RequestDataPacket {
    GetRegisteredNodeNameList,
    GetRegisteredNodeAddrAndCert { node_name: String },
    HolePunching { addr: SocketAddr },
}
#[derive(Serialize, Deserialize)]
pub enum ResponseDataPacket {
    GetRegisteredNodeNameList {
        registered_node_name_list: Vec<String>,
    },
    GetRegisteredNodeAddrAndCert(Result<NodeAddrAndCert, String>),
    HolePunching,
}
#[derive(Serialize, Deserialize)]
pub struct NodeAddrAndCert {
    pub addr: SocketAddr,
    pub cert: Vec<u8>,
}
