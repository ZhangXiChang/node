use eframe::egui;

use crate::window::Window;

use super::Widget;

#[derive(Clone, PartialEq)]
pub enum HubNodeState {
    Connected,
    Disconnect,
    Connecting,
}

#[derive(Clone)]
pub enum Message {
    Info(String),
    Error(String),
}

#[derive(Clone)]
pub struct StateBar {
    state_bar_msg: Option<Message>,
    hub_node_state: HubNodeState,
    hub_node_delay: i32,
}
impl StateBar {
    pub fn new() -> Self {
        Self {
            state_bar_msg: None,
            hub_node_state: HubNodeState::Disconnect,
            hub_node_delay: -1,
        }
    }
    pub fn set_msg(&mut self, msg: Option<Message>) {
        self.state_bar_msg = msg;
    }
    pub fn set_hub_node_state(&mut self, state: HubNodeState) {
        self.hub_node_state = state;
    }
    pub fn get_hub_node_state(&self) -> &HubNodeState {
        &self.hub_node_state
    }
    pub fn set_hub_node_delay(&mut self, delay: i32) {
        self.hub_node_delay = delay;
    }
}
impl Widget for StateBar {
    fn update(window: &mut Window, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.label("根节点状态:");
        match window.state_bar.hub_node_state {
            HubNodeState::Connected => ui.colored_label(egui::Color32::LIGHT_GREEN, "🌏 在线"),
            HubNodeState::Disconnect => ui.colored_label(egui::Color32::LIGHT_RED, "❌ 离线"),
            HubNodeState::Connecting => ui.colored_label(egui::Color32::LIGHT_BLUE, "⏳ 连接中..."),
        };
        ui.label("|");
        ui.label("延迟:");
        if window.state_bar.hub_node_delay == -1 {
            ui.colored_label(
                egui::Color32::LIGHT_RED,
                window.state_bar.hub_node_delay.to_string(),
            );
        } else if window.state_bar.hub_node_delay < 100 {
            ui.colored_label(
                egui::Color32::LIGHT_GREEN,
                window.state_bar.hub_node_delay.to_string(),
            );
        } else {
            ui.colored_label(
                egui::Color32::from_rgb(230, 230, 0),
                window.state_bar.hub_node_delay.to_string(),
            );
        }
        ui.label("|");
        if let Some(msg) = &window.state_bar.state_bar_msg {
            match msg {
                Message::Info(text) => ui.colored_label(egui::Color32::GRAY, text),
                Message::Error(text) => ui.colored_label(egui::Color32::LIGHT_RED, text),
            };
        }
    }
}
