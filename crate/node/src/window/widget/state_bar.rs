use eframe::egui;
use share_code::lock::ArcMutex;

use crate::window::Window;

use super::Widget;

#[derive(Clone)]
pub enum RootNodeState {
    Connected,
    Disconnect,
    Connecting,
}

#[derive(Clone)]
pub enum Log {
    Info(String),
    Error(String),
}

#[derive(Clone)]
pub struct StateBar {
    root_node_state: ArcMutex<RootNodeState>,
    log: ArcMutex<Option<Log>>,
}
impl StateBar {
    pub fn new() -> Self {
        Self {
            root_node_state: ArcMutex::new(RootNodeState::Disconnect),
            log: ArcMutex::new(None),
        }
    }
    pub fn set_root_node_state(&self, state: RootNodeState) {
        *self.root_node_state.lock() = state;
    }
    pub fn set_log(&self, log: Option<Log>) {
        *self.log.lock() = log;
    }
}
impl Widget for StateBar {
    fn update(window: &mut Window, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.label("根节点状态:");
        match {
            let a = window.state_bar.root_node_state.lock().clone();
            a
        } {
            RootNodeState::Connected => ui.colored_label(egui::Color32::LIGHT_GREEN, "🌏 在线"),
            RootNodeState::Disconnect => ui.colored_label(egui::Color32::LIGHT_RED, "❌ 离线"),
            RootNodeState::Connecting => {
                ui.colored_label(egui::Color32::LIGHT_BLUE, "⏳ 连接中...")
            }
        };
        ui.label("|");
        if let Some(msg) = {
            let a = window.state_bar.log.lock().clone();
            a
        } {
            match msg {
                Log::Info(text) => ui.colored_label(egui::Color32::GRAY, text),
                Log::Error(text) => ui.colored_label(egui::Color32::LIGHT_RED, text),
            };
        }
    }
}
