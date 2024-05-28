use std::{fs::File, io::Read};

use eyre::Result;
use node_network::Node;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
    let node = Node::new(
        "0.0.0.0:10270".parse()?,
        {
            let mut cert_der = Vec::new();
            File::open("./certs/root_node.cer")?.read_to_end(&mut cert_der)?;
            cert_der
        },
        {
            let mut key_pair = Vec::new();
            File::open("./certs/root_node.key")?.read_to_end(&mut key_pair)?;
            key_pair
        },
    )?;
    node.accept_peer_node_as_hub_node().await?;
    Ok(())
}
