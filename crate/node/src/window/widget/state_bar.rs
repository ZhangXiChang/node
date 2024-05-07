use eframe::egui;

use crate::window::Window;

use super::Widget;

enum ConnectionState {
    Connected,
    Disconnect,
    Connecting,
}

enum Log {
    Info(String),
    Error(String),
}

pub struct StateBar {
    connection_state: ConnectionState,
    log: Option<Log>,
}
impl StateBar {
    pub fn new() -> Self {
        Self {
            connection_state: ConnectionState::Disconnect,
            log: None,
        }
    }
}
impl Widget for StateBar {
    fn update(window: &mut Window, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.label("根节点状态:");
        match window.state_bar.connection_state {
            ConnectionState::Connected => ui.colored_label(egui::Color32::LIGHT_GREEN, "🌏 在线"),
            ConnectionState::Disconnect => ui.colored_label(egui::Color32::LIGHT_RED, "❌ 离线"),
            ConnectionState::Connecting => {
                ui.colored_label(egui::Color32::LIGHT_BLUE, "⏳ 连接中...")
            }
        };
        ui.label("|");
        if let Some(msg) = &window.state_bar.log {
            match msg {
                Log::Info(text) => ui.colored_label(egui::Color32::GRAY, text),
                Log::Error(text) => ui.colored_label(egui::Color32::LIGHT_RED, text),
            };
        }
    }
}
