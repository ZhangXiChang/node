use eframe::egui;
use tool_code_rs::lock::ArcMutex;

use crate::window::Window;

use super::Widget;

#[derive(Clone)]
pub enum HubNodeState {
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
    hub_node_state: ArcMutex<HubNodeState>,
    log: ArcMutex<Option<Log>>,
}
impl StateBar {
    pub fn new() -> Self {
        Self {
            hub_node_state: ArcMutex::new(HubNodeState::Disconnect),
            log: ArcMutex::new(None),
        }
    }
    pub fn set_root_node_state(&mut self, state: HubNodeState) {
        *self.hub_node_state.lock() = state;
    }
    pub fn set_log(&mut self, log: Option<Log>) {
        *self.log.lock() = log;
    }
}
impl Widget for StateBar {
    fn update(window: &mut Window, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.label("根节点状态:");
        match {
            let a = window.state_bar.hub_node_state.lock().clone();
            a
        } {
            HubNodeState::Connected => ui.colored_label(egui::Color32::LIGHT_GREEN, "🌏 在线"),
            HubNodeState::Disconnect => ui.colored_label(egui::Color32::LIGHT_RED, "❌ 离线"),
            HubNodeState::Connecting => ui.colored_label(egui::Color32::LIGHT_BLUE, "⏳ 连接中..."),
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
