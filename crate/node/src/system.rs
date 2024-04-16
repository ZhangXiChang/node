use std::{fs::File, io::Read, net::SocketAddr, path::PathBuf};

use eyre::Result;
use serde::{Deserialize, Serialize};

use crate::node::Node;

#[derive(Serialize, Deserialize)]
pub struct RootNodeInfo {
    pub name: String,
    pub dns_name: String,
    pub socket_addr: SocketAddr,
}
#[derive(Serialize, Deserialize)]
struct Config {
    user_name: String,
    description: String,
    root_node_info_list: Vec<RootNodeInfo>,
}

pub struct System {
    pub node: Node,
    pub root_node_info_list: Vec<RootNodeInfo>,
}
impl System {
    pub fn new() -> Result<Self> {
        let config = Self::load_config()?;
        Ok(Self {
            node: Node::new(config.user_name, config.description)?,
            root_node_info_list: config.root_node_info_list,
        })
    }
    fn load_config() -> Result<Config> {
        //初始配置
        let mut config = Config {
            user_name: String::new(),
            description: String::new(),
            root_node_info_list: vec![RootNodeInfo {
                name: "默认根节点".to_string(),
                dns_name: "root_node".to_string(),
                socket_addr: "127.0.0.1:10270".parse()?,
            }],
        };
        //解析配置文件
        let config_file_path = PathBuf::from("./config.json");
        match File::open(config_file_path.clone()) {
            Ok(mut config_file) => {
                let mut config_bytes = Vec::new();
                config_file.read_to_end(&mut config_bytes)?;
                config = serde_json::from_slice(&config_bytes)?;
            }
            Err(_) => {
                config.serialize(&mut serde_json::Serializer::with_formatter(
                    File::create(config_file_path)?,
                    serde_json::ser::PrettyFormatter::with_indent(b"    "),
                ))?;
            }
        }
        Ok(config)
    }
    pub fn save_config(&self) -> Result<()> {
        let config_file_path = PathBuf::from("./config.json");
        let mut config_bytes = Vec::new();
        File::open(config_file_path.clone())?.read_to_end(&mut config_bytes)?;
        let mut config = serde_json::from_slice::<Config>(&config_bytes)?;
        config.user_name = self.node.user_name.clone();
        config.description = self.node.description.clone();
        config.serialize(&mut serde_json::Serializer::with_formatter(
            File::create(config_file_path)?,
            serde_json::ser::PrettyFormatter::with_indent(b"    "),
        ))?;
        Ok(())
    }
}
