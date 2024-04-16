use std::net::SocketAddr;

#[derive(Clone)]
pub struct NodeInfo {
    pub name: String,
    pub uuid: String,
    pub description: String,
    pub socket_addr: SocketAddr,
}
