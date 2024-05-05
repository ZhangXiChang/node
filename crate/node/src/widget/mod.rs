mod central_panel;
mod menu_bar;
mod state_bar;

use eframe::egui;
use eyre::Result;
use share_code::lock::ArcMutex;

use crate::system::System;

use self::{
    central_panel::{
        CentralPanel, ChatBar, ConnectRootNodeBar, FoldCentralPanel, FoldCentralPanelLayoutState,
        MessageBar, NodeBrowserBar, TextInputBar, UnfoldCentralPanel,
        UnfoldCentralPanelLayoutState,
    },
    menu_bar::MenuBar,
    state_bar::StateBar,
};

#[derive(Clone)]
enum ConnectionState {
    Connected,
    Disconnect,
    Connecting,
}

#[derive(Clone)]
enum Log {
    Info(String),
    Error(String),
}

#[derive(Clone)]
enum WidgetLayoutState {
    Fold,
    Unfold,
}

pub struct Widget {
    menu_bar: MenuBar,
    state_bar: StateBar,
    central_panel: CentralPanel,
    system: System,
    widget_layout_state: WidgetLayoutState,
    widget_layout_state_switch_next_window_inner_size: Option<egui::Vec2>,
    root_node_connection_state: ArcMutex<ConnectionState>,
}
impl Widget {
    pub fn new(system: System) -> Result<Self> {
        let selfx = Self {
            menu_bar: MenuBar {
                widget_layout_state_switch_button_text: WidgetLayoutState::Fold.into(),
            },
            state_bar: StateBar {
                log: ArcMutex::new(None),
            },
            central_panel: CentralPanel {
                fold_central_panel: FoldCentralPanel {
                    widget_layout_state: FoldCentralPanelLayoutState::Readme,
                    connect_root_node_bar: ConnectRootNodeBar {
                        is_enable: ArcMutex::new(true),
                        root_node_selected: 0,
                    },
                },
                unfold_central_panel: UnfoldCentralPanel {
                    widget_layout_state: ArcMutex::new(UnfoldCentralPanelLayoutState::NodeBrowser),
                    node_browser_bar: NodeBrowserBar {
                        node_info_list: ArcMutex::new(Vec::new()),
                        row_selected_index: ArcMutex::new(None),
                    },
                    chat_bar: ChatBar {
                        message_bar: MessageBar {
                            msg_logs: ArcMutex::new(Vec::new()),
                        },
                        text_input_bar: TextInputBar {
                            input_text: ArcMutex::new(String::new()),
                        },
                    },
                    wait_node_connect_task: None,
                },
            },
            system,
            widget_layout_state: WidgetLayoutState::Fold,
            widget_layout_state_switch_next_window_inner_size: Some(egui::Vec2::new(1000., 750.)),
            root_node_connection_state: ArcMutex::new(ConnectionState::Disconnect),
        };
        if !selfx.system.node.user_name.is_empty() {
            selfx.connect_root_node();
        }
        Ok(selfx)
    }
    fn widget_layout_state_switch(
        &mut self,
        ctx: &egui::Context,
        widget_layout_state: WidgetLayoutState,
    ) {
        if let Some(gui_layout_state_switch_next_window_inner_size) =
            self.widget_layout_state_switch_next_window_inner_size
        {
            self.widget_layout_state = widget_layout_state;
            self.menu_bar.widget_layout_state_switch_button_text =
                self.widget_layout_state.clone().into();
            self.widget_layout_state_switch_next_window_inner_size =
                ctx.input(|i| i.viewport().inner_rect).map(|v| v.size());
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                gui_layout_state_switch_next_window_inner_size,
            ));
            match self.widget_layout_state {
                WidgetLayoutState::Fold => {
                    *self
                        .central_panel
                        .unfold_central_panel
                        .node_browser_bar
                        .node_info_list
                        .lock() = Vec::new();
                }
                WidgetLayoutState::Unfold => match {
                    let a = self
                        .central_panel
                        .unfold_central_panel
                        .widget_layout_state
                        .lock()
                        .clone();
                    a
                } {
                    UnfoldCentralPanelLayoutState::NodeBrowser => {
                        tokio::spawn({
                            let node = self.system.node.clone();
                            let node_browser_bar_node_info_list = self
                                .central_panel
                                .unfold_central_panel
                                .node_browser_bar
                                .node_info_list
                                .clone();
                            async move {
                                match node.request_register_node_info_list().await {
                                    Ok(node_info_list) => {
                                        *node_browser_bar_node_info_list.lock() = node_info_list
                                    }
                                    Err(err) => log::error!("{}", err),
                                }
                            }
                        });
                    }
                    UnfoldCentralPanelLayoutState::Chat => (),
                },
            }
        }
    }
    fn connect_root_node(&self) {
        {
            *self
                .central_panel
                .fold_central_panel
                .connect_root_node_bar
                .is_enable
                .lock() = false;
        }
        {
            *self.root_node_connection_state.lock() = ConnectionState::Connecting;
        }
        tokio::spawn({
            let node = self.system.node.clone();
            let root_node_socket_addr = self.system.root_node_info_list[self
                .central_panel
                .fold_central_panel
                .connect_root_node_bar
                .root_node_selected]
                .socket_addr;
            let root_node_dns_name = self.system.root_node_info_list[self
                .central_panel
                .fold_central_panel
                .connect_root_node_bar
                .root_node_selected]
                .dns_name
                .clone();
            let root_node_connection_state = self.root_node_connection_state.clone();
            let state_bar_log = self.state_bar.log.clone();
            let node_browser_bar_node_info_list = self
                .central_panel
                .unfold_central_panel
                .node_browser_bar
                .node_info_list
                .clone();
            let root_node_connect_ui_is_enable = self
                .central_panel
                .fold_central_panel
                .connect_root_node_bar
                .is_enable
                .clone();
            async move {
                match node
                    .connect_root_node(root_node_socket_addr, root_node_dns_name)
                    .await
                {
                    Ok(_) => {
                        {
                            *root_node_connection_state.lock() = ConnectionState::Connected;
                        }
                        {
                            *state_bar_log.lock() =
                                Some(Log::Info("连接根节点成功啦~✨，快去玩耍吧~".to_string()));
                        }
                        //等待根节点断开
                        tokio::spawn({
                            let node = node.clone();
                            async move {
                                if let Err(err) = node.wait_root_node_disconnect().await {
                                    log::error!("{}", err);
                                }
                                {
                                    *root_node_connection_state.lock() =
                                        ConnectionState::Disconnect;
                                }
                                {
                                    *node_browser_bar_node_info_list.lock() = Vec::new();
                                }
                                {
                                    *root_node_connect_ui_is_enable.lock() = true;
                                }
                                {
                                    *state_bar_log.lock() = Some(Log::Info(
                                        "根节点断开连接惹！不要离开我呀~😭".to_string(),
                                    ));
                                }
                            }
                        });
                    }
                    Err(_) => {
                        {
                            *root_node_connection_state.lock() = ConnectionState::Disconnect;
                        }
                        {
                            *root_node_connect_ui_is_enable.lock() = true;
                        }
                        {
                            *state_bar_log.lock() =
                                Some(Log::Error("连接根节点失败惹！可恶💢".to_string()));
                        }
                    }
                }
            }
        });
    }
}
impl Drop for Widget {
    fn drop(&mut self) {
        self.system.node.close(0, "程序关闭".as_bytes().to_vec());
    }
}
impl eframe::App for Widget {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("MenuBar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                MenuBar::update(self, ctx, ui);
            });
        });
        egui::TopBottomPanel::bottom("StateBar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                StateBar::update(self, ui);
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            CentralPanel::update(self, ctx, ui);
        });
    }
}
