use std::borrow::Cow;

use eframe::egui;
use eyre::Result;
use protocol::NodeInfo;
use share_code::lock::ArcMutex;
use tokio::task::JoinHandle;

use crate::{node::Node, system::System};

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

#[derive(Clone)]
enum GUILayoutState {
    Fold,
    Unfold,
}

#[derive(Clone)]
struct GUILayoutStateSwitchButtonText(String);
impl From<GUILayoutStateSwitchButtonText> for String {
    fn from(value: GUILayoutStateSwitchButtonText) -> Self {
        let GUILayoutStateSwitchButtonText(text) = value;
        text
    }
}
impl From<GUILayoutState> for GUILayoutStateSwitchButtonText {
    fn from(value: GUILayoutState) -> Self {
        match value {
            GUILayoutState::Fold => Self("🗖 展开程序".to_string()),
            GUILayoutState::Unfold => Self("🗕 折叠程序".to_string()),
        }
    }
}

struct MenuBar {
    gui_layout_state_switch_button_text: GUILayoutStateSwitchButtonText,
}

#[derive(Clone)]
enum Log {
    Info(String),
    Error(String),
}

struct StateBar {
    log: ArcMutex<Option<Log>>,
}

#[derive(Clone, PartialEq)]
enum FoldCentralPanelLayoutState {
    Readme,
    ConnectRootNode,
}

struct ConnectRootNodeBar {
    is_enable: ArcMutex<bool>,
    root_node_selected: usize,
}

struct FoldCentralPanel {
    gui_layout_state: FoldCentralPanelLayoutState,
    connect_root_node_bar: ConnectRootNodeBar,
}

struct MessageBar {
    msg_logs: ArcMutex<Vec<Message>>,
}

struct TextInputBar {
    input_text: ArcMutex<String>,
}

#[derive(Clone)]
enum UnfoldCentralPanelLayoutState {
    NodeBrowser,
    Chat,
}

#[derive(Clone)]
struct NodeBrowserBar {
    node_info_list: ArcMutex<Vec<NodeInfo>>,
    row_selected_index: ArcMutex<Option<usize>>,
}

struct ChatBar {
    message_bar: MessageBar,
    text_input_bar: TextInputBar,
}

struct UnfoldCentralPanel {
    gui_layout_state: ArcMutex<UnfoldCentralPanelLayoutState>,
    node_browser_bar: NodeBrowserBar,
    chat_bar: ChatBar,
    wait_node_connect_task: Option<JoinHandle<()>>,
}

pub struct GUInterface {
    system: System,
    gui_layout_state: GUILayoutState,
    gui_layout_state_switch_next_window_inner_size: Option<egui::Vec2>,
    menu_bar: MenuBar,
    state_bar: StateBar,
    fold_central_panel: FoldCentralPanel,
    unfold_central_panel: UnfoldCentralPanel,
    root_node_connection_state: ArcMutex<ConnectionState>,
}
impl GUInterface {
    pub fn new(system: System) -> Result<Self> {
        let selfx = Self {
            system,
            gui_layout_state: GUILayoutState::Fold,
            gui_layout_state_switch_next_window_inner_size: Some(egui::Vec2::new(1000., 750.)),
            menu_bar: MenuBar {
                gui_layout_state_switch_button_text: GUILayoutState::Fold.into(),
            },
            state_bar: StateBar {
                log: ArcMutex::new(None),
            },
            fold_central_panel: FoldCentralPanel {
                gui_layout_state: FoldCentralPanelLayoutState::Readme,
                connect_root_node_bar: ConnectRootNodeBar {
                    is_enable: ArcMutex::new(true),
                    root_node_selected: 0,
                },
            },
            unfold_central_panel: UnfoldCentralPanel {
                gui_layout_state: ArcMutex::new(UnfoldCentralPanelLayoutState::NodeBrowser),
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
            root_node_connection_state: ArcMutex::new(ConnectionState::Disconnect),
        };
        if !selfx.system.node.user_name.is_empty() {
            selfx.connect_root_node();
        }
        Ok(selfx)
    }
    fn gui_layout_state_switch(&mut self, ctx: &egui::Context, gui_layout_state: GUILayoutState) {
        if let Some(gui_layout_state_switch_next_window_inner_size) =
            self.gui_layout_state_switch_next_window_inner_size
        {
            self.gui_layout_state = gui_layout_state;
            self.menu_bar.gui_layout_state_switch_button_text =
                self.gui_layout_state.clone().into();
            self.gui_layout_state_switch_next_window_inner_size =
                ctx.input(|i| i.viewport().inner_rect).map(|v| v.size());
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(
                gui_layout_state_switch_next_window_inner_size,
            ));
            match self.gui_layout_state {
                GUILayoutState::Fold => {
                    *self
                        .unfold_central_panel
                        .node_browser_bar
                        .node_info_list
                        .lock() = Vec::new();
                }
                GUILayoutState::Unfold => match {
                    let a = self.unfold_central_panel.gui_layout_state.lock().clone();
                    a
                } {
                    UnfoldCentralPanelLayoutState::NodeBrowser => {
                        tokio::spawn({
                            let node = self.system.node.clone();
                            let node_browser_bar_node_info_list = self
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
                .fold_central_panel
                .connect_root_node_bar
                .root_node_selected]
                .socket_addr;
            let root_node_dns_name = self.system.root_node_info_list[self
                .fold_central_panel
                .connect_root_node_bar
                .root_node_selected]
                .dns_name
                .clone();
            let root_node_connection_state = self.root_node_connection_state.clone();
            let state_bar_log = self.state_bar.log.clone();
            let node_browser_bar_node_info_list = self
                .unfold_central_panel
                .node_browser_bar
                .node_info_list
                .clone();
            let root_node_connect_ui_is_enable = self
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
    async fn accept_message(node: Node, message_bar_logs: ArcMutex<Vec<Message>>) {
        loop {
            match node.accept_uni().await {
                Ok(mut recv) => match recv.read_to_end(usize::MAX).await {
                    Ok(data) => match String::from_utf8(data) {
                        Ok(text) => message_bar_logs.lock().push(Message {
                            src_user_name: node.user_name.clone(),
                            text,
                        }),
                        Err(err) => {
                            log::error!("{}", err);
                            break;
                        }
                    },
                    Err(err) => {
                        log::error!("{}", err);
                        break;
                    }
                },
                Err(err) => {
                    log::error!("{}", err);
                    break;
                }
            }
        }
    }
}
impl Drop for GUInterface {
    fn drop(&mut self) {
        self.system.node.close(0, "程序关闭".as_bytes().to_vec());
    }
}
impl eframe::App for GUInterface {
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
                            self.menu_bar.gui_layout_state_switch_button_text.clone(),
                        ))
                        .clicked()
                    {
                        match self.gui_layout_state {
                            GUILayoutState::Fold => {
                                self.gui_layout_state_switch(ctx, GUILayoutState::Unfold);
                            }
                            GUILayoutState::Unfold => {
                                self.gui_layout_state_switch(ctx, GUILayoutState::Fold);
                            }
                        }
                    }
                    match self.gui_layout_state {
                        GUILayoutState::Fold => {
                            ui.menu_button("切换视图", |ui| {
                                ui.radio_value(
                                    &mut self.fold_central_panel.gui_layout_state,
                                    FoldCentralPanelLayoutState::Readme,
                                    "软件自述视图",
                                );
                                ui.radio_value(
                                    &mut self.fold_central_panel.gui_layout_state,
                                    FoldCentralPanelLayoutState::ConnectRootNode,
                                    "连接根节点视图",
                                );
                            });
                        }
                        GUILayoutState::Unfold => (),
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
                        ui.colored_label(egui::Color32::LIGHT_GREEN, "🌏 在线")
                    }
                    ConnectionState::Disconnect => {
                        ui.colored_label(egui::Color32::LIGHT_RED, "❌ 离线")
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
        egui::CentralPanel::default().show(ctx, |ui| match self.gui_layout_state {
            GUILayoutState::Fold => match self.fold_central_panel.gui_layout_state {
                FoldCentralPanelLayoutState::Readme => {
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
                FoldCentralPanelLayoutState::ConnectRootNode => {
                    ui.vertical_centered(|ui| {
                        ui.add_space((ui.available_height() / 2.) - 70.);
                        ui.add_enabled_ui(
                            {
                                let a = self
                                    .fold_central_panel
                                    .connect_root_node_bar
                                    .is_enable
                                    .lock()
                                    .clone();
                                a
                            },
                            |ui| {
                                ui.allocate_ui(egui::Vec2::new(200., 0.), |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label("根节点");
                                        egui::ComboBox::from_id_source(
                                            "CentralPanel-RootNodeSelectsItem",
                                        )
                                        .width(ui.available_width())
                                        .show_index(
                                            ui,
                                            &mut self
                                                .fold_central_panel
                                                .connect_root_node_bar
                                                .root_node_selected,
                                            self.system.root_node_info_list.len(),
                                            |i| self.system.root_node_info_list[i].name.clone(),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("昵称");
                                        if ui
                                            .text_edit_singleline(&mut self.system.node.user_name)
                                            .lost_focus()
                                        {
                                            if let Err(err) = self.system.save_config() {
                                                *self.state_bar.log.lock() = Some(Log::Error(
                                                    format!("配置保存错误！原因：{}", err),
                                                ))
                                            }
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("自述");
                                        if ui
                                            .text_edit_multiline(&mut self.system.node.readme)
                                            .lost_focus()
                                        {
                                            if let Err(err) = self.system.save_config() {
                                                *self.state_bar.log.lock() = Some(Log::Error(
                                                    format!("配置保存错误！原因：{}", err),
                                                ))
                                            }
                                        }
                                    });
                                });
                                ui.add_enabled_ui(!self.system.node.user_name.is_empty(), |ui| {
                                    if ui.button("🖧 连接根节点").clicked() {
                                        self.connect_root_node();
                                    }
                                });
                            },
                        );
                        ui.add_enabled_ui(
                            match self.system.node.root_node_is_disconnect() {
                                Ok(result) => result.is_none(),
                                Err(_) => false,
                            },
                            |ui| {
                                if ui.button("断开连接").clicked() {
                                    self.system.node.disconnect_root_node(
                                        0,
                                        "手动关闭连接".as_bytes().to_vec(),
                                    );
                                }
                            },
                        );
                    });
                }
            },
            GUILayoutState::Unfold => match {
                let a = self.unfold_central_panel.gui_layout_state.lock().clone();
                a
            } {
                UnfoldCentralPanelLayoutState::NodeBrowser => {
                    egui::TopBottomPanel::bottom("CentralPanel-Unfold-NodeBrowser-BottomPanel")
                        .show_inside(ui, |ui| {
                            ui.add_space(3.);
                            ui.add_enabled_ui(
                                match self.system.node.root_node_is_disconnect() {
                                    Ok(result) => result.is_none(),
                                    Err(_) => false,
                                },
                                |ui| {
                                    if ui.button("注册节点").clicked() {
                                        {
                                            *self.unfold_central_panel.gui_layout_state.lock() =
                                                UnfoldCentralPanelLayoutState::Chat;
                                        }
                                        {
                                            *self
                                                .unfold_central_panel
                                                .node_browser_bar
                                                .row_selected_index
                                                .lock() = None;
                                        }
                                        {
                                            *self
                                                .unfold_central_panel
                                                .node_browser_bar
                                                .node_info_list
                                                .lock() = Vec::new();
                                        }
                                        self.unfold_central_panel.wait_node_connect_task =
                                            Some(tokio::spawn({
                                                let node = self.system.node.clone();
                                                let state_bar_log = self.state_bar.log.clone();
                                                let message_bar_logs = self
                                                    .unfold_central_panel
                                                    .chat_bar
                                                    .message_bar
                                                    .msg_logs
                                                    .clone();
                                                async move {
                                                    match node.register_node().await {
                                                        Ok(_) => (),
                                                        Err(err) => {
                                                            log::error!("{}", err);
                                                            return;
                                                        }
                                                    }
                                                    match node.accept().await {
                                                        Ok(_) => {
                                                            *state_bar_log.lock() =
                                                                Some(Log::Info(
                                                                    "有人连接了欸！好耶✨"
                                                                        .to_string(),
                                                                ));
                                                            Self::accept_message(
                                                                node,
                                                                message_bar_logs,
                                                            )
                                                            .await;
                                                        }
                                                        Err(err) => {
                                                            match node.unregister_node().await {
                                                                Ok(_) => {
                                                                    log::error!("{}", err);
                                                                }
                                                                Err(err) => {
                                                                    log::error!("{}", err);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }));
                                    }
                                },
                            );
                        });
                    egui::CentralPanel::default().show_inside(ui, |ui| {
                        egui_extras::TableBuilder::new(ui)
                            .striped(true)
                            .resizable(true)
                            .sense(egui::Sense::click())
                            .column(egui_extras::Column::exact(125.))
                            .column(egui_extras::Column::exact(275.))
                            .column(egui_extras::Column::remainder())
                            .header(20., |mut header| {
                                header.col(|ui| {
                                    ui.vertical_centered(|ui| {
                                        ui.heading("用户名");
                                    });
                                });
                                header.col(|ui| {
                                    ui.vertical_centered(|ui| {
                                        ui.heading("UUID");
                                    });
                                });
                                header.col(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.add_space(30.);
                                        ui.heading("自述");
                                    });
                                });
                            })
                            .body(|body| {
                                body.rows(
                                    20.,
                                    {
                                        let a = self
                                            .unfold_central_panel
                                            .node_browser_bar
                                            .node_info_list
                                            .lock()
                                            .len();
                                        a
                                    },
                                    |mut row| {
                                        //点击选中突出显示
                                        if let Some(row_selected_index) = {
                                            let a = self
                                                .unfold_central_panel
                                                .node_browser_bar
                                                .row_selected_index
                                                .lock()
                                                .clone();
                                            a
                                        } {
                                            row.set_selected(row.index() == row_selected_index);
                                        }
                                        //绘制字段
                                        let node_info = {
                                            let a = self
                                                .unfold_central_panel
                                                .node_browser_bar
                                                .node_info_list
                                                .lock()[row.index()]
                                            .clone();
                                            a
                                        };
                                        row.col(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.add_space(10.);
                                                ui.add(
                                                    egui::Label::new(node_info.user_name)
                                                        .wrap(false),
                                                );
                                            });
                                        });
                                        row.col(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.add_space(10.);
                                                ui.add(
                                                    egui::Label::new(node_info.uuid).wrap(false),
                                                );
                                            });
                                        });
                                        row.col(|ui| {
                                            ui.horizontal(|ui| {
                                                ui.add_space(10.);
                                                ui.add(
                                                    egui::Label::new(node_info.readme).wrap(false),
                                                );
                                            });
                                        });
                                        //点击选中
                                        if row.response().clicked() {
                                            *self
                                                .unfold_central_panel
                                                .node_browser_bar
                                                .row_selected_index
                                                .lock() = Some(row.index());
                                        }
                                    },
                                );
                            });
                    });
                }
                UnfoldCentralPanelLayoutState::Chat => {
                    egui::TopBottomPanel::bottom("CentralPanel-Fold-Chat-BottomPanel").show_inside(
                        ui,
                        |ui| {
                            ui.horizontal(|ui| {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button("断开连接").clicked() {
                                            self.system
                                                .node
                                                .disconnect_node(0, "断开连接".as_bytes().to_vec());
                                            if let Some(wait_node_connect_task) =
                                                &self.unfold_central_panel.wait_node_connect_task
                                            {
                                                wait_node_connect_task.abort();
                                            }
                                            {
                                                *self
                                                    .unfold_central_panel
                                                    .gui_layout_state
                                                    .lock() =
                                                    UnfoldCentralPanelLayoutState::NodeBrowser;
                                            }
                                            tokio::spawn({
                                                let node = self.system.node.clone();
                                                let node_browser_bar_node_info_list = self
                                                    .unfold_central_panel
                                                    .node_browser_bar
                                                    .node_info_list
                                                    .clone();
                                                let is_register_node =
                                                    self.system.node.is_register_node.clone();
                                                async move {
                                                    if {
                                                        let a = is_register_node.lock().clone();
                                                        a
                                                    } {
                                                        match node.unregister_node().await {
                                                            Ok(_) => (),
                                                            Err(err) => log::error!("{}", err),
                                                        }
                                                    }
                                                    match node
                                                        .request_register_node_info_list()
                                                        .await
                                                    {
                                                        Ok(node_info_list) => {
                                                            *node_browser_bar_node_info_list
                                                                .lock() = node_info_list
                                                        }
                                                        Err(err) => log::error!("{}", err),
                                                    }
                                                }
                                            });
                                        }
                                    },
                                );
                            });
                            if ui
                                .add_enabled(
                                    match self.system.node.node_is_disconnect() {
                                        Ok(result) => result.is_none(),
                                        Err(_) => false,
                                    },
                                    egui::TextEdit::multiline(
                                        &mut *self
                                            .unfold_central_panel
                                            .chat_bar
                                            .text_input_bar
                                            .input_text
                                            .lock(),
                                    )
                                    .desired_width(ui.available_width()),
                                )
                                .has_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            {
                                {
                                    self.unfold_central_panel
                                        .chat_bar
                                        .message_bar
                                        .msg_logs
                                        .lock()
                                        .push(Message {
                                            src_user_name: self.system.node.user_name.clone(),
                                            text: self
                                                .unfold_central_panel
                                                .chat_bar
                                                .text_input_bar
                                                .input_text
                                                .lock()
                                                .trim()
                                                .to_string(),
                                        });
                                }
                                tokio::spawn({
                                    let node = self.system.node.clone();
                                    let input_text = self
                                        .unfold_central_panel
                                        .chat_bar
                                        .text_input_bar
                                        .input_text
                                        .clone();
                                    async move {
                                        match node.open_uni().await {
                                            Ok(mut send) => {
                                                match send
                                                    .write_all(&{
                                                        let a = input_text
                                                            .lock()
                                                            .trim()
                                                            .as_bytes()
                                                            .to_vec();
                                                        a
                                                    })
                                                    .await
                                                {
                                                    Ok(_) => (),
                                                    Err(err) => log::error!("{}", err),
                                                }
                                                match send.finish().await {
                                                    Ok(_) => (),
                                                    Err(err) => log::error!("{}", err),
                                                }
                                            }
                                            Err(err) => log::error!("{}", err),
                                        }
                                    }
                                });
                                self.unfold_central_panel
                                    .chat_bar
                                    .text_input_bar
                                    .input_text
                                    .lock()
                                    .clear();
                            }
                        },
                    );
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
                                        .lock()
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
            },
        });
        if let Some(row_selected_index) = {
            let a = self
                .unfold_central_panel
                .node_browser_bar
                .row_selected_index
                .lock()
                .clone();
            a
        } {
            egui::SidePanel::right("RightSideBar-Cover")
                .min_width(300.)
                .show(ctx, |ui| {
                    if ui.button("关闭").clicked() {
                        *self
                            .unfold_central_panel
                            .node_browser_bar
                            .row_selected_index
                            .lock() = None;
                    }
                    if ui.button("连接").clicked() {
                        tokio::spawn({
                            let mut node = self.system.node.clone();
                            let state_bar_log = self.state_bar.log.clone();
                            let arc_mutex_row_selected_index = self
                                .unfold_central_panel
                                .node_browser_bar
                                .row_selected_index
                                .clone();
                            let unfold_central_panel_gui_layout_state =
                                self.unfold_central_panel.gui_layout_state.clone();
                            let node_browser_bar_node_info_list = self
                                .unfold_central_panel
                                .node_browser_bar
                                .node_info_list
                                .clone();
                            let message_bar_logs = self
                                .unfold_central_panel
                                .chat_bar
                                .message_bar
                                .msg_logs
                                .clone();
                            async move {
                                //连接节点
                                match node
                                    .connect_node({
                                        let a = node_browser_bar_node_info_list.lock()
                                            [row_selected_index]
                                            .uuid
                                            .clone();
                                        a
                                    })
                                    .await
                                {
                                    Ok(_) => {
                                        {
                                            *unfold_central_panel_gui_layout_state.lock() =
                                                UnfoldCentralPanelLayoutState::Chat;
                                        }
                                        {
                                            *arc_mutex_row_selected_index.lock() = None;
                                        }
                                        {
                                            *node_browser_bar_node_info_list.lock() = Vec::new();
                                        }
                                        {
                                            *state_bar_log.lock() = Some(Log::Info(
                                                "连接节点成功了欸！好耶✨".to_string(),
                                            ));
                                        }
                                        Self::accept_message(node, message_bar_logs).await;
                                    }
                                    Err(_) => {
                                        *state_bar_log.lock() =
                                            Some(Log::Error("连接节点失败惹！可恶💢".to_string()));
                                    }
                                };
                            }
                        });
                    }
                });
        }
    }
}
