mod node;
mod system;

use std::sync::Arc;

use eframe::egui;
use eyre::{eyre, Result};
use system::System;

struct App {
    system: System,
}
impl App {
    fn new() -> Result<Self> {
        Ok(Self {
            system: System::new()?,
        })
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("按钮").clicked() {
                tokio::spawn({
                    let node = self.system.node.clone();
                    async move {
                        match async {
                            node.register_node(
                                "127.0.0.1:10270".parse()?,
                                include_bytes!("../../../certs/root_node.cer").to_vec(),
                            )
                            .await?;
                            eyre::Ok(())
                        }
                        .await
                        {
                            Ok(_) => (),
                            Err(err) => log::error!("{}", err),
                        }
                    }
                });
            }
        });
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();
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
                inner_size: Some(egui::Vec2::new(1000., 750.)),
                resizable: Some(false),
                maximize_button: Some(false),
                ..Default::default()
            },
            ..Default::default()
        },
        Box::new(|cc| {
            set_font(&cc.egui_ctx);
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::new(match App::new() {
                Ok(app) => app,
                Err(err) => {
                    log::error!("{}", err);
                    panic!()
                }
            })
        }),
    )
    .map_err(|err| eyre!("{}", err))?;
    Ok(())
}
fn set_font(ctx: &egui::Context) {
    let mut font_definitions = egui::FontDefinitions::default();
    font_definitions.font_data.insert(
        "font".to_string(),
        egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/SourceHanSansCN-Bold.otf"
        )),
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
