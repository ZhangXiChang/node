use std::{
    fs::{create_dir_all, File},
    io::{stdin, stdout, Read, Write},
    path::PathBuf,
};

use anyhow::Result;
use clap::Parser;
use quinn::{ClientConfig, Endpoint, ServerConfig};

#[derive(Parser)]
struct CLIArgs {
    ///节点名称
    #[arg(long)]
    node_name: String,
    ///根节点证书文件路径
    #[arg(long)]
    root_node_cert_path: String,
    ///根节点IP地址
    #[arg(long)]
    root_node_ipaddr: String,
    ///根节点名称
    #[arg(long)]
    root_node_name: String,
    ///节点证书输出目录
    #[arg(long)]
    node_cert_out_dir: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    //解析命令行参数
    let cli_args = CLIArgs::parse();
    //生成证书
    let certificate_params = rcgen::CertificateParams::new(vec![cli_args.node_name.clone()]);
    let certificate = rcgen::Certificate::from_params(certificate_params)?;
    //设置节点证书输出目录
    let mut node_cert_out_dir = PathBuf::from("./");
    if let Some(cli_args_node_cert_out_dir) = cli_args.node_cert_out_dir {
        node_cert_out_dir = PathBuf::from(cli_args_node_cert_out_dir);
    }
    create_dir_all(node_cert_out_dir.clone())?;
    File::create(node_cert_out_dir.join(cli_args.node_name.clone() + ".cer"))?
        .write_all(certificate.serialize_der()?.as_slice())?;
    println!("证书生成成功");
    //创建节点
    let mut endpoint = Endpoint::server(
        ServerConfig::with_single_cert(
            vec![rustls::Certificate(certificate.serialize_der()?)],
            rustls::PrivateKey(certificate.serialize_private_key_der()),
        )?,
        "0.0.0.0:0".parse()?,
    )?;
    println!("节点创建成功");
    //加载根节点证书
    let mut cert = Vec::new();
    File::open(cli_args.root_node_cert_path)?.read_to_end(&mut cert)?;
    let mut root_cert_store = rustls::RootCertStore::empty();
    root_cert_store.add(&rustls::Certificate(cert))?;
    endpoint.set_default_client_config(ClientConfig::with_root_certificates(root_cert_store));
    println!("加载根节点证书成功");
    //连接根节点
    let connection = endpoint
        .connect(cli_args.root_node_ipaddr.parse()?, &cli_args.root_node_name)?
        .await?;
    println!("根节点连接成功");
    //从根节点获取自身外部IP地址
    let ipaddr = connection
        .accept_uni()
        .await?
        .read_to_end(usize::MAX)
        .await?;
    println!("获取到的IP地址[{}]", String::from_utf8(ipaddr)?);
    //终端交互
    loop {
        stdout().write_all(b">")?;
        stdout().flush()?;
        let mut stdin_str = String::new();
        stdin().read_line(&mut stdin_str)?;
        let stdin_str = stdin_str.trim_end();
        match stdin_str {
            "quit" => break,
            "accept" => {
                //接受连接
                println!("接受连接...");
                if let Some(connecting) = endpoint.accept().await {
                    let connection = connecting.await?;
                    println!("[{}]节点连接成功", connection.remote_address());
                    //向对象节点打招呼
                    let mut send = connection.open_uni().await?;
                    send.write_all("你好".as_bytes()).await?;
                    send.finish().await?;
                }
            }
            "connect" => {
                //接收参数
                stdout().write_all("连接对象证书路径>".as_bytes())?;
                stdout().flush()?;
                let mut stdin_str = String::new();
                stdin().read_line(&mut stdin_str)?;
                let cert_path = stdin_str.trim_end();
                stdout().write_all("连接对象IP地址>".as_bytes())?;
                stdout().flush()?;
                let mut stdin_str = String::new();
                stdin().read_line(&mut stdin_str)?;
                let ipaddr = stdin_str.trim_end();
                stdout().write_all("连接对象节点名称>".as_bytes())?;
                stdout().flush()?;
                let mut stdin_str = String::new();
                stdin().read_line(&mut stdin_str)?;
                let node_name = stdin_str.trim_end();
                //连接节点
                let mut cert = Vec::new();
                File::open(cert_path)?.read_to_end(&mut cert)?;
                let mut cert_store = rustls::RootCertStore::empty();
                cert_store.add(&rustls::Certificate(cert))?;
                let connection = endpoint
                    .connect_with(
                        ClientConfig::with_root_certificates(cert_store),
                        ipaddr.parse()?,
                        &node_name,
                    )?
                    .await?;
                println!("节点连接成功");
                //获取对象节点的打招呼
                let greet = connection
                    .accept_uni()
                    .await?
                    .read_to_end(usize::MAX)
                    .await?;
                println!("对方的回复：{}", String::from_utf8(greet)?);
            }
            _ => println!("没有[{}]这样的命令", stdin_str),
        }
    }
    endpoint.wait_idle().await;
    Ok(())
}
