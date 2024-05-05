use eframe::egui;

#[derive(Clone)]
pub struct WidgetLayoutStateSwitchButtonText(String);
impl From<WidgetLayoutStateSwitchButtonText> for String {
    fn from(value: WidgetLayoutStateSwitchButtonText) -> Self {
        let WidgetLayoutStateSwitchButtonText(text) = value;
        text
    }
}
impl From<super::WidgetLayoutState> for WidgetLayoutStateSwitchButtonText {
    fn from(value: super::WidgetLayoutState) -> Self {
        match value {
            super::WidgetLayoutState::Fold => Self("🗖 展开程序".to_string()),
            super::WidgetLayoutState::Unfold => Self("🗕 折叠程序".to_string()),
        }
    }
}

pub struct MenuBar {
    pub widget_layout_state_switch_button_text: WidgetLayoutStateSwitchButtonText,
}
impl MenuBar {
    pub fn update(widget: &mut super::Widget, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.menu_button("关于", |ui| {
            ui.label("版本：0.1.0");
            ui.label("作者：✨张喜昌✨");
            if ui.link("源代码").clicked() {
                opener::open("https://github.com/ZhangXiChang/node-network").unwrap();
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button(String::from(
                    widget
                        .menu_bar
                        .widget_layout_state_switch_button_text
                        .clone(),
                ))
                .clicked()
            {
                match widget.widget_layout_state {
                    super::WidgetLayoutState::Fold => {
                        widget.widget_layout_state_switch(ctx, super::WidgetLayoutState::Unfold);
                    }
                    super::WidgetLayoutState::Unfold => {
                        widget.widget_layout_state_switch(ctx, super::WidgetLayoutState::Fold);
                    }
                }
            }
            match widget.widget_layout_state {
                super::WidgetLayoutState::Fold => {
                    ui.menu_button("切换视图", |ui| {
                        ui.radio_value(
                            &mut widget.central_panel.fold_central_panel.widget_layout_state,
                            super::FoldCentralPanelLayoutState::Readme,
                            "软件自述视图",
                        );
                        ui.radio_value(
                            &mut widget.central_panel.fold_central_panel.widget_layout_state,
                            super::FoldCentralPanelLayoutState::ConnectRootNode,
                            "连接根节点视图",
                        );
                    });
                }
                super::WidgetLayoutState::Unfold => (),
            }
        });
    }
}
