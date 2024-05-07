use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct NodeInfo {
    pub name: String,
    pub uuid: String,
    pub description: String,
}

#[derive(Serialize, Deserialize)]
pub enum DataPacket {
    NodeInfo(NodeInfo),
}
