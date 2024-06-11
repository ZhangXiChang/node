use eframe::egui;

use crate::window::Window;

use super::Widget;

pub struct MenuBar;
impl Widget for MenuBar {
    fn update(_window: &mut Window, ui: &mut egui::Ui, _ctx: &egui::Context) {
        ui.menu_button("关于", |ui| {
            ui.label("版本: 0.1.0");
            ui.horizontal(|ui| {
                ui.label("作者:");
                if ui.link("✨张喜昌✨").clicked() {
                    match opener::open("https://github.com/ZhangXiChang") {
                        Ok(_) => (),
                        Err(err) => log::warn!("打开失败，原因：{}", err),
                    }
                }
            });
            if ui.link("源代码").clicked() {
                match opener::open("https://github.com/ZhangXiChang/node") {
                    Ok(_) => (),
                    Err(err) => log::warn!("打开失败，原因：{}", err),
                }
            }
        });
    }
}
