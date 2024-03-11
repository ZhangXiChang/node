use std::{
    fs::{create_dir_all, File},
    io::{stdin, Read, Write},
    path::PathBuf,
};

use anyhow::Result;
use clap::Parser;
use quinn::{ClientConfig, Connection, Endpoint, ServerConfig, VarInt};

#[derive(Parser, Clone)]
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

struct App {
    endpoint: Endpoint,
}
impl App {
    async fn new() -> Result<Self> {
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
        File::create(node_cert_out_dir.join(cli_args.node_name + ".cer"))?
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
        //加载根节点证书设置为默认信任证书
        let mut cert = Vec::new();
        File::open(cli_args.root_node_cert_path)?.read_to_end(&mut cert)?;
        let mut root_cert_store = rustls::RootCertStore::empty();
        root_cert_store.add(&rustls::Certificate(cert))?;
        endpoint.set_default_client_config(ClientConfig::with_root_certificates(root_cert_store));
        println!("加载根节点证书设置为默认信任证书成功");
        //连接根节点
        let connection = endpoint
            .connect(cli_args.root_node_ipaddr.parse()?, &cli_args.root_node_name)?
            .await?;
        println!("根节点连接成功");
        //从根节点获取自身外部IP地址
        let ipaddr = String::from_utf8(
            connection
                .accept_uni()
                .await?
                .read_to_end(usize::MAX)
                .await?,
        )?;
        println!("获取到的IP地址[{}]", ipaddr);
        Ok(Self { endpoint })
    }
    async fn run(self) -> Result<()> {
        //终端交互
        println!("[quit]退出程序");
        println!("[accept]接收连接");
        println!("[connect]连接");
        println!("[输入命令]");
        loop {
            let mut stdin_str = String::new();
            stdin().read_line(&mut stdin_str)?;
            match stdin_str.trim_end() {
                "quit" => break,
                "accept" => {
                    //接收连接
                    println!("等待连接");
                    if let Some(connecting) = self.endpoint.accept().await {
                        let connection = connecting.await?;
                        println!("[{}]节点连接成功", connection.remote_address());
                        Self::chat(connection).await?;
                    }
                }
                "connect" => {
                    //接收参数
                    println!("[对象证书路径]");
                    let mut stdin_str = String::new();
                    stdin().read_line(&mut stdin_str)?;
                    let cert_path = stdin_str.trim_end();
                    println!("[对象IP地址]");
                    let mut stdin_str = String::new();
                    stdin().read_line(&mut stdin_str)?;
                    let ipaddr = stdin_str.trim_end();
                    println!("[对象节点名称]");
                    let mut stdin_str = String::new();
                    stdin().read_line(&mut stdin_str)?;
                    let node_name = stdin_str.trim_end();
                    //连接节点
                    let mut cert = Vec::new();
                    File::open(cert_path)?.read_to_end(&mut cert)?;
                    let mut cert_store = rustls::RootCertStore::empty();
                    cert_store.add(&rustls::Certificate(cert))?;
                    let connection = self
                        .endpoint
                        .connect_with(
                            ClientConfig::with_root_certificates(cert_store),
                            ipaddr.parse()?,
                            &node_name,
                        )?
                        .await?;
                    println!("节点连接成功");
                    Self::chat(connection).await?;
                }
                _ => println!("没有[{}]这样的命令", stdin_str),
            }
        }
        self.endpoint.wait_idle().await;
        Ok(())
    }
    async fn chat(connection: Connection) -> Result<()> {
        //接收消息
        tokio::spawn({
            let connection = connection.clone();
            async move {
                loop {
                    match connection.accept_uni().await {
                        Ok(mut recv) => println!(
                            "[{}]：{}",
                            connection.remote_address(),
                            String::from_utf8(recv.read_to_end(usize::MAX).await?)?
                        ),
                        Err(err) => {
                            println!(
                                "[{}]节点断开连接，原因：{}",
                                connection.remote_address(),
                                err
                            );
                            break;
                        }
                    }
                }
                anyhow::Ok(())
            }
        });
        //发送消息
        println!("[/close]关闭连接");
        println!("[开始聊天吧]");
        loop {
            let mut stdin_str = String::new();
            stdin().read_line(&mut stdin_str)?;
            let stdin_str = stdin_str.trim_end();
            if stdin_str == "/close" {
                connection.close(VarInt::from_u32(0), "正常关闭连接".as_bytes());
                break;
            }
            let mut send = connection.open_uni().await?;
            send.write_all(stdin_str.as_bytes()).await?;
            send.finish().await?;
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    App::new().await?.run().await?;
    Ok(())
}
