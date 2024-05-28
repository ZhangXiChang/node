pub mod central_panel;
pub mod menu_bar;
pub mod state_bar;

use eframe::egui;

use super::Window;

pub trait Widget {
    fn update(window: &mut Window, ui: &mut egui::Ui, ctx: &egui::Context);
}
