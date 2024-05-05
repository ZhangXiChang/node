use std::borrow::Cow;

use eframe::egui;
use protocol::NodeInfo;
use share_code::lock::ArcMutex;
use tokio::task::JoinHandle;

use super::state_bar::StateBar;

#[derive(Clone, PartialEq)]
pub enum FoldCentralPanelLayoutState {
    Readme,
    ConnectRootNode,
}

pub struct ConnectRootNodeBar {
    pub is_enable: ArcMutex<bool>,
    pub root_node_selected: usize,
}

pub struct FoldCentralPanel {
    pub widget_layout_state: FoldCentralPanelLayoutState,
    pub connect_root_node_bar: ConnectRootNodeBar,
}

#[derive(Clone)]
pub enum UnfoldCentralPanelLayoutState {
    NodeBrowser,
    Chat,
}

#[derive(Clone)]
pub struct NodeBrowserBar {
    pub node_info_list: ArcMutex<Vec<NodeInfo>>,
    pub row_selected_index: ArcMutex<Option<usize>>,
}

#[derive(Clone)]
pub struct Message {
    pub src_user_name: String,
    pub text: String,
}

pub struct MessageBar {
    pub msg_logs: ArcMutex<Vec<Message>>,
}

pub struct TextInputBar {
    pub input_text: ArcMutex<String>,
}

pub struct ChatBar {
    pub message_bar: MessageBar,
    pub text_input_bar: TextInputBar,
}

pub struct UnfoldCentralPanel {
    pub widget_layout_state: ArcMutex<UnfoldCentralPanelLayoutState>,
    pub node_browser_bar: NodeBrowserBar,
    pub chat_bar: ChatBar,
    pub wait_node_connect_task: Option<JoinHandle<()>>,
}

pub struct CentralPanel {
    pub fold_central_panel: FoldCentralPanel,
    pub unfold_central_panel: UnfoldCentralPanel,
}
impl CentralPanel {
    pub fn update(widget: &mut super::Widget, ctx: &egui::Context, ui: &mut egui::Ui) {
        match widget.widget_layout_state {
            super::WidgetLayoutState::Fold => match widget
                .central_panel
                .fold_central_panel
                .widget_layout_state
            {
                FoldCentralPanelLayoutState::Readme => {
                    ui.horizontal_top(|ui| {
                        ui.add(
                            egui::Image::new(egui::ImageSource::Bytes {
                                uri: Cow::default(),
                                bytes: egui::load::Bytes::Static(include_bytes!(
                                    "../../../../assets/icon/node_network_icon.png"
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
                                let a = widget
                                    .central_panel
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
                                            &mut widget
                                                .central_panel
                                                .fold_central_panel
                                                .connect_root_node_bar
                                                .root_node_selected,
                                            widget.system.root_node_info_list.len(),
                                            |i| widget.system.root_node_info_list[i].name.clone(),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("昵称");
                                        if ui
                                            .text_edit_singleline(&mut widget.system.node.user_name)
                                            .lost_focus()
                                        {
                                            if let Err(err) = widget.system.save_config() {
                                                *widget.state_bar.log.lock() =
                                                    Some(super::Log::Error(format!(
                                                        "配置保存错误！原因：{}",
                                                        err
                                                    )))
                                            }
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("自述");
                                        if ui
                                            .text_edit_multiline(&mut widget.system.node.readme)
                                            .lost_focus()
                                        {
                                            if let Err(err) = widget.system.save_config() {
                                                *widget.state_bar.log.lock() =
                                                    Some(super::Log::Error(format!(
                                                        "配置保存错误！原因：{}",
                                                        err
                                                    )))
                                            }
                                        }
                                    });
                                });
                                ui.add_enabled_ui(!widget.system.node.user_name.is_empty(), |ui| {
                                    if ui.button("🖧 连接根节点").clicked() {
                                        widget.connect_root_node();
                                    }
                                });
                            },
                        );
                        ui.add_enabled_ui(
                            match widget.system.node.root_node_is_disconnect() {
                                Ok(result) => result.is_none(),
                                Err(_) => false,
                            },
                            |ui| {
                                if ui.button("断开连接").clicked() {
                                    widget.system.node.disconnect_root_node(
                                        0,
                                        "手动关闭连接".as_bytes().to_vec(),
                                    );
                                }
                            },
                        );
                    });
                }
            },
            super::WidgetLayoutState::Unfold => match {
                let a = widget
                    .central_panel
                    .unfold_central_panel
                    .widget_layout_state
                    .lock()
                    .clone();
                a
            } {
                UnfoldCentralPanelLayoutState::NodeBrowser => {
                    egui::TopBottomPanel::bottom("CentralPanel-Unfold-NodeBrowser-BottomPanel")
                        .show_inside(ui, |ui| {
                            ui.add_space(3.);
                            ui.add_enabled_ui(
                                match widget.system.node.root_node_is_disconnect() {
                                    Ok(result) => result.is_none(),
                                    Err(_) => false,
                                },
                                |ui| {
                                    if ui.button("注册节点").clicked() {
                                        {
                                            *widget
                                                .central_panel
                                                .unfold_central_panel
                                                .widget_layout_state
                                                .lock() = UnfoldCentralPanelLayoutState::Chat;
                                        }
                                        {
                                            *widget
                                                .central_panel
                                                .unfold_central_panel
                                                .node_browser_bar
                                                .row_selected_index
                                                .lock() = None;
                                        }
                                        {
                                            *widget
                                                .central_panel
                                                .unfold_central_panel
                                                .node_browser_bar
                                                .node_info_list
                                                .lock() = Vec::new();
                                        }
                                        widget
                                            .central_panel
                                            .unfold_central_panel
                                            .wait_node_connect_task = Some(tokio::spawn({
                                            let node = widget.system.node.clone();
                                            let state_bar_log = widget.state_bar.log.clone();
                                            let message_bar_logs = widget
                                                .central_panel
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
                                                            Some(super::Log::Info(
                                                                "有人连接了欸！好耶✨".to_string(),
                                                            ));
                                                        StateBar::accept_message(
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
                                        let a = widget
                                            .central_panel
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
                                            let a = widget
                                                .central_panel
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
                                            let a = widget
                                                .central_panel
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
                                            *widget
                                                .central_panel
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
                                            widget
                                                .system
                                                .node
                                                .disconnect_node(0, "断开连接".as_bytes().to_vec());
                                            if let Some(wait_node_connect_task) = &widget
                                                .central_panel
                                                .unfold_central_panel
                                                .wait_node_connect_task
                                            {
                                                wait_node_connect_task.abort();
                                            }
                                            {
                                                *widget
                                                    .central_panel
                                                    .unfold_central_panel
                                                    .widget_layout_state
                                                    .lock() =
                                                    UnfoldCentralPanelLayoutState::NodeBrowser;
                                            }
                                            tokio::spawn({
                                                let node = widget.system.node.clone();
                                                let node_browser_bar_node_info_list = widget
                                                    .central_panel
                                                    .unfold_central_panel
                                                    .node_browser_bar
                                                    .node_info_list
                                                    .clone();
                                                let is_register_node =
                                                    widget.system.node.is_register_node.clone();
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
                                    match widget.system.node.node_is_disconnect() {
                                        Ok(result) => result.is_none(),
                                        Err(_) => false,
                                    },
                                    egui::TextEdit::multiline(
                                        &mut *widget
                                            .central_panel
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
                                    widget
                                        .central_panel
                                        .unfold_central_panel
                                        .chat_bar
                                        .message_bar
                                        .msg_logs
                                        .lock()
                                        .push(Message {
                                            src_user_name: widget.system.node.user_name.clone(),
                                            text: widget
                                                .central_panel
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
                                    let node = widget.system.node.clone();
                                    let input_text = widget
                                        .central_panel
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
                                widget
                                    .central_panel
                                    .unfold_central_panel
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
                                    let a = widget
                                        .central_panel
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
        }
        if let Some(row_selected_index) = {
            let a = widget
                .central_panel
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
                        *widget
                            .central_panel
                            .unfold_central_panel
                            .node_browser_bar
                            .row_selected_index
                            .lock() = None;
                    }
                    if ui.button("连接").clicked() {
                        tokio::spawn({
                            let mut node = widget.system.node.clone();
                            let state_bar_log = widget.state_bar.log.clone();
                            let arc_mutex_row_selected_index = widget
                                .central_panel
                                .unfold_central_panel
                                .node_browser_bar
                                .row_selected_index
                                .clone();
                            let unfold_central_panel_gui_layout_state = widget
                                .central_panel
                                .unfold_central_panel
                                .widget_layout_state
                                .clone();
                            let node_browser_bar_node_info_list = widget
                                .central_panel
                                .unfold_central_panel
                                .node_browser_bar
                                .node_info_list
                                .clone();
                            let message_bar_logs = widget
                                .central_panel
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
                                            *state_bar_log.lock() = Some(super::Log::Info(
                                                "连接节点成功了欸！好耶✨".to_string(),
                                            ));
                                        }
                                        StateBar::accept_message(node, message_bar_logs).await;
                                    }
                                    Err(_) => {
                                        *state_bar_log.lock() = Some(super::Log::Error(
                                            "连接节点失败惹！可恶💢".to_string(),
                                        ));
                                    }
                                };
                            }
                        });
                    }
                });
        }
    }
}
