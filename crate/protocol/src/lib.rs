use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct NodeInfo {
    pub user_name: String,
    pub uuid: String,
    pub description: String,
}

#[derive(Serialize, Deserialize)]
pub struct NodeRegisterInfo {
    pub node_info: NodeInfo,
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
}
#[derive(Serialize, Deserialize)]
pub enum Response {
    RegisterNodeInfoList(Vec<NodeInfo>),
}
