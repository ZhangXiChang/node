use std::{borrow::Cow, sync::Arc};

use eyre::{eyre, Result};

use eframe::egui;
use log::LevelFilter;

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
            UIMode::Unfold => Self("展开程序".to_owned()),
            UIMode::Fold => Self("折叠程序".to_owned()),
        }
    }
}

struct App {
    root_node_connection_state: ConnectionState,
    state_bar_message: Message,
    ui_mode: UIMode,
    ui_mode_switch_button_text: UIModeSwitchButtonText,
}
impl App {
    fn new() -> Self {
        Self {
            root_node_connection_state: ConnectionState::Disconnect,
            state_bar_message: Message::Info(String::new()),
            ui_mode: UIMode::Unfold,
            ui_mode_switch_button_text: UIMode::Unfold.into(),
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
                        }
                        UIMode::Fold => {
                            self.ui_mode = UIMode::Unfold;
                            self.ui_mode_switch_button_text = self.ui_mode.clone().into();
                        }
                    }
                }
                ui.menu_button("关于", |ui| {
                    ui.label("作者：张喜昌");
                    if ui.button("源代码").clicked() {
                        let _ = opener::open("https://github.com/ZhangXiChang/node-network");
                    }
                });
            });
        });
        egui::TopBottomPanel::bottom("StateBar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("连接根节点").clicked() {
                    self.state_bar_message = Message::Error("根节点连接失败！".to_owned());
                }
                match self.root_node_connection_state {
                    ConnectionState::Connected => {
                        ui.colored_label(egui::Color32::LIGHT_GREEN, "根节点已连接")
                    }
                    ConnectionState::Disconnect => {
                        ui.colored_label(egui::Color32::LIGHT_RED, "根节点未连接")
                    }
                    ConnectionState::Connecting => {
                        ui.colored_label(egui::Color32::LIGHT_BLUE, "根节点连接中...")
                    }
                };
                ui.label("|");
                match self.state_bar_message.clone() {
                    Message::Info(msg) => ui.colored_label(egui::Color32::LIGHT_GRAY, msg),
                    Message::Error(msg) => ui.colored_label(egui::Color32::LIGHT_RED, msg),
                };
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(
                egui::Image::new(egui::ImageSource::Bytes {
                    uri: Cow::default(),
                    bytes: egui::load::Bytes::Static(ICON),
                })
                .max_size(egui::Vec2::new(ICON_WIDTH as f32, ICON_HEIGHT as f32)),
            )
        });
    }
}

fn main() -> Result<()> {
    //初始化日志消费者
    env_logger::builder().filter_level(LevelFilter::Info).init();
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
