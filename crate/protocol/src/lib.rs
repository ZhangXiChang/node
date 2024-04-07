use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum DataPacket {
    Request(RequestDataPacket),
    Response(ResponseDataPacket),
    RegisterNode {
        self_name: String,
        self_cert: Vec<u8>,
    },
    UnRegisterNode,
}
#[derive(Serialize, Deserialize)]
pub enum RequestDataPacket {
    GetRootNodeInfo,
    GetOnlineNodeNameList,
    GetRegisteredNodeIPAddrAndCert {
        self_name: String,
        object_name: String,
    },
    HolePunching {
        object_name: String,
        object_socket_addr: SocketAddr,
    },
}
#[derive(Serialize, Deserialize)]
pub enum ResponseDataPacket {
    GetRootNodeInfo { name: String, description: String },
    GetOnlineNodeNameList { online_node_name_list: Vec<String> },
    GetRegisteredNodeIPAddrAndCert(Option<NodeIPAddrAndCert>),
    HolePunching,
}
#[derive(Serialize, Deserialize)]
pub struct NodeIPAddrAndCert {
    pub socket_addr: SocketAddr,
    pub cert: Vec<u8>,
}
