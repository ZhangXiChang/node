use std::{borrow::Cow, sync::Arc};

use eyre::{eyre, Result};

use eframe::egui;
use log::error;

const ICON: &[u8] = include_bytes!("../../../assets/icon/node_network_icon.png");
const ICON_WIDTH: u32 = 512;
const ICON_HEIGHT: u32 = 512;

struct App;
impl Default for App {
    fn default() -> Self {
        Self
    }
}
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("MenuBar").show(ctx, |ui| {
            if ui.button("绘制曲线").clicked() {
                error!("执行绘制曲线");
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add(
                egui::Image::new(egui::ImageSource::Bytes {
                    uri: Cow::default(),
                    bytes: egui::load::Bytes::Static(ICON),
                })
                .max_size(egui::Vec2::new(ICON_WIDTH as f32, ICON_HEIGHT as f32)),
            );
        });
    }
}

fn main() -> Result<()> {
    //初始化日志消费者
    env_logger::init();
    //加载应用图标
    let icon = image::load_from_memory(ICON)?;
    //运行应用
    eframe::run_native(
        "节点网络",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder {
                icon: Some(Arc::new(egui::IconData {
                    rgba: icon.into_bytes(),
                    width: ICON_WIDTH,
                    height: ICON_HEIGHT,
                })),
                inner_size: Some(egui::Vec2::new(
                    ICON_WIDTH as f32 + 145.,
                    ICON_HEIGHT as f32,
                )),
                resizable: Some(false),
                ..Default::default()
            },
            ..Default::default()
        },
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            set_font(&cc.egui_ctx);
            Box::new(App::default())
        }),
    )
    .map_err(|err| eyre!("{}", err))?;
    Ok(())
}
fn set_font(ctx: &egui::Context) {
    let mut font_definitions = egui::FontDefinitions::default();
    font_definitions.font_data.insert(
        "SourceHanSansCN-Bold".to_string(),
        egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/SourceHanSansCN-Bold.otf"
        )),
    );
    font_definitions
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "SourceHanSansCN-Bold".to_string());
    font_definitions
        .families
        .get_mut(&egui::FontFamily::Monospace)
        .unwrap()
        .push("SourceHanSansCN-Bold".to_string());
    ctx.set_fonts(font_definitions);
}
