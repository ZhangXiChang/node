use std::{
    fs::{create_dir_all, File},
    io::Write,
    path::PathBuf,
};

use clap::Parser;
use eyre::Result;

#[derive(Parser)]
struct CLIArgs {
    ///DNS名称
    #[arg(long)]
    dns_name: String,
    ///输出目录，默认"./"
    #[arg(long)]
    out_dir: Option<String>,
}

fn main() -> Result<()> {
    //解析命令行参数
    let cli_args = CLIArgs::parse();
    //配置数字证书
    let certificate_params = rcgen::CertificateParams::new(vec![cli_args.dns_name.clone()]);
    let certificate = rcgen::Certificate::from_params(certificate_params)?;
    //设置输出目录
    let mut out_dir = PathBuf::from("./");
    if let Some(cli_args_out_dir) = cli_args.out_dir {
        out_dir = PathBuf::from(cli_args_out_dir);
    }
    //输出到文件
    create_dir_all(out_dir.clone())?;
    File::create(out_dir.join(cli_args.dns_name.clone() + ".cer"))?
        .write_all(certificate.serialize_der()?.as_slice())?;
    File::create(out_dir.join(cli_args.dns_name.clone() + ".key"))?
        .write_all(certificate.serialize_private_key_der().as_slice())?;
    Ok(())
}
