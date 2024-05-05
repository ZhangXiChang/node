use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub enum DataPacket {
    NodeInfo { name: String, uuid: String },
}
