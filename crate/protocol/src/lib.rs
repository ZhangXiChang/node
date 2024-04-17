use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeInfo {
    pub user_name: String,
    pub uuid: String,
    pub readme: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct NodeRegisterInfo {
    pub node_info: NodeInfo,
    pub cert_der: Vec<u8>,
}
#[derive(Serialize, Deserialize)]
pub struct ConnectNodeInfo {
    pub socket_addr: SocketAddr,
    pub cert_der: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub enum DataPacket {
    Request(Request),
    Response(Response),
}
#[derive(Serialize, Deserialize)]
pub enum Request {
    RegisterNode(NodeRegisterInfo),
    UnregisterNode,
    RegisterNodeInfoList,
    ConnectNode(String),
}
#[derive(Serialize, Deserialize)]
pub enum Response {
    RegisterNodeInfoList(Vec<NodeInfo>),
    ConnectNode(Option<ConnectNodeInfo>),
}
