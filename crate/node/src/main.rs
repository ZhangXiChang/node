use std::{borrow::Cow, sync::Arc};

use eyre::{eyre, Result};

use eframe::egui;
use log::{info, LevelFilter};

const ICON: &[u8] = include_bytes!("../../../assets/icon/node_network_icon.png");
const ICON_WIDTH: u32 = 512;
const ICON_HEIGHT: u32 = 512;

#[derive(Default)]
struct App;
impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("MenuBar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("关闭").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                let (response, _) = ui.allocate_painter(ui.available_size(), egui::Sense::drag());
                if response.dragged() {
                    info!("拖动的距离{:?}", response.drag_delta());
                    ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::Pos2::new(
                        0., -10.,
                    )));
                }
            });
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
    env_logger::builder().filter_level(LevelFilter::Info).init();
    //运行应用
    eframe::run_native(
        "节点网络",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder {
                icon: Some(Arc::new(egui::IconData {
                    rgba: image::load_from_memory(ICON)?.into_bytes(),
                    width: ICON_WIDTH,
                    height: ICON_HEIGHT,
                })),
                inner_size: Some(egui::Vec2::new(
                    ICON_WIDTH as f32 + 300.,
                    ICON_HEIGHT as f32 + 300.,
                )),
                resizable: Some(false),
                decorations: Some(true),
                ..Default::default()
            },
            ..Default::default()
        },
        Box::new(|cc| {
            set_font(&cc.egui_ctx);
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Box::new(App::default())
        }),
    )
    .map_err(|err| eyre!("{}", err))?;
    Ok(())
}
fn set_font(ctx: &egui::Context) {
    let mut font_definitions = egui::FontDefinitions::default();
    font_definitions.font_data.insert(
        "SourceHanSansCN-Bold".to_owned(),
        egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/SourceHanSansCN-Bold.otf"
        )),
    );
    font_definitions
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "SourceHanSansCN-Bold".to_owned());
    font_definitions
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("SourceHanSansCN-Bold".to_owned());
    ctx.set_fonts(font_definitions);
}
