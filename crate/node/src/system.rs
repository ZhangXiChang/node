use std::{fs::File, io::Read, net::SocketAddr, path::PathBuf};

use eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RootNodeInfo {
    pub name: String,
    pub dns_name: String,
    pub socket_addr: SocketAddr,
}
#[derive(Serialize, Deserialize)]
struct Config {
    user_name: String,
    root_node_info_list: Vec<RootNodeInfo>,
}

pub struct System {
    user_name: String,
    root_node_info_list: Vec<RootNodeInfo>,
}
impl System {
    pub fn new() -> Result<Self> {
        let config = Self::load_config()?;
        Ok(Self {
            user_name: config.user_name,
            root_node_info_list: config.root_node_info_list,
        })
    }
    fn load_config() -> Result<Config> {
        //初始配置
        let mut config = Config {
            user_name: "".to_string(),
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
    pub fn write_user_name_to_config(&self) -> Result<()> {
        let config_file_path = PathBuf::from("./config.json");
        let mut config_bytes = Vec::new();
        File::open(config_file_path.clone())?.read_to_end(&mut config_bytes)?;
        let mut config = serde_json::from_slice::<Config>(&config_bytes)?;
        config.user_name = self.user_name.clone();
        config.serialize(&mut serde_json::Serializer::with_formatter(
            File::create(config_file_path)?,
            serde_json::ser::PrettyFormatter::with_indent(b"    "),
        ))?;
        Ok(())
    }
    pub fn user_name<'a>(&'a self) -> &'a String {
        &self.user_name
    }
    pub fn user_name_mut<'a>(&'a mut self) -> &'a mut String {
        &mut self.user_name
    }
    pub fn root_node_info_list<'a>(&'a self) -> &'a Vec<RootNodeInfo> {
        &self.root_node_info_list
    }
}
