use eframe::egui;
use eyre::Result;

use crate::{window::Window, ICON_FILE_DATA, ICON_HEIGHT, ICON_WIDTH};

use super::Widget;

pub struct CentralPanel {
    hub_node_socket_addr: String,
}
impl CentralPanel {
    pub fn new() -> Result<Self> {
        Ok(Self {
            hub_node_socket_addr: String::new(),
        })
    }
}
impl Widget for CentralPanel {
    fn update(window: &mut Window, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            ui.add(
                egui::Image::new(egui::ImageSource::Bytes {
                    uri: Default::default(),
                    bytes: egui::load::Bytes::Static(ICON_FILE_DATA),
                })
                .max_size(egui::Vec2::new(ICON_WIDTH, ICON_HEIGHT)),
            );
            ui.horizontal(|ui| {
                ui.label("中枢服务器IP地址");
                ui.text_edit_singleline(&mut window.central_panel.hub_node_socket_addr);
            });
            if ui.button("登录").changed() {}
        });
    }
}
