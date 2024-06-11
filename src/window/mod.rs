mod widget;

use std::sync::Arc;

use eframe::egui;
use eyre::{eyre, Result};

use crate::{system::System, FONT_FILE_DATA, ICON_FILE_DATA};

use self::widget::{central_panel::CentralPanel, menu_bar::MenuBar, state_bar::StateBar, Widget};

pub struct Window {
    system: System,
    state_bar: StateBar,
    central_panel: CentralPanel,
}
impl Window {
    pub fn new(system: System) -> Result<()> {
        let self_ = Self {
            system,
            state_bar: StateBar::new(),
            central_panel: CentralPanel::new()?,
        };
        eframe::run_native(
            "节点网络",
            eframe::NativeOptions {
                viewport: egui::ViewportBuilder {
                    icon: Some(Arc::new(egui::IconData {
                        rgba: image::load_from_memory(ICON_FILE_DATA)?.into_bytes(),
                        width: 512,
                        height: 512,
                    })),
                    inner_size: Some(egui::Vec2::new(1250., 800.)),
                    resizable: Some(false),
                    maximize_button: Some(false),
                    ..Default::default()
                },
                ..Default::default()
            },
            Box::new(|cc| {
                Self::set_font(&cc.egui_ctx);
                egui_extras::install_image_loaders(&cc.egui_ctx);
                Box::new(self_)
            }),
        )
        .map_err(|err| eyre!("{}", err))?;
        Ok(())
    }
    fn set_font(ctx: &egui::Context) {
        let mut font_definitions = egui::FontDefinitions::default();
        font_definitions.font_data.insert(
            "font".to_string(),
            egui::FontData::from_static(FONT_FILE_DATA),
        );
        font_definitions
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "font".to_string());
        font_definitions
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("font".to_string());
        ctx.set_fonts(font_definitions);
    }
}
impl eframe::App for Window {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("MenuBar").show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                MenuBar::update(self, ui, ctx);
            });
        });
        egui::TopBottomPanel::bottom("StateBar").show(ctx, |ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                StateBar::update(self, ui, ctx);
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| CentralPanel::update(self, ui, ctx));
    }
}
