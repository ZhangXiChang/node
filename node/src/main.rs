use std::{
    fs::{create_dir_all, File},
    io::{stdin, Read, Write},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use clap::Parser;
use quinn::{ClientConfig, Endpoint, ServerConfig, TransportConfig};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
enum DataPacket {
    RequestRegisterNode {
        node_name: String,
        node_cert: Vec<u8>,
    },
    RequestGetOnlineNode,
    Test,
}

#[derive(Parser, Clone)]
struct CLIArgs {
    ///节点名称
    #[arg(long)]
    node_name: String,
    ///根节点IP地址
    #[arg(long)]
    root_node_ipaddr: String,
    ///根节点名称
    #[arg(long)]
    root_node_name: String,
    ///证书文件目录路径，默认"./"。会根据设置的节点和根节点名称配置证书文件名称
    #[arg(long)]
    cert_file_dir_path: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    //解析命令行参数
    let cli_args = CLIArgs::parse();
    //设置默认证书输出目录
    let mut cert_file_dir_path = PathBuf::from("./");
    //生成证书
    let certificate_params = rcgen::CertificateParams::new(vec![cli_args.node_name.clone()]);
    let certificate = rcgen::Certificate::from_params(certificate_params)?;
    //确认证书输出目录
    if let Some(cli_args_cert_file_dir_path) = cli_args.cert_file_dir_path {
        cert_file_dir_path = PathBuf::from(cli_args_cert_file_dir_path);
    }
    //输出节点证书
    create_dir_all(cert_file_dir_path.clone())?;
    File::create(cert_file_dir_path.join(cli_args.node_name + ".cer"))?
        .write_all(certificate.serialize_der()?.as_slice())?;
    println!("证书生成成功");
    //创建节点
    let mut transport_config = TransportConfig::default();
    transport_config.keep_alive_interval(Some(Duration::from_secs(5)));
    let mut endpoint = Endpoint::server(
        ServerConfig::with_single_cert(
            vec![rustls::Certificate(certificate.serialize_der()?)],
            rustls::PrivateKey(certificate.serialize_private_key_der()),
        )?
        .transport_config(Arc::new(transport_config))
        .clone(),
        "0.0.0.0:0".parse()?,
    )?;
    println!("节点创建成功");
    //加载根节点证书设置为默认信任证书
    let mut root_node_cert = Vec::new();
    File::open(cert_file_dir_path.join(cli_args.root_node_name.clone() + ".cer"))?
        .read_to_end(&mut root_node_cert)?;
    let mut root_node_cert_store = rustls::RootCertStore::empty();
    root_node_cert_store.add(&rustls::Certificate(root_node_cert))?;
    endpoint.set_default_client_config(ClientConfig::with_root_certificates(root_node_cert_store));
    println!("加载根节点证书设置为默认信任证书成功");
    //连接根节点
    let root_node_connection = endpoint
        .connect(cli_args.root_node_ipaddr.parse()?, &cli_args.root_node_name)?
        .await?;
    println!("根节点连接成功");
    //终端交互
    println!("[quit]退出程序");
    println!("[accept]接收连接");
    println!("[connect]连接");
    loop {
        println!("[输入命令]");
        let mut stdin_str = String::new();
        stdin().read_line(&mut stdin_str)?;
        let stdin_str = stdin_str.trim_end();
        match stdin_str {
            "quit" => break,
            "accept" => {
                //接收连接
                println!("等待连接中...");
                if let Some(connecting) = endpoint.accept().await {
                    let connection = connecting.await?;
                    println!("[{}]节点连接成功", connection.remote_address());
                }
            }
            "connect" => {
                //接收参数
                println!("[对象IP地址]");
                let mut stdin_str = String::new();
                stdin().read_line(&mut stdin_str)?;
                let node_addr = stdin_str.trim_end();
                println!("[对象节点名称]");
                let mut stdin_str = String::new();
                stdin().read_line(&mut stdin_str)?;
                let node_name = stdin_str.trim_end();
                //连接节点
                let mut node_cert = Vec::new();
                File::open(cert_file_dir_path.join(node_name.to_string() + ".cer"))?
                    .read_to_end(&mut node_cert)?;
                let mut node_cert_store = rustls::RootCertStore::empty();
                node_cert_store.add(&rustls::Certificate(node_cert))?;
                match endpoint
                    .connect_with(
                        ClientConfig::with_root_certificates(node_cert_store),
                        node_addr.parse()?,
                        &node_name,
                    )?
                    .await
                {
                    Ok(connection) => {
                        println!("节点连接成功");
                    }
                    Err(err) => {
                        println!("节点连接失败，原因：{}", err);
                    }
                };
            }
            _ => println!("没有[{}]这样的命令", stdin_str),
        }
    }
    Ok(())
}
