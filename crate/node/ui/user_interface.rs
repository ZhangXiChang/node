
use fltk::browser::*;
use fltk::button::*;
use fltk::dialog::*;
use fltk::enums::*;
use fltk::frame::*;
use fltk::group::*;
use fltk::group::experimental::*;
use fltk::image::*;
use fltk::input::*;
use fltk::menu::*;
use fltk::misc::*;
use fltk::output::*;
use fltk::prelude::*;
use fltk::table::*;
use fltk::text::*;
use fltk::tree::*;
use fltk::valuator::*;
use fltk::widget::*;
use fltk::window::*;

#[derive(Debug, Clone)]
pub struct MainWindow {
    pub window: Window,
    pub root_node_connect_button: Button,
    pub root_node_ip_addr_text_input: Input,
    pub root_node_connect_state_text: Output,
    pub node_name_text_input: Input,
}

impl MainWindow {
    pub fn new() -> Self {
	let mut window = Window::new(498, 278, 1180, 523, None);
	window.set_label(r#"对等网络节点"#);
	window.set_type(WindowType::Double);
	window.set_color(Color::by_index(7));
	let mut root_node_connect_button = Button::new(575, 255, 50, 25, None);
	root_node_connect_button.set_label(r#"连接"#);
	root_node_connect_button.set_color(Color::by_index(7));
	let mut root_node_ip_addr_text_input = Input::new(250, 310, 160, 25, None);
	root_node_ip_addr_text_input.set_label(r#"根节点IP地址："#);
	let mut root_node_connect_state_text = Output::new(185, 450, 80, 25, None);
	root_node_connect_state_text.set_label(r#"根节点连接状态："#);
	let mut node_name_text_input = Input::new(495, 150, 70, 25, None);
	node_name_text_input.set_label(r#"节点名称"#);
	let mut fl2rust_widget_0 = FileBrowser::new(725, 65, 370, 370, None);
	window.end();
	window.show();
	Self {
	    window,
	    root_node_connect_button,
	    root_node_ip_addr_text_input,
	    root_node_connect_state_text,
	    node_name_text_input,
	}
    }
}


