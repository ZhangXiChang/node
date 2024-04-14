use std::sync::Arc;

use eframe::egui;
use eyre::{eyre, Result};

pub struct Window;
impl Window {
    pub fn new(eframe_app: impl eframe::App + 'static) -> Result<()> {
        eframe::run_native(
            "节点网络",
            eframe::NativeOptions {
                viewport: egui::ViewportBuilder {
                    icon: Some(Arc::new(egui::IconData {
                        rgba: image::load_from_memory(include_bytes!(
                            "../../../assets/icon/node_network_icon.png"
                        ))?
                        .into_bytes(),
                        width: 512,
                        height: 512,
                    })),
                    inner_size: Some(egui::Vec2::new(500., 500. + 50.)),
                    resizable: Some(false),
                    maximize_button: Some(false),
                    ..Default::default()
                },
                ..Default::default()
            },
            Box::new(|cc| {
                Self::set_font(&cc.egui_ctx);
                egui_extras::install_image_loaders(&cc.egui_ctx);
                Box::new(eframe_app)
            }),
        )
        .map_err(|err| eyre!("{}", err))?;
        Ok(())
    }
    fn set_font(ctx: &egui::Context) {
        let mut font_definitions = egui::FontDefinitions::default();
        font_definitions.font_data.insert(
            "font".to_owned(),
            egui::FontData::from_static(include_bytes!(
                "../../../assets/fonts/SourceHanSansCN-Bold.otf"
            )),
        );
        font_definitions
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "font".to_owned());
        font_definitions
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .push("font".to_owned());
        ctx.set_fonts(font_definitions);
    }
}
