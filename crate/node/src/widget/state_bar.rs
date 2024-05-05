use eframe::egui;
use share_code::lock::ArcMutex;

use crate::node::Node;

use super::central_panel::Message;

pub struct StateBar {
    pub log: ArcMutex<Option<super::Log>>,
}
impl StateBar {
    pub async fn accept_message(node: Node, message_bar_logs: ArcMutex<Vec<Message>>) {
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
impl StateBar {
    pub fn update(widget: &mut super::Widget, ui: &mut egui::Ui) {
        ui.label("根节点状态:");
        match {
            let a = widget.root_node_connection_state.lock().clone();
            a
        } {
            super::ConnectionState::Connected => {
                ui.colored_label(egui::Color32::LIGHT_GREEN, "🌏 在线")
            }
            super::ConnectionState::Disconnect => {
                ui.colored_label(egui::Color32::LIGHT_RED, "❌ 离线")
            }
            super::ConnectionState::Connecting => {
                ui.colored_label(egui::Color32::LIGHT_BLUE, "⏳ 连接中...")
            }
        };
        ui.label("|");
        if let Some(msg) = {
            let a = widget.state_bar.log.lock().clone();
            a
        } {
            match msg {
                super::Log::Info(text) => ui.colored_label(egui::Color32::GRAY, text),
                super::Log::Error(text) => ui.colored_label(egui::Color32::LIGHT_RED, text),
            };
        }
    }
}
