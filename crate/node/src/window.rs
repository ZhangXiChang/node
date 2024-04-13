use std::sync::Arc;

use eframe::egui;
use eyre::{eyre, Result};

pub struct Size {
    pub width: f32,
    pub height: f32,
}

pub struct Icon {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Default)]
pub struct Info<'a> {
    pub app_name: Option<&'a str>,
    pub font_data: Option<Vec<u8>>,
    pub icon: Option<Icon>,
    pub inner_size: Option<Size>,
    pub resizable: Option<bool>,
    pub maximize_button: Option<bool>,
}
pub fn new(info: Info, eframe_app: impl eframe::App + 'static) -> Result<()> {
    eframe::run_native(
        info.app_name.map_or("标题", |v| v),
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder {
                icon: info.icon.map_or(
                    Some(Arc::new(egui::IconData {
                        rgba: image::load_from_memory(include_bytes!(
                            "../../../assets/icon/node_network_icon.png"
                        ))?
                        .into_bytes(),
                        width: 512,
                        height: 512,
                    })),
                    |v| {
                        Some(Arc::new(egui::IconData {
                            rgba: v.rgba,
                            width: v.width,
                            height: v.height,
                        }))
                    },
                ),
                inner_size: info
                    .inner_size
                    .map_or(None, |v| Some(egui::Vec2::new(v.width, v.height))),
                resizable: info.resizable,
                maximize_button: info.maximize_button,
                ..Default::default()
            },
            ..Default::default()
        },
        Box::new(|cc| {
            set_font(info.font_data, &cc.egui_ctx);
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::new(eframe_app)
        }),
    )
    .map_err(|err| eyre!("{}", err))?;
    Ok(())
}
fn set_font(font_data: Option<Vec<u8>>, ctx: &egui::Context) {
    let mut font_definitions = egui::FontDefinitions::default();
    font_definitions.font_data.insert(
        "font".to_owned(),
        egui::FontData::from_owned(font_data.map_or(
            include_bytes!("../../../assets/fonts/SourceHanSansCN-Bold.otf").to_vec(),
            |v| v,
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
