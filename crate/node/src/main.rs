use std::{
    borrow::Cow,
    fs::{create_dir_all, File},
    io::Read,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use eyre::{eyre, Result};

use eframe::egui;
use quinn::{ClientConfig, Connection, Endpoint, ServerConfig, TransportConfig, VarInt};
use rustls::RootCertStore;
use share::ArcMutex;
use uuid::Uuid;

const ICON: &[u8] = include_bytes!("../../../assets/icon/node_network_icon.png");
const ICON_WIDTH: u32 = 512;
const ICON_HEIGHT: u32 = 512;

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
    Fold,
    Unfold,
}
impl From<UIMode> for UIModeSwitchButtonText {
    fn from(value: UIMode) -> Self {
        match value {
            UIMode::Fold => Self("🗖 展开程序".to_owned()),
            UIMode::Unfold => Self("🗕 折叠程序".to_owned()),
        }
    }
}

#[derive(PartialEq)]
enum ModeView {
    Readme,
    Connect,
}

struct NodeMessage {
    node_name: String,
    msg: String,
}

struct App {
    //UI状态
    root_node_connection_state: ArcMutex<ConnectionState>,
    state_bar_message: ArcMutex<Option<Message>>,
    ui_mode: UIMode,
    fold_mode_current_view: ModeView,
    ui_mode_switch_button_text: UIModeSwitchButtonText,
    ui_mode_switch_inner_size: Option<egui::Vec2>,
    left_side_bar_is_show: bool,
    connect_root_node_ui_is_enable: ArcMutex<bool>,
    chat_input_str: String,
    chat_input_bar_is_enable: ArcMutex<bool>,
    chat_bar_text_list: Vec<NodeMessage>,
    ui_root_node_dns_name: String,
    ui_root_node_socket_addr: String,
    ui_self_name: String,
    //通信状态
    endpoint: ArcMutex<Option<Endpoint>>,
    root_cert_store: RootCertStore,
    root_node_connection: ArcMutex<Option<Connection>>,
}
impl App {
    fn new() -> Self {
        let state_bar_message = ArcMutex::new(None);
        //创建节点
        let endpoint = ArcMutex::new(None);
        let connect_root_node_ui_is_enable = ArcMutex::new(false);
        tokio::spawn({
            let state_bar_message = state_bar_message.clone();
            let endpoint = endpoint.clone();
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
        //加载证书目录证书
        let mut root_cert_store = RootCertStore::empty();
        match load_certs_path_root_cert_store() {
            Ok(rcs) => root_cert_store = rcs,
            Err(err) => {
                *state_bar_message.lock() = Some(Message::Error(format!(
                    "加载证书目录证书失败！原因：{}",
                    err
                )))
            }
        }
        Self {
            root_node_connection_state: ArcMutex::new(ConnectionState::Disconnect),
            state_bar_message,
            ui_mode: UIMode::Fold,
            fold_mode_current_view: ModeView::Readme,
            ui_mode_switch_button_text: UIMode::Fold.into(),
            ui_mode_switch_inner_size: Some(egui::Vec2::new(1150., 750.)),
            left_side_bar_is_show: false,
            connect_root_node_ui_is_enable,
            chat_input_str: String::new(),
            chat_input_bar_is_enable: ArcMutex::new(false),
            chat_bar_text_list: vec![],
            ui_root_node_dns_name: "root_node".to_owned(),
            ui_root_node_socket_addr: "127.0.0.1".to_owned(),
            ui_self_name: String::new(),
            endpoint,
            root_cert_store,
            root_node_connection: ArcMutex::new(None),
        }
    }
    fn connect_root_node_for_tokio(&mut self, endpoint: Endpoint) {
        tokio::spawn({
            let root_node_connection = self.root_node_connection.clone();
            let connect_root_node_ui_is_enable = self.connect_root_node_ui_is_enable.clone();
            let state_bar_message = self.state_bar_message.clone();
            let root_node_connection_state = self.root_node_connection_state.clone();
            let chat_input_bar_is_enable = self.chat_input_bar_is_enable.clone();
            let root_cert_store = self.root_cert_store.clone();
            let ui_root_node_dns_name = self.ui_root_node_dns_name.clone();
            let ui_root_node_socket_addr = self.ui_root_node_socket_addr.clone();
            async move {
                match connect_root_node(
                    endpoint,
                    root_cert_store,
                    ui_root_node_dns_name,
                    ui_root_node_socket_addr,
                )
                .await
                {
                    Ok(connection) => {
                        {
                            *root_node_connection.lock() = Some(connection.clone());
                        }
                        {
                            *chat_input_bar_is_enable.lock() = true;
                        }
                        {
                            *root_node_connection_state.lock() = ConnectionState::Connected;
                        }
                        {
                            *state_bar_message.lock() =
                                Some(Message::Info("连接根节点成功啦~✨，快去玩耍吧~".to_owned()));
                        }
                        connection.closed().await;
                        {
                            *root_node_connection.lock() = None;
                        }
                        {
                            *chat_input_bar_is_enable.lock() = false;
                        }
                        {
                            *connect_root_node_ui_is_enable.lock() = true;
                        }
                        {
                            *root_node_connection_state.lock() = ConnectionState::Disconnect;
                        }
                        {
                            *state_bar_message.lock() = Some(Message::Info(
                                "根节点断开连接惹！不要离开我呀~😭".to_owned(),
                            ));
                        }
                    }
                    Err(_) => {
                        {
                            *connect_root_node_ui_is_enable.lock() = true;
                        }
                        {
                            *root_node_connection_state.lock() = ConnectionState::Disconnect;
                        }
                        {
                            *state_bar_message.lock() =
                                Some(Message::Error("连接根节点失败惹！可恶💢".to_owned()));
                        }
                    }
                }
            }
        });
    }
}
impl Drop for App {
    fn drop(&mut self) {
        if let Some(endpoint) = {
            let a = self.endpoint.lock().clone();
            a
        } {
            endpoint.close(VarInt::from_u32(0), "程序关闭".as_bytes());
        }
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("MenuBar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("切换视图", |ui| match self.ui_mode {
                    UIMode::Fold => {
                        ui.radio_value(
                            &mut self.fold_mode_current_view,
                            ModeView::Readme,
                            "自述视图",
                        );
                        ui.radio_value(
                            &mut self.fold_mode_current_view,
                            ModeView::Connect,
                            "连接视图",
                        );
                    }
                    UIMode::Unfold => {
                        ui.label("空");
                    }
                });
                ui.menu_button("关于", |ui| {
                    ui.label("版本：0.1.0");
                    ui.label("作者：✨张喜昌✨");
                    if ui.link("源代码").clicked() {
                        opener::open("https://github.com/ZhangXiChang/node-network").unwrap();
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(String::from(self.ui_mode_switch_button_text.clone()))
                        .clicked()
                    {
                        match self.ui_mode {
                            UIMode::Fold => {
                                self.ui_mode = UIMode::Unfold;
                                self.ui_mode_switch_button_text = self.ui_mode.clone().into();
                                if let Some(self_inner_size) = self.ui_mode_switch_inner_size {
                                    let inner_size =
                                        ctx.input(|i| i.viewport().inner_rect).map(|v| v.size());
                                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                                        self_inner_size,
                                    ));
                                    self.ui_mode_switch_inner_size = inner_size;
                                }
                                self.left_side_bar_is_show = true;
                            }
                            UIMode::Unfold => {
                                self.ui_mode = UIMode::Fold;
                                self.ui_mode_switch_button_text = self.ui_mode.clone().into();
                                if let Some(self_inner_size) = self.ui_mode_switch_inner_size {
                                    let inner_size =
                                        ctx.input(|i| i.viewport().inner_rect).map(|v| v.size());
                                    ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                                        self_inner_size,
                                    ));
                                    self.ui_mode_switch_inner_size = inner_size;
                                }
                                self.left_side_bar_is_show = false;
                            }
                        }
                    }
                });
            });
        });
        egui::TopBottomPanel::bottom("StateBar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("根节点状态:");
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
                        Message::Info(msg_str) => ui.colored_label(egui::Color32::GRAY, msg_str),
                        Message::Error(msg_str) => {
                            ui.colored_label(egui::Color32::LIGHT_RED, msg_str)
                        }
                    };
                }
            });
        });
        if self.left_side_bar_is_show {
            egui::SidePanel::left("LeftSideBar").show(ctx, |_ui| {
                //TODO 展开模式侧边栏
            });
        }
        egui::CentralPanel::default().show(ctx, |ui| match self.ui_mode {
            UIMode::Fold => match self.fold_mode_current_view {
                ModeView::Readme => {
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
                                let _ =
                                    opener::open("https://github.com/ZhangXiChang/node-network");
                            }
                        });
                    });
                    ui.label(
                        "=====================================================================",
                    );
                    ui.vertical_centered(|ui| {
                        ui.label("这里是作者玩耍的地方，✨欸嘿✨");
                    });
                }
                ModeView::Connect => {
                    //TODO 连接视图界面设计
                    ui.vertical_centered(|ui| {
                        ui.add_space((ui.available_height() / 2.) - 70.);
                        ui.add_enabled_ui(
                            {
                                let a = self.connect_root_node_ui_is_enable.lock().clone();
                                a
                            },
                            |ui| {
                                ui.allocate_ui(egui::Vec2::new(200., 0.), |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label("昵称:");
                                        ui.text_edit_singleline(&mut self.ui_self_name);
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("根节点DNSName:");
                                        ui.text_edit_singleline(&mut self.ui_root_node_dns_name);
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("根节点IP地址:");
                                        ui.text_edit_singleline(&mut self.ui_root_node_socket_addr);
                                    });
                                });
                                ui.add_enabled_ui(
                                    !self.ui_self_name.is_empty()
                                        && !self.ui_root_node_dns_name.is_empty()
                                        && !self.ui_root_node_socket_addr.is_empty(),
                                    |ui| {
                                        if ui.button("🖧 连接根节点").clicked() {
                                            if let Some(endpoint) = {
                                                let a = self.endpoint.lock().clone();
                                                a
                                            } {
                                                {
                                                    *self.connect_root_node_ui_is_enable.lock() =
                                                        false;
                                                }
                                                {
                                                    *self.root_node_connection_state.lock() =
                                                        ConnectionState::Connecting;
                                                }
                                                self.connect_root_node_for_tokio(endpoint);
                                            }
                                        }
                                    },
                                );
                            },
                        );
                        ui.add_enabled_ui(
                            !{
                                let a = self.connect_root_node_ui_is_enable.lock().clone();
                                a
                            } && {
                                let a = self.root_node_connection.lock().is_some();
                                a
                            },
                            |ui| {
                                if ui.button("断开连接").clicked() {
                                    if let Some(root_node_connection) = {
                                        let a = self.root_node_connection.lock().clone();
                                        a
                                    } {
                                        root_node_connection
                                            .close(VarInt::from_u32(0), "手动关闭连接".as_bytes());
                                    }
                                }
                            },
                        );
                    });
                }
            },
            UIMode::Unfold => {
                egui::TopBottomPanel::bottom("CentralPanel-BottomPanel").show_inside(ui, |ui| {
                    ui.add_space(10.);
                    if ui
                        .add_enabled(
                            {
                                let a = self.chat_input_bar_is_enable.lock().clone();
                                a
                            },
                            egui::TextEdit::multiline(&mut self.chat_input_str)
                                .desired_width(ui.available_width()),
                        )
                        .has_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        self.chat_bar_text_list.push(NodeMessage {
                            node_name: self.ui_self_name.clone(),
                            msg: self.chat_input_str.trim().to_owned(),
                        });
                        self.chat_input_str.clear();
                    }
                });
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for node_msg in self.chat_bar_text_list.iter() {
                                ui.horizontal(|ui| {
                                    //TODO 实现用户头像
                                    ui.label(node_msg.node_name.clone());
                                    ui.group(|ui| {
                                        ui.add(egui::Label::new(node_msg.msg.clone()).wrap(true));
                                    });
                                });
                            }
                        });
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
                inner_size: Some(egui::Vec2::new(500., 500. + 50.)),
                resizable: Some(false),
                maximize_button: Some(false),
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
// fn load_config() -> Result<Config> {
//     //解析配置文件
//     let mut config = Config {
//         node_name: "无名氏".to_owned(),
//         root_node: RootNode {
//             node_name: "root_node".to_owned(),
//             socket_addr: "127.0.0.1:10270".parse()?,
//         },
//     };
//     let config_file_path = PathBuf::from("./config.json");
//     match File::open(config_file_path.clone()) {
//         Ok(mut config_file) => {
//             let mut config_bytes = Vec::new();
//             config_file.read_to_end(&mut config_bytes)?;
//             config = serde_json::from_slice(&config_bytes)?;
//         }
//         Err(_) => {
//             config.serialize(&mut serde_json::Serializer::with_formatter(
//                 File::create(config_file_path)?,
//                 serde_json::ser::PrettyFormatter::with_indent(b"    "),
//             ))?;
//         }
//     }
//     Ok(config)
// }
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
    endpoint: Endpoint,
    root_cert_store: RootCertStore,
    root_node_name: String,
    root_node_socket_addr: String,
) -> Result<Connection> {
    Ok(endpoint
        .connect_with(
            ClientConfig::with_root_certificates(root_cert_store),
            root_node_socket_addr.parse()?,
            &root_node_name,
        )?
        .await?)
}
