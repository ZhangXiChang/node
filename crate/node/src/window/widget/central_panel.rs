use std::{borrow::Cow, net::SocketAddr};

use eframe::egui;

use crate::window::Window;

use super::{
    state_bar::{Log, RootNodeState},
    Widget,
};

#[derive(PartialEq)]
pub enum CentralPanelLayoutState {
    Readme,
    RootNode,
}

pub struct CentralPanel {
    pub layout_state: CentralPanelLayoutState,
    root_node_socket_addr_str: String,
}
impl CentralPanel {
    pub fn new() -> Self {
        Self {
            layout_state: CentralPanelLayoutState::Readme,
            root_node_socket_addr_str: String::new(),
        }
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
                                "../../../../../assets/icon/node_network_icon.png"
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
                            match opener::open("https://github.com/ZhangXiChang/node-network") {
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
            CentralPanelLayoutState::RootNode => {
                ui.add_enabled_ui(
                    match window.system.node.root_node_state() {
                        Ok(root_node_state) => root_node_state.is_some(),
                        Err(_) => true,
                    },
                    |ui| {
                        ui.horizontal(|ui| {
                            ui.label("根节点");
                            ui.text_edit_singleline(
                                &mut window.central_panel.root_node_socket_addr_str,
                            );
                        });
                        ui.add_enabled_ui(
                            window
                                .central_panel
                                .root_node_socket_addr_str
                                .parse::<SocketAddr>()
                                .is_ok(),
                            |ui| {
                                if ui.button("连接").clicked() {
                                    window
                                        .state_bar
                                        .set_root_node_state(RootNodeState::Connecting);
                                    tokio::spawn({
                                        let node = window.system.node.clone();
                                        let root_node_socket_addr =
                                            window.central_panel.root_node_socket_addr_str.clone();
                                        let state_bar = window.state_bar.clone();
                                        async move {
                                            match async {
                                                match node
                                                    .connect_root_node(
                                                        root_node_socket_addr.parse()?,
                                                        include_bytes!(
                                                            "../../../../../certs/root_node.cer"
                                                        )
                                                        .to_vec(),
                                                    )
                                                    .await
                                                {
                                                    Ok(_) => {
                                                        state_bar.set_root_node_state(
                                                            RootNodeState::Connected,
                                                        );
                                                        state_bar.set_log(Some(Log::Info(
                                                            "连接成功".to_string(),
                                                        )));
                                                    }
                                                    Err(err) => {
                                                        state_bar.set_root_node_state(
                                                            RootNodeState::Disconnect,
                                                        );
                                                        state_bar.set_log(Some(Log::Error(
                                                            format!("连接失败！原因：{}", err),
                                                        )));
                                                    }
                                                }
                                                node.register().await?;
                                                eyre::Ok(())
                                            }
                                            .await
                                            {
                                                Ok(_) => (),
                                                Err(err) => {
                                                    log::error!("意外的错误，原因：{}", err);
                                                }
                                            }
                                        }
                                    });
                                }
                            },
                        );
                    },
                );
                ui.add_enabled_ui(
                    match window.system.node.root_node_state() {
                        Ok(root_node_state) => root_node_state.is_none(),
                        Err(_) => false,
                    },
                    |ui| {
                        if ui.button("断开连接").clicked() {
                            window
                                .system
                                .node
                                .close_root_node_connect(0, "手动关闭连接".as_bytes().to_vec());
                            window
                                .state_bar
                                .set_root_node_state(RootNodeState::Disconnect);
                        }
                    },
                );
            }
        }
    }
}
