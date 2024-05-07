use std::borrow::Cow;

use eframe::egui;

use crate::window::Window;

use super::Widget;

pub struct CentralPanel;
impl Widget for CentralPanel {
    fn update(window: &mut Window, ui: &mut egui::Ui, ctx: &egui::Context) {
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
                    let _ = opener::open("https://github.com/ZhangXiChang/node-network");
                }
            });
        });
        ui.label("=====================================================================");
        ui.vertical_centered(|ui| {
            ui.label("这里是作者玩耍的地方，✨欸嘿✨");
        });
    }
}
