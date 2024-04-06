use std::{borrow::Cow, sync::Arc, time::Duration};

use eyre::{eyre, Result};

use eframe::egui;
use quinn::{ClientConfig, Endpoint, ServerConfig, TransportConfig};
use uuid::Uuid;

const ICON: &[u8] = include_bytes!("../../../assets/icon/node_network_icon.png");
const ICON_WIDTH: u32 = 512;
const ICON_HEIGHT: u32 = 512;

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
    root_node_connection_state: ConnectionState,
    state_bar_message: Option<Message>,
    ui_mode: UIMode,
    ui_mode_switch_button_text: UIModeSwitchButtonText,
    ui_mode_switch_inner_size: Option<egui::Vec2>,
    side_bar_is_show: bool,
    endpoint: Endpoint,
}
impl App {
    fn new() -> Result<Self> {
        //创建节点
        let rcgen::CertifiedKey { cert, key_pair } =
            rcgen::generate_simple_self_signed(vec![Uuid::new_v4().to_string()])?;
        let mut transport_config = TransportConfig::default();
        transport_config.keep_alive_interval(Some(Duration::from_secs(5)));
        let endpoint = Endpoint::server(
            ServerConfig::with_single_cert(
                vec![rustls::Certificate(cert.der().to_vec())],
                rustls::PrivateKey(key_pair.serialize_der()),
            )?
            .transport_config(Arc::new(transport_config))
            .clone(),
            "0.0.0.0:0".parse()?,
        )?;
        Ok(Self {
            root_node_connection_state: ConnectionState::Disconnect,
            state_bar_message: None,
            ui_mode: UIMode::Fold,
            ui_mode_switch_button_text: UIMode::Fold.into(),
            ui_mode_switch_inner_size: Some(egui::Vec2::new(1150., 750.)),
            side_bar_is_show: false,
            endpoint,
        })
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
                            let _ = opener::open("https://github.com/ZhangXiChang/node-network");
                        }
                    });
                });
            });
        });
        egui::TopBottomPanel::bottom("StateBar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("🖧 连接根节点").clicked() {
                    //TODO 连接根节点实现
                    tokio::spawn({
                        //let endpoint = self.endpoint.clone();
                        async {
                            //endpoint.connect_with(config, addr, server_name);
                            eyre::Ok(())
                        }
                    });
                }
                match self.root_node_connection_state {
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
                if let Some(msg) = self.state_bar_message.clone() {
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
            match App::new() {
                Ok(app) => Box::new(app),
                Err(err) => panic!("{}", err),
            }
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
