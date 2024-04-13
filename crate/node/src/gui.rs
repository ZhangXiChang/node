use std::{
    borrow::Cow,
    fs::{create_dir_all, File},
    io::Read,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};

use eframe::egui;
use eyre::Result;
use quinn::{ClientConfig, Connection, Endpoint, ServerConfig, TransportConfig, VarInt};
use rcgen::CertifiedKey;
use rustls::{Certificate, PrivateKey, RootCertStore};
use serde::{Deserialize, Serialize};
use share_code::ArcMutex;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct RootNodeInfo {
    name: String,
    dns_name: String,
    socket_addr: SocketAddr,
}
#[derive(Serialize, Deserialize)]
struct Config {
    user_name: String,
    root_node_info_list: Vec<RootNodeInfo>,
}

#[derive(Clone)]
enum Log {
    Info(String),
    Error(String),
}

#[derive(Clone)]
enum AppUILayoutState {
    Fold,
    Unfold,
}

#[derive(Clone)]
struct AppUILayoutStateSwitchButtonText(String);
impl From<AppUILayoutStateSwitchButtonText> for String {
    fn from(value: AppUILayoutStateSwitchButtonText) -> Self {
        let AppUILayoutStateSwitchButtonText(text) = value;
        text
    }
}
impl From<AppUILayoutState> for AppUILayoutStateSwitchButtonText {
    fn from(value: AppUILayoutState) -> Self {
        match value {
            AppUILayoutState::Fold => Self("🗖 展开程序".to_owned()),
            AppUILayoutState::Unfold => Self("🗕 折叠程序".to_owned()),
        }
    }
}

#[derive(Clone, PartialEq)]
enum FoldCentralPanelUILayoutState {
    Readme,
    ConnectRootNode,
}

#[derive(Clone)]
enum ConnectionState {
    Connected,
    Disconnect,
    Connecting,
}

#[derive(Clone)]
struct Message {
    src_user_name: String,
    text: String,
}
struct MessageBar {
    msg_logs: Vec<Message>,
}
struct TextInputBar {
    is_enable: ArcMutex<bool>,
    input_text: String,
}
struct ChatBar {
    message_bar: MessageBar,
    text_input_bar: TextInputBar,
}

struct MenuBar {
    app_ui_layout_state_switch_button_text: AppUILayoutStateSwitchButtonText,
}

struct StateBar {
    log: ArcMutex<Option<Log>>,
}

struct FoldCentralPanel {
    ui_layout_state: FoldCentralPanelUILayoutState,
    root_node_connect_ui_is_enable: ArcMutex<bool>,
    root_node_selected: usize,
}
struct UnFoldCentralPanel {
    chat_bar: ChatBar,
}

pub struct GUI {
    //应用配置
    config: Config,
    //图形界面
    menu_bar: MenuBar,
    state_bar: StateBar,
    fold_central_panel: FoldCentralPanel,
    unfold_central_panel: UnFoldCentralPanel,
    ui_layout_state: AppUILayoutState,
    ui_layout_state_switch_next_window_inner_size: Option<egui::Vec2>,
    left_side_bar_is_show: bool,
    root_node_connection_state: ArcMutex<ConnectionState>,
    //网络通信
    endpoint: Endpoint,
    root_node_connection: ArcMutex<Option<Connection>>,
}
impl GUI {
    pub fn new() -> Result<Self> {
        Ok(Self {
            config: Self::load_config()?,
            menu_bar: MenuBar {
                app_ui_layout_state_switch_button_text: AppUILayoutState::Fold.into(),
            },
            state_bar: StateBar {
                log: ArcMutex::new(None),
            },
            fold_central_panel: FoldCentralPanel {
                ui_layout_state: FoldCentralPanelUILayoutState::Readme,
                root_node_connect_ui_is_enable: ArcMutex::new(true),
                root_node_selected: 0,
            },
            unfold_central_panel: UnFoldCentralPanel {
                chat_bar: ChatBar {
                    message_bar: MessageBar { msg_logs: vec![] },
                    text_input_bar: TextInputBar {
                        is_enable: ArcMutex::new(false),
                        input_text: "".to_owned(),
                    },
                },
            },
            ui_layout_state: AppUILayoutState::Fold,
            ui_layout_state_switch_next_window_inner_size: Some(egui::Vec2::new(1150., 750.)),
            left_side_bar_is_show: false,
            root_node_connection_state: ArcMutex::new(ConnectionState::Disconnect),
            endpoint: Endpoint::server(
                {
                    let CertifiedKey { cert, key_pair } =
                        rcgen::generate_simple_self_signed(vec![Uuid::new_v4().to_string()])?;
                    ServerConfig::with_single_cert(
                        vec![Certificate(cert.der().to_vec())],
                        PrivateKey(key_pair.serialize_der()),
                    )?
                    .transport_config(Arc::new({
                        let mut a = TransportConfig::default();
                        a.keep_alive_interval(Some(Duration::from_secs(5)));
                        a
                    }))
                    .to_owned()
                },
                "0.0.0.0:0".parse()?,
            )?,
            root_node_connection: ArcMutex::new(None),
        })
    }
    fn load_config() -> Result<Config> {
        //初始配置
        let mut config = Config {
            user_name: "".to_owned(),
            root_node_info_list: vec![RootNodeInfo {
                name: "默认根节点".to_owned(),
                dns_name: "root_node".to_owned(),
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
    fn write_user_name_to_config(user_name: String) -> Result<()> {
        let config_file_path = PathBuf::from("./config.json");
        let mut config_bytes = Vec::new();
        File::open(config_file_path.clone())?.read_to_end(&mut config_bytes)?;
        let mut config = serde_json::from_slice::<Config>(&config_bytes)?;
        config.user_name = user_name;
        config.serialize(&mut serde_json::Serializer::with_formatter(
            File::create(config_file_path)?,
            serde_json::ser::PrettyFormatter::with_indent(b"    "),
        ))?;
        Ok(())
    }
    fn window_ui_layout_state_switch_to(
        app_ui_layout_state: AppUILayoutState,
        ctx: &egui::Context,
        dst_app_ui_layout_state: &mut AppUILayoutState,
        ui_layout_state_switch_next_window_inner_size: &mut Option<egui::Vec2>,
        app_ui_layout_state_switch_button_text: &mut AppUILayoutStateSwitchButtonText,
        left_side_bar_is_show: &mut bool,
    ) {
        if let Some(next_window_inner_size) = ui_layout_state_switch_next_window_inner_size.clone()
        {
            *dst_app_ui_layout_state = app_ui_layout_state;
            *app_ui_layout_state_switch_button_text = dst_app_ui_layout_state.clone().into();
            *ui_layout_state_switch_next_window_inner_size =
                ctx.input(|i| i.viewport().inner_rect).map(|v| v.size());
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(next_window_inner_size));
            match *dst_app_ui_layout_state {
                AppUILayoutState::Fold => *left_side_bar_is_show = false,
                AppUILayoutState::Unfold => *left_side_bar_is_show = true,
            }
        }
    }
    async fn connect_root_node(
        endpoint: Endpoint,
        socket_addr: SocketAddr,
        dns_name: String,
    ) -> Result<Connection> {
        Ok(endpoint
            .connect_with(
                ClientConfig::with_root_certificates({
                    let mut a = RootCertStore::empty();
                    let cert_dir_path = PathBuf::from("./certs/");
                    create_dir_all(cert_dir_path.clone())?;
                    for dir_entry in cert_dir_path.read_dir()? {
                        if let Ok(dir_entry) = dir_entry {
                            let path = dir_entry.path();
                            if let Some(extension) = path.extension() {
                                if extension == "cer" {
                                    let mut cert_der = Vec::new();
                                    File::open(path)?.read_to_end(&mut cert_der)?;
                                    a.add(&Certificate(cert_der))?;
                                }
                            }
                        }
                    }
                    a
                }),
                socket_addr,
                &dns_name,
            )?
            .await?)
    }
    fn connect_root_node_for_tokio(&self) {
        tokio::spawn({
            let endpoint = self.endpoint.clone();
            let root_node_socket_addr = self.config.root_node_info_list
                [self.fold_central_panel.root_node_selected]
                .socket_addr
                .clone();
            let root_node_dns_name = self.config.root_node_info_list
                [self.fold_central_panel.root_node_selected]
                .dns_name
                .clone();
            let root_node_connection = self.root_node_connection.clone();
            let chat_bar_text_input_bar_is_enable = self
                .unfold_central_panel
                .chat_bar
                .text_input_bar
                .is_enable
                .clone();
            let root_node_connection_state = self.root_node_connection_state.clone();
            let state_bar_log = self.state_bar.log.clone();
            let root_node_connect_ui_is_enable = self
                .fold_central_panel
                .root_node_connect_ui_is_enable
                .clone();
            async move {
                match Self::connect_root_node(endpoint, root_node_socket_addr, root_node_dns_name)
                    .await
                {
                    Ok(connection) => {
                        {
                            *root_node_connection.lock() = Some(connection.clone());
                        }
                        {
                            *chat_bar_text_input_bar_is_enable.lock() = true;
                        }
                        {
                            *root_node_connection_state.lock() = ConnectionState::Connected;
                        }
                        {
                            *state_bar_log.lock() =
                                Some(Log::Info("连接根节点成功啦~✨，快去玩耍吧~".to_owned()));
                        }
                        connection.closed().await;
                        {
                            *root_node_connection.lock() = None;
                        }
                        {
                            *chat_bar_text_input_bar_is_enable.lock() = false;
                        }
                        {
                            *root_node_connection_state.lock() = ConnectionState::Disconnect;
                        }
                        {
                            *state_bar_log.lock() =
                                Some(Log::Info("根节点断开连接惹！不要离开我呀~😭".to_owned()));
                        }
                        {
                            *root_node_connect_ui_is_enable.lock() = true;
                        }
                    }
                    Err(_) => {
                        {
                            *root_node_connection_state.lock() = ConnectionState::Disconnect;
                        }
                        {
                            *state_bar_log.lock() =
                                Some(Log::Error("连接根节点失败惹！可恶💢".to_owned()));
                        }
                        {
                            *root_node_connect_ui_is_enable.lock() = true;
                        }
                    }
                }
            }
        });
    }
}
impl Default for GUI {
    fn default() -> Self {
        match Self::new() {
            Ok(selfa) => selfa,
            Err(err) => panic!("{}", err),
        }
    }
}
impl Drop for GUI {
    fn drop(&mut self) {
        self.endpoint
            .close(VarInt::from_u32(0), "程序关闭".as_bytes());
    }
}
impl eframe::App for GUI {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("MenuBar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("关于", |ui| {
                    ui.label("版本：0.1.0");
                    ui.label("作者：✨张喜昌✨");
                    if ui.link("源代码").clicked() {
                        opener::open("https://github.com/ZhangXiChang/node-network").unwrap();
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(String::from(
                            self.menu_bar.app_ui_layout_state_switch_button_text.clone(),
                        ))
                        .clicked()
                    {
                        match self.ui_layout_state {
                            AppUILayoutState::Fold => Self::window_ui_layout_state_switch_to(
                                AppUILayoutState::Unfold,
                                ctx,
                                &mut self.ui_layout_state,
                                &mut self.ui_layout_state_switch_next_window_inner_size,
                                &mut self.menu_bar.app_ui_layout_state_switch_button_text,
                                &mut self.left_side_bar_is_show,
                            ),
                            AppUILayoutState::Unfold => Self::window_ui_layout_state_switch_to(
                                AppUILayoutState::Fold,
                                ctx,
                                &mut self.ui_layout_state,
                                &mut self.ui_layout_state_switch_next_window_inner_size,
                                &mut self.menu_bar.app_ui_layout_state_switch_button_text,
                                &mut self.left_side_bar_is_show,
                            ),
                        }
                    }
                    match self.ui_layout_state {
                        AppUILayoutState::Fold => {
                            ui.menu_button("切换视图", |ui| {
                                ui.radio_value(
                                    &mut self.fold_central_panel.ui_layout_state,
                                    FoldCentralPanelUILayoutState::Readme,
                                    "自述视图",
                                );
                                ui.radio_value(
                                    &mut self.fold_central_panel.ui_layout_state,
                                    FoldCentralPanelUILayoutState::ConnectRootNode,
                                    "连接根节点视图",
                                );
                            });
                        }
                        AppUILayoutState::Unfold => (),
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
                        ui.colored_label(egui::Color32::LIGHT_GREEN, "🌏 已连接")
                    }
                    ConnectionState::Disconnect => {
                        ui.colored_label(egui::Color32::LIGHT_RED, "❌ 未连接")
                    }
                    ConnectionState::Connecting => {
                        ui.colored_label(egui::Color32::LIGHT_BLUE, "⏳ 连接中...")
                    }
                };
                ui.label("|");
                if let Some(msg) = {
                    let a = self.state_bar.log.lock().clone();
                    a
                } {
                    match msg {
                        Log::Info(text) => ui.colored_label(egui::Color32::GRAY, text),
                        Log::Error(text) => ui.colored_label(egui::Color32::LIGHT_RED, text),
                    };
                }
            });
        });
        if self.left_side_bar_is_show {
            egui::SidePanel::left("LeftSideBar").show(ctx, |_ui| {});
        }
        egui::CentralPanel::default().show(ctx, |ui| match self.ui_layout_state {
            AppUILayoutState::Fold => match self.fold_central_panel.ui_layout_state {
                FoldCentralPanelUILayoutState::Readme => {
                    ui.horizontal_top(|ui| {
                        ui.add(
                            egui::Image::new(egui::ImageSource::Bytes {
                                uri: Cow::default(),
                                bytes: egui::load::Bytes::Static(include_bytes!(
                                    "../../../assets/icon/node_network_icon.png"
                                )),
                            })
                            .max_size(egui::Vec2::new(512. * 0.3, 512. * 0.3)),
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
                FoldCentralPanelUILayoutState::ConnectRootNode => {
                    ui.vertical_centered(|ui| {
                        ui.add_space((ui.available_height() / 2.) - 70.);
                        ui.add_enabled_ui(
                            {
                                let a = self
                                    .fold_central_panel
                                    .root_node_connect_ui_is_enable
                                    .lock()
                                    .clone();
                                a
                            },
                            |ui| {
                                ui.allocate_ui(egui::Vec2::new(200., 0.), |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label("昵称");
                                        if ui
                                            .text_edit_singleline(&mut self.config.user_name)
                                            .lost_focus()
                                        {
                                            match Self::write_user_name_to_config(
                                                self.config.user_name.clone(),
                                            ) {
                                                Ok(_) => (),
                                                Err(err) => {
                                                    *self.state_bar.log.lock() = Some(Log::Error(
                                                        format!("配置保存错误！原因：{}", err),
                                                    ))
                                                }
                                            }
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        egui::ComboBox::from_label("根节点")
                                            .width(ui.available_width())
                                            .show_index(
                                                ui,
                                                &mut self.fold_central_panel.root_node_selected,
                                                self.config.root_node_info_list.len(),
                                                |i| self.config.root_node_info_list[i].name.clone(),
                                            );
                                    });
                                });
                                ui.add_enabled_ui(!self.config.user_name.is_empty(), |ui| {
                                    if ui.button("🖧 连接根节点").clicked() {
                                        {
                                            *self
                                                .fold_central_panel
                                                .root_node_connect_ui_is_enable
                                                .lock() = false;
                                        }
                                        {
                                            *self.root_node_connection_state.lock() =
                                                ConnectionState::Connecting;
                                        }
                                        self.connect_root_node_for_tokio();
                                    }
                                });
                            },
                        );
                        ui.add_enabled_ui(
                            {
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
            AppUILayoutState::Unfold => {
                egui::TopBottomPanel::bottom("CentralPanel-BottomPanel").show_inside(ui, |ui| {
                    ui.add_space(10.);
                    if ui
                        .add_enabled(
                            {
                                let a = self
                                    .unfold_central_panel
                                    .chat_bar
                                    .text_input_bar
                                    .is_enable
                                    .lock()
                                    .clone();
                                a
                            },
                            egui::TextEdit::multiline(
                                &mut self.unfold_central_panel.chat_bar.text_input_bar.input_text,
                            )
                            .desired_width(ui.available_width()),
                        )
                        .has_focus()
                        && ui.input(|i| i.key_pressed(egui::Key::Enter))
                    {
                        self.unfold_central_panel
                            .chat_bar
                            .message_bar
                            .msg_logs
                            .push(Message {
                                src_user_name: self.config.user_name.clone(),
                                text: self
                                    .unfold_central_panel
                                    .chat_bar
                                    .text_input_bar
                                    .input_text
                                    .trim()
                                    .to_owned(),
                            });
                        self.unfold_central_panel
                            .chat_bar
                            .text_input_bar
                            .input_text
                            .clear();
                    }
                });
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for msg in {
                                let a = self
                                    .unfold_central_panel
                                    .chat_bar
                                    .message_bar
                                    .msg_logs
                                    .clone();
                                a
                            }
                            .iter()
                            {
                                ui.horizontal(|ui| {
                                    ui.label(msg.src_user_name.clone());
                                    ui.group(|ui| {
                                        ui.add(egui::Label::new(msg.text.clone()).wrap(true));
                                    });
                                });
                            }
                        });
                });
            }
        });
    }
}
