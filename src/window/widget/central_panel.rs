use std::{borrow::Cow, net::SocketAddr};

use eframe::egui;

use crate::window::Window;

use super::{
    state_bar::{HubNodeState, Log},
    Widget,
};

#[derive(PartialEq, Clone)]
pub enum CentralPanelLayoutState {
    Readme,
    HubNode,
}

pub struct CentralPanel {
    layout_state: CentralPanelLayoutState,
    hub_node_socket_addr_str: String,
}
impl CentralPanel {
    pub fn new() -> Self {
        Self {
            layout_state: CentralPanelLayoutState::Readme,
            hub_node_socket_addr_str: "127.0.0.1:10270".to_string(),
        }
    }
    pub fn get_layout_state(&self) -> CentralPanelLayoutState {
        self.layout_state.clone()
    }
    pub fn set_layout_state(&mut self, layout_state: CentralPanelLayoutState) {
        self.layout_state = layout_state;
    }
}
impl Widget for CentralPanel {
    fn update(window: &mut Window, ui: &mut egui::Ui, _ctx: &egui::Context) {
        match window.central_panel.layout_state {
            CentralPanelLayoutState::Readme => {
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
                        if ui.link("作者主页").clicked() {
                            match opener::open("https://github.com/ZhangXiChang") {
                                Ok(_) => (),
                                Err(err) => log::warn!("打开失败，原因：{}", err),
                            }
                        }
                    });
                });
                ui.label("=====================================================================");
                ui.vertical_centered(|ui| {
                    ui.label("这里是作者玩耍的地方，✨欸嘿✨");
                });
            }
            CentralPanelLayoutState::HubNode => {
                ui.add_enabled_ui(
                    window.state_bar.get_hub_node_state() != HubNodeState::Connected,
                    |ui| {
                        ui.horizontal(|ui| {
                            ui.label("中枢节点");
                            ui.text_edit_singleline(
                                &mut window.central_panel.hub_node_socket_addr_str,
                            );
                        });
                        let hub_node_socket_addr = window
                            .central_panel
                            .hub_node_socket_addr_str
                            .parse::<SocketAddr>();
                        ui.add_enabled_ui(hub_node_socket_addr.is_ok(), |ui| {
                            if ui.button("连接").clicked() {
                                window
                                    .state_bar
                                    .set_hub_node_state(HubNodeState::Connecting);
                                tokio::spawn({
                                    let node = window.system.node.clone();
                                    let state_bar = window.state_bar.clone();
                                    async move {
                                        node.set_name("测试节点".to_string());
                                        node.set_description("测试节点的描述".to_string());
                                        match node
                                            .access_hub_node(
                                                hub_node_socket_addr.unwrap(),
                                                include_bytes!("../../../certs/root_node.cer")
                                                    .to_vec(),
                                            )
                                            .await
                                        {
                                            Ok(_) => {
                                                state_bar
                                                    .set_hub_node_state(HubNodeState::Connected);
                                                state_bar.set_log(Some(Log::Info(
                                                    "连接成功".to_string(),
                                                )));
                                            }
                                            Err(err) => {
                                                state_bar
                                                    .set_hub_node_state(HubNodeState::Disconnect);
                                                state_bar.set_log(Some(Log::Error(format!(
                                                    "连接失败！原因：{}",
                                                    err
                                                ))));
                                            }
                                        }
                                    }
                                });
                            }
                        });
                    },
                );
                ui.add_enabled_ui(
                    window.state_bar.get_hub_node_state() == HubNodeState::Connected,
                    |ui| {
                        if ui.button("断开连接").clicked() {
                            window
                                .system
                                .node
                                .close_hub_node(0, 0, "手动关闭连接".as_bytes());
                            window
                                .state_bar
                                .set_hub_node_state(HubNodeState::Disconnect);
                        }
                    },
                );
            }
        }
    }
}
