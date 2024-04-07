use std::{
    borrow::Cow,
    fs::{create_dir_all, File},
    io::Read,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use eyre::{eyre, Result};

use eframe::egui;
use quinn::{ClientConfig, Connection, Endpoint, ServerConfig, TransportConfig};
use rustls::RootCertStore;
use serde::{Deserialize, Serialize};
use share::ArcMutex;
use uuid::Uuid;

const ICON: &[u8] = include_bytes!("../../../assets/icon/node_network_icon.png");
const ICON_WIDTH: u32 = 512;
const ICON_HEIGHT: u32 = 512;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RootNode {
    node_name: String,
    socket_addr: SocketAddr,
}
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Config {
    node_name: String,
    root_node: RootNode,
}

#[derive(Clone)]
enum ConnectionState {
    Connected,
    Disconnect,
    Connecting,
}

#[derive(Clone)]
enum Message {
    Info(String),
    Error(String),
}

#[derive(Clone)]
struct UIModeSwitchButtonText(String);
impl From<UIModeSwitchButtonText> for String {
    fn from(value: UIModeSwitchButtonText) -> Self {
        value.0
    }
}

#[derive(Clone)]
enum UIMode {
    Unfold,
    Fold,
}
impl From<UIMode> for UIModeSwitchButtonText {
    fn from(value: UIMode) -> Self {
        match value {
            UIMode::Unfold => Self("🗕 折叠程序".to_owned()),
            UIMode::Fold => Self("🗖 展开程序".to_owned()),
        }
    }
}

struct App {
    config: Option<Config>,
    root_node_connection_state: ArcMutex<ConnectionState>,
    state_bar_message: ArcMutex<Option<Message>>,
    ui_mode: UIMode,
    ui_mode_switch_button_text: UIModeSwitchButtonText,
    ui_mode_switch_inner_size: Option<egui::Vec2>,
    side_bar_is_show: bool,
    connect_root_node_ui_is_enable: ArcMutex<bool>,
    endpoint: ArcMutex<Option<Endpoint>>,
    root_cert_store: RootCertStore,
    root_node_connection: ArcMutex<Option<Connection>>,
}
impl App {
    fn new() -> Self {
        let mut config = None;
        let endpoint = ArcMutex::new(None);
        let mut root_cert_store = RootCertStore::empty();
        let state_bar_message = ArcMutex::new(None);
        let connect_root_node_ui_is_enable = ArcMutex::new(false);
        //加载配置
        match load_config() {
            Ok(cfg) => config = Some(cfg),
            Err(err) => {
                *state_bar_message.lock() =
                    Some(Message::Error(format!("加载配置失败！原因：{}", err)))
            }
        }
        //加载证书目录证书
        match load_certs_path_root_cert_store() {
            Ok(rcs) => root_cert_store = rcs,
            Err(err) => {
                *state_bar_message.lock() = Some(Message::Error(format!(
                    "加载证书目录证书失败！原因：{}",
                    err
                )))
            }
        }
        //创建节点
        tokio::spawn({
            let endpoint = endpoint.clone();
            let state_bar_message = state_bar_message.clone();
            let connect_root_node_ui_is_enable = connect_root_node_ui_is_enable.clone();
            async move {
                match new_endpoint() {
                    Ok(ep) => {
                        {
                            *endpoint.lock() = Some(ep);
                        }
                        {
                            *connect_root_node_ui_is_enable.lock() = true;
                        }
                    }
                    Err(_) => {
                        *state_bar_message.lock() =
                            Some(Message::Error("创建节点失败！".to_owned()))
                    }
                }
            }
        });
        Self {
            config,
            root_node_connection_state: ArcMutex::new(ConnectionState::Disconnect),
            state_bar_message,
            ui_mode: UIMode::Fold,
            ui_mode_switch_button_text: UIMode::Fold.into(),
            ui_mode_switch_inner_size: Some(egui::Vec2::new(1150., 750.)),
            side_bar_is_show: false,
            connect_root_node_ui_is_enable,
            endpoint,
            root_cert_store,
            root_node_connection: ArcMutex::new(None),
        }
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("MenuBar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .button(String::from(self.ui_mode_switch_button_text.clone()))
                    .clicked()
                {
                    match self.ui_mode {
                        UIMode::Unfold => {
                            self.ui_mode = UIMode::Fold;
                            self.ui_mode_switch_button_text = self.ui_mode.clone().into();
                            if let Some(self_inner_size) = self.ui_mode_switch_inner_size {
                                let inner_size =
                                    ctx.input(|is| is.viewport().inner_rect).map(|v| v.size());
                                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                                    self_inner_size,
                                ));
                                self.ui_mode_switch_inner_size = inner_size;
                            }
                            self.side_bar_is_show = false;
                        }
                        UIMode::Fold => {
                            self.ui_mode = UIMode::Unfold;
                            self.ui_mode_switch_button_text = self.ui_mode.clone().into();
                            if let Some(self_inner_size) = self.ui_mode_switch_inner_size {
                                let inner_size =
                                    ctx.input(|is| is.viewport().inner_rect).map(|v| v.size());
                                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                                    self_inner_size,
                                ));
                                self.ui_mode_switch_inner_size = inner_size;
                            }
                            self.side_bar_is_show = true;
                        }
                    }
                }
                ui.menu_button("关于", |ui| {
                    ui.vertical(|ui| {
                        ui.label("版本：0.1.0");
                        ui.label("作者：✨张喜昌✨");
                        if ui.link("源代码").clicked() {
                            opener::open("https://github.com/ZhangXiChang/node-network").unwrap();
                        }
                    });
                });
            });
        });
        egui::TopBottomPanel::bottom("StateBar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        {
                            let a = self.connect_root_node_ui_is_enable.lock().clone();
                            a
                        },
                        egui::Button::new("🖧 连接根节点"),
                    )
                    .clicked()
                {
                    if let (Some(endpoint), Some(config)) = (
                        {
                            let a = self.endpoint.lock().clone();
                            a
                        },
                        self.config.clone(),
                    ) {
                        {
                            *self.connect_root_node_ui_is_enable.lock() = false;
                        }
                        {
                            *self.root_node_connection_state.lock() = ConnectionState::Connecting;
                        }
                        tokio::spawn({
                            let root_node_connection = self.root_node_connection.clone();
                            let connect_root_node_ui_is_enable =
                                self.connect_root_node_ui_is_enable.clone();
                            let state_bar_message = self.state_bar_message.clone();
                            let root_node_connection_state =
                                self.root_node_connection_state.clone();
                            let root_cert_store = self.root_cert_store.clone();
                            async move {
                                match connect_root_node(config, endpoint, root_cert_store).await {
                                    Ok(connection) => {
                                        {
                                            *root_node_connection.lock() = Some(connection.clone());
                                        }
                                        {
                                            *root_node_connection_state.lock() =
                                                ConnectionState::Connected;
                                        }
                                        {
                                            *state_bar_message.lock() = Some(Message::Info(
                                                "连接根节点成功啦~✨，快去玩耍吧~".to_owned(),
                                            ));
                                        }
                                        connection.closed().await;
                                        {
                                            *root_node_connection.lock() = None;
                                        }
                                        {
                                            *connect_root_node_ui_is_enable.lock() = true;
                                        }
                                        {
                                            *root_node_connection_state.lock() =
                                                ConnectionState::Disconnect;
                                        }
                                        {
                                            *state_bar_message.lock() = Some(Message::Error(
                                                "根节点断开连接惹！盖亚！💢".to_owned(),
                                            ));
                                        }
                                    }
                                    Err(_) => {
                                        {
                                            *connect_root_node_ui_is_enable.lock() = true;
                                        }
                                        {
                                            *root_node_connection_state.lock() =
                                                ConnectionState::Disconnect;
                                        }
                                        {
                                            *state_bar_message.lock() = Some(Message::Error(
                                                "连接根节点失败惹！可恶💢".to_owned(),
                                            ));
                                        }
                                    }
                                }
                            }
                        });
                    }
                }
                match {
                    let a = self.root_node_connection_state.lock().clone();
                    a
                } {
                    ConnectionState::Connected => {
                        ui.colored_label(egui::Color32::LIGHT_GREEN, "🌏 根节点已连接")
                    }
                    ConnectionState::Disconnect => {
                        ui.colored_label(egui::Color32::LIGHT_RED, "❌ 根节点未连接")
                    }
                    ConnectionState::Connecting => {
                        ui.colored_label(egui::Color32::LIGHT_BLUE, "⏳ 根节点连接中...")
                    }
                };
                ui.label("|");
                if let Some(msg) = {
                    let a = self.state_bar_message.lock().clone();
                    a
                } {
                    match msg {
                        Message::Info(msg_str) => {
                            ui.colored_label(egui::Color32::LIGHT_GRAY, msg_str)
                        }
                        Message::Error(msg_str) => {
                            ui.colored_label(egui::Color32::LIGHT_RED, msg_str)
                        }
                    };
                }
            });
        });
        if self.side_bar_is_show {
            egui::SidePanel::left("SideBar").show(ctx, |ui| {
                ui.label("侧边栏");
            });
        }
        egui::CentralPanel::default().show(ctx, |ui| match self.ui_mode {
            UIMode::Unfold => (),
            UIMode::Fold => {
                ui.horizontal_top(|ui| {
                    ui.add(
                        egui::Image::new(egui::ImageSource::Bytes {
                            uri: Cow::default(),
                            bytes: egui::load::Bytes::Static(ICON),
                        })
                        .max_size(egui::Vec2::new(
                            ICON_WIDTH as f32 * 0.3,
                            ICON_HEIGHT as f32 * 0.3,
                        )),
                    );
                    ui.vertical_centered(|ui| {
                        ui.heading("节点网络");
                        ui.add_space(10.);
                        ui.label("版本：0.1.0");
                        ui.label("作者：✨张喜昌✨");
                        if ui.link("源代码").clicked() {
                            let _ = opener::open("https://github.com/ZhangXiChang/node-network");
                        }
                    });
                });
                ui.label("======================================================================");
                ui.vertical_centered(|ui| {
                    ui.label("这里是作者玩耍的地方，✨欸嘿✨");
                });
            }
        });
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    //运行应用
    eframe::run_native(
        "节点网络",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder {
                icon: Some(Arc::new(egui::IconData {
                    rgba: image::load_from_memory(ICON)?.into_bytes(),
                    width: ICON_WIDTH,
                    height: ICON_HEIGHT,
                })),
                inner_size: Some(egui::Vec2::new(ICON_WIDTH as f32, ICON_HEIGHT as f32 + 50.)),
                resizable: Some(false),
                ..Default::default()
            },
            ..Default::default()
        },
        Box::new(|cc| {
            set_font(&cc.egui_ctx);
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::new(App::new())
        }),
    )
    .map_err(|err| eyre!("{}", err))?;
    Ok(())
}
fn set_font(ctx: &egui::Context) {
    let mut font_definitions = egui::FontDefinitions::default();
    font_definitions.font_data.insert(
        "SourceHanSansCN-Bold".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/SourceHanSansCN-Bold.otf"
        )),
    );
    font_definitions
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "SourceHanSansCN-Bold".to_owned());
    font_definitions
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("SourceHanSansCN-Bold".to_owned());
    ctx.set_fonts(font_definitions);
}
fn load_config() -> Result<Config> {
    //解析配置文件
    let mut config = Config {
        node_name: "无名氏".to_owned(),
        root_node: RootNode {
            node_name: "root_node".to_owned(),
            socket_addr: "127.0.0.1:10270".parse()?,
        },
    };
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
fn load_certs_path_root_cert_store() -> Result<RootCertStore> {
    let mut root_cert_store = RootCertStore::empty();
    let cert_dir_path = PathBuf::from("./certs/");
    create_dir_all(cert_dir_path.clone())?;
    for dir_entry in cert_dir_path.read_dir()? {
        if let Ok(dir_entry) = dir_entry {
            let path = dir_entry.path();
            if let Some(extension) = path.extension() {
                if extension == "cer" {
                    let mut root_node_cert = Vec::new();
                    File::open(path)?.read_to_end(&mut root_node_cert)?;
                    root_cert_store.add(&rustls::Certificate(root_node_cert))?;
                }
            }
        }
    }
    Ok(root_cert_store)
}
fn new_endpoint() -> Result<Endpoint> {
    let rcgen::CertifiedKey { cert, key_pair } =
        rcgen::generate_simple_self_signed(vec![Uuid::new_v4().to_string()])?;
    let mut transport_config = TransportConfig::default();
    transport_config.keep_alive_interval(Some(Duration::from_secs(5)));
    Ok(Endpoint::server(
        ServerConfig::with_single_cert(
            vec![rustls::Certificate(cert.der().to_vec())],
            rustls::PrivateKey(key_pair.serialize_der()),
        )?
        .transport_config(Arc::new(transport_config))
        .clone(),
        "0.0.0.0:0".parse()?,
    )?)
}
async fn connect_root_node(
    config: Config,
    endpoint: Endpoint,
    root_cert_store: RootCertStore,
) -> Result<Connection> {
    Ok(endpoint
        .connect_with(
            ClientConfig::with_root_certificates(root_cert_store),
            config.root_node.socket_addr,
            &config.root_node.node_name,
        )?
        .await?)
}
