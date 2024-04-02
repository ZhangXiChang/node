// mod widgets;

// use std::{
//     fs::{create_dir_all, File},
//     io::{stdout, Read},
//     net::SocketAddr,
//     path::PathBuf,
//     sync::{Arc, Mutex},
//     time::Duration,
// };

// use crossterm::{
//     event::{self, Event, KeyCode, KeyEventKind},
//     terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
//     ExecutableCommand,
// };
// use eyre::{eyre, Result};
// use quinn::{ClientConfig, Connection, Endpoint, ServerConfig, TransportConfig, VarInt};
// use ratatui::{
//     backend::CrosstermBackend,
//     layout::{Constraint, Layout},
//     style::{Color, Modifier, Style},
//     widgets::{Block, Borders},
//     Frame, Terminal,
// };
// use serde::{Deserialize, Serialize};
// use share::{x509_dns_name_from_der, DataPacket, RequestDataPacket, ResponseDataPacket};
// use tokio::task::JoinHandle;
// use tui_textarea::{CursorMove, TextArea};
// use uuid::Uuid;
// use widgets::{
//     menu_bar::{MenuBar, MenuBarInfo},
//     message_bar::{MessageBar, MessageBarInfo},
//     title_bar::{TitleBar, TitleBarInfo},
// };

// #[derive(Serialize, Deserialize)]
// struct RootNodeConfig {
//     ip_addr: SocketAddr,
//     dns_name: String,
// }
// #[derive(Serialize, Deserialize)]
// struct Config {
//     node_name: String,
//     dns_name: String,
//     root_node_config: RootNodeConfig,
// }

// pub enum Focus {
//     MenuBar,
//     MessageBar,
// }

// #[derive(Clone)]
// pub enum MenuBarState {
//     MainMenu,
//     WaitConnectMenu,
//     NodeListMenu,
//     ChatMenu,
// }
// impl From<MenuBarState> for Vec<&str> {
//     fn from(value: MenuBarState) -> Self {
//         match value {
//             MenuBarState::MainMenu => vec!["接收连接", "主动连接", "退出程序"],
//             MenuBarState::WaitConnectMenu => vec!["结束"],
//             MenuBarState::ChatMenu => vec!["断开连接"],
//             _ => vec![],
//         }
//     }
// }

// #[derive(Debug)]
// enum NatBehavior {
//     Undefined,
//     Invariant,
//     AutoIncrement,
// }

// struct App<'a> {
//     focus: Focus,
//     title_bar: TitleBar,
//     menu_bar: Arc<Mutex<MenuBar>>,
//     menu_bar_state: Arc<Mutex<MenuBarState>>,
//     message_bar: Arc<Mutex<MessageBar>>,
//     text_input_bar: TextArea<'a>,
//     endpoint: Endpoint,
//     root_node_connection: Connection,
//     name: String,
//     cert: Vec<u8>,
//     node_connection: Arc<Mutex<Option<Connection>>>,
//     node_name: Arc<Mutex<Option<String>>>,
//     hole_punching_task: Option<JoinHandle<Result<()>>>,
//     recv_connect_task: Option<JoinHandle<Result<()>>>,
//     nat_behavior: NatBehavior, //TODO 将检测到的Nat行为实际应用
// }
// impl<'a> App<'a> {
//     async fn new() -> Result<Self> {
//         //设置路径
//         let config_file_path = PathBuf::from("./config.json");
//         let cert_dir_path = PathBuf::from("./certs/");
//         //解析配置文件
//         let mut config = Config {
//             node_name: "无名氏".to_string(),
//             dns_name: Uuid::new_v4().to_string(),
//             root_node_config: RootNodeConfig {
//                 ip_addr: "127.0.0.1:10270".parse()?,
//                 dns_name: "local_node".to_string(),
//             },
//         };
//         match File::open(config_file_path.clone()) {
//             Ok(mut config_file) => {
//                 let mut config_bytes = Vec::new();
//                 config_file.read_to_end(&mut config_bytes)?;
//                 config = serde_json::from_slice(&config_bytes)?;
//             }
//             Err(_) => {
//                 config.serialize(&mut serde_json::Serializer::with_formatter(
//                     File::create(config_file_path)?,
//                     serde_json::ser::PrettyFormatter::with_indent(b"    "),
//                 ))?;
//             }
//         }
//         //加载根节点证书设置为默认信任证书
//         let mut root_node_cert_store = rustls::RootCertStore::empty();
//         create_dir_all(cert_dir_path.clone())?;
//         for dir_entry in cert_dir_path.read_dir()? {
//             if let Ok(dir_entry) = dir_entry {
//                 let path = dir_entry.path();
//                 if let Some(extension) = path.extension() {
//                     if extension == "cer" {
//                         let mut root_node_cert = Vec::new();
//                         File::open(path)?.read_to_end(&mut root_node_cert)?;
//                         root_node_cert_store.add(&rustls::Certificate(root_node_cert))?;
//                     }
//                 }
//             }
//         }
//         if root_node_cert_store.is_empty() {
//             return Err(eyre!("没有找到证书"));
//         }
//         //检测自身Nat行为
//         let mut nat_behavior = NatBehavior::Undefined;
//         let mut port = None;
//         for _ in 0..3 {
//             let mut detecting_endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
//             detecting_endpoint.set_default_client_config(ClientConfig::with_root_certificates(
//                 root_node_cert_store.clone(),
//             ));
//             let detecting_node = detecting_endpoint
//                 .connect(
//                     SocketAddr::new(
//                         config.root_node_config.ip_addr.ip(),
//                         config.root_node_config.ip_addr.port() + 1,
//                     ),
//                     &config.root_node_config.dns_name,
//                 )?
//                 .await?;
//             let mut recv = detecting_node.accept_uni().await?;
//             let temp_port = rmp_serde::from_slice::<u16>(&recv.read_to_end(usize::MAX).await?)?;
//             if let Some(port) = port {
//                 if temp_port == port {
//                     nat_behavior = NatBehavior::Invariant;
//                     break;
//                 }
//                 if temp_port == port + 1 {
//                     nat_behavior = NatBehavior::AutoIncrement;
//                     break;
//                 }
//             }
//             port = Some(temp_port);
//         }
//         //创建证书
//         let certificate =
//             rcgen::Certificate::from_params(rcgen::CertificateParams::new(vec![config.dns_name]))?;
//         //创建节点
//         let mut transport_config = TransportConfig::default();
//         transport_config.keep_alive_interval(Some(Duration::from_secs(5)));
//         let mut endpoint = Endpoint::server(
//             ServerConfig::with_single_cert(
//                 vec![rustls::Certificate(certificate.serialize_der()?)],
//                 rustls::PrivateKey(certificate.serialize_private_key_der()),
//             )?
//             .transport_config(Arc::new(transport_config))
//             .clone(),
//             "0.0.0.0:0".parse()?,
//         )?;
//         endpoint
//             .set_default_client_config(ClientConfig::with_root_certificates(root_node_cert_store));
//         //连接根节点
//         let root_node_connection = endpoint
//             .connect(
//                 config.root_node_config.ip_addr,
//                 &config.root_node_config.dns_name,
//             )?
//             .await?;
//         //获取根节点信息
//         let (mut send, mut recv) = root_node_connection.open_bi().await?;
//         send.write_all(&rmp_serde::to_vec(&DataPacket::Request(
//             RequestDataPacket::GetRootNodeInfo,
//         ))?)
//         .await?;
//         send.finish().await?;
//         let (root_node_name, root_node_description) =
//             match rmp_serde::from_slice::<DataPacket>(&recv.read_to_end(usize::MAX).await?)? {
//                 DataPacket::Response(ResponseDataPacket::GetRootNodeInfo { name, description }) => {
//                     (name, description)
//                 }
//                 _ => return Err(eyre!("服务端返回了预料之外的数据包")),
//             };
//         Ok(Self {
//             focus: Focus::MenuBar,
//             title_bar: TitleBar::new(TitleBarInfo {
//                 title: format!("欢迎使用节点网络，根节点[{}]为您服务", root_node_name).as_str(),
//             }),
//             menu_bar: Arc::new(Mutex::new(MenuBar::new(MenuBarInfo {
//                 title: "主菜单",
//                 title_modifier: Modifier::REVERSED,
//                 items: MenuBarState::MainMenu.into(),
//                 items_state: Some(0),
//             }))),
//             menu_bar_state: Arc::new(Mutex::new(MenuBarState::MainMenu)),
//             message_bar: Arc::new(Mutex::new(MessageBar::new(MessageBarInfo {
//                 title: "消息栏",
//                 items: vec![format!("{}：{}", root_node_name, root_node_description).as_str()],
//                 ..Default::default()
//             }))),
//             text_input_bar: {
//                 let mut text_input_bar = TextArea::default();
//                 text_input_bar.set_cursor_style(Style::default());
//                 text_input_bar.set_cursor_line_style(Style::default());
//                 text_input_bar.set_block(Block::new().borders(Borders::ALL));
//                 text_input_bar
//             },
//             endpoint,
//             root_node_connection,
//             name: config.node_name,
//             cert: certificate.serialize_der()?,
//             node_connection: Arc::new(Mutex::new(None)),
//             node_name: Arc::new(Mutex::new(None)),
//             hole_punching_task: None,
//             recv_connect_task: None,
//             nat_behavior,
//         })
//     }
//     async fn run(mut self) -> Result<()> {
//         //终端界面
//         stdout().execute(EnterAlternateScreen)?;
//         enable_raw_mode()?;
//         let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
//         let mut quit = false;
//         while !quit {
//             terminal.draw(|frame| {
//                 self.draw(frame);
//             })?;
//             if event::poll(Duration::ZERO)? {
//                 self.input_handling(&mut quit).await?;
//             }
//         }
//         disable_raw_mode()?;
//         stdout().execute(LeaveAlternateScreen)?;
//         Ok(())
//     }
//     fn draw(&self, frame: &mut Frame) {
//         //根布局
//         let [title_area, interactive_area] =
//             Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(frame.size());
//         //标题栏
//         self.title_bar.draw(frame, title_area);
//         //内容布局
//         let [menu_area, chat_area] =
//             Layout::horizontal([Constraint::Length(20), Constraint::Min(0)])
//                 .areas(interactive_area);
//         //菜单栏
//         {
//             self.menu_bar.lock().unwrap().draw(frame, menu_area);
//         }
//         //消息栏布局
//         let [message_area, text_input_area] =
//             Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).areas(chat_area);
//         //消息栏
//         {
//             self.message_bar.lock().unwrap().draw(frame, message_area);
//         }
//         //输入栏
//         frame.render_widget(self.text_input_bar.widget(), text_input_area);
//     }
//     async fn input_handling(&mut self, quit: &mut bool) -> Result<()> {
//         match event::read()? {
//             Event::Key(key) => match key.kind {
//                 KeyEventKind::Press => match self.focus {
//                     Focus::MenuBar => match key.code {
//                         KeyCode::Up => self.menu_bar.lock().unwrap().up_select(),
//                         KeyCode::Down => self.menu_bar.lock().unwrap().down_select(),
//                         KeyCode::Enter => match {
//                             let a = self.menu_bar_state.lock().unwrap().clone();
//                             a
//                         } {
//                             MenuBarState::MainMenu => {
//                                 if let Some(index) = {
//                                     let a = self.menu_bar.lock().unwrap().selected();
//                                     a
//                                 } {
//                                     match index {
//                                         0 => {
//                                             self.accept_connect().await?;
//                                         }
//                                         1 => {
//                                             let all_registered_node_name =
//                                                 self.get_all_registered_node_name().await?;
//                                             if !all_registered_node_name.is_empty() {
//                                                 {
//                                                     *self.menu_bar_state.lock().unwrap() =
//                                                         MenuBarState::NodeListMenu;
//                                                 }
//                                                 {
//                                                     let mut menu_bar =
//                                                         self.menu_bar.lock().unwrap();
//                                                     menu_bar.set_items(
//                                                         all_registered_node_name
//                                                             .iter()
//                                                             .map(|s| s.as_str())
//                                                             .collect(),
//                                                     );
//                                                     menu_bar.select(Some(0));
//                                                 }
//                                             } else {
//                                                 self.message_bar
//                                                     .lock()
//                                                     .unwrap()
//                                                     .append("没有注册的节点");
//                                             }
//                                         }
//                                         2 => *quit = true,
//                                         _ => (),
//                                     }
//                                 }
//                             }
//                             MenuBarState::WaitConnectMenu => {
//                                 if let Some(index) = {
//                                     let a = self.menu_bar.lock().unwrap().selected();
//                                     a
//                                 } {
//                                     match index {
//                                         0 => {
//                                             let (mut send, _) =
//                                                 self.root_node_connection.open_bi().await?;
//                                             send.write_all(&rmp_serde::to_vec(
//                                                 &DataPacket::UnRegisterNode,
//                                             )?)
//                                             .await?;
//                                             send.finish().await?;
//                                             {
//                                                 *self.menu_bar_state.lock().unwrap() =
//                                                     MenuBarState::MainMenu;
//                                             }
//                                             {
//                                                 let mut menu_bar = self.menu_bar.lock().unwrap();
//                                                 menu_bar.set_items(MenuBarState::MainMenu.into());
//                                                 menu_bar.select(Some(0));
//                                             }
//                                             self.hole_punching_task.as_ref().unwrap().abort();
//                                             self.recv_connect_task.as_ref().unwrap().abort();
//                                         }
//                                         _ => (),
//                                     }
//                                 }
//                             }
//                             MenuBarState::NodeListMenu => {
//                                 if let Some(index) = {
//                                     let a = self.menu_bar.lock().unwrap().selected();
//                                     a
//                                 } {
//                                     self.connect({
//                                         let a =
//                                             self.menu_bar.lock().unwrap().items()[index].clone();
//                                         a
//                                     })
//                                     .await?;
//                                 }
//                             }
//                             MenuBarState::ChatMenu => {
//                                 if let Some(index) = {
//                                     let a = self.menu_bar.lock().unwrap().selected();
//                                     a
//                                 } {
//                                     match index {
//                                         0 => {
//                                             let mut node_connection =
//                                                 self.node_connection.lock().unwrap();
//                                             node_connection.as_ref().unwrap().close(
//                                                 VarInt::from_u32(0),
//                                                 "主动关闭连接".as_bytes(),
//                                             );
//                                             *node_connection = None;
//                                         }
//                                         _ => (),
//                                     }
//                                 }
//                             }
//                         },
//                         KeyCode::Esc => match {
//                             let a = self.menu_bar_state.lock().unwrap().clone();
//                             a
//                         } {
//                             MenuBarState::NodeListMenu => {
//                                 {
//                                     *self.menu_bar_state.lock().unwrap() = MenuBarState::MainMenu;
//                                 }
//                                 {
//                                     let mut menu_bar = self.menu_bar.lock().unwrap();
//                                     menu_bar.set_items(MenuBarState::MainMenu.into());
//                                     menu_bar.select(Some(0));
//                                 }
//                             }
//                             _ => (),
//                         },
//                         KeyCode::Tab => {
//                             self.focus = Focus::MessageBar;
//                             {
//                                 self.menu_bar
//                                     .lock()
//                                     .unwrap()
//                                     .set_title_modifier(Modifier::default());
//                             }
//                             {
//                                 self.message_bar
//                                     .lock()
//                                     .unwrap()
//                                     .set_title_modifier(Modifier::REVERSED);
//                             }
//                             if {
//                                 let a = self.node_connection.lock().unwrap().is_some();
//                                 a
//                             } {
//                                 self.text_input_bar
//                                     .set_cursor_style(Style::default().bg(Color::Black));
//                             }
//                         }
//                         _ => (),
//                     },
//                     Focus::MessageBar => {
//                         match key.code {
//                             KeyCode::Tab => {
//                                 self.focus = Focus::MenuBar;
//                                 {
//                                     self.menu_bar
//                                         .lock()
//                                         .unwrap()
//                                         .set_title_modifier(Modifier::REVERSED);
//                                 }
//                                 {
//                                     self.message_bar
//                                         .lock()
//                                         .unwrap()
//                                         .set_title_modifier(Modifier::default());
//                                 }
//                                 self.text_input_bar.set_cursor_style(Style::default());
//                             }
//                             _ => (),
//                         }
//                         if let Some(connection) = {
//                             let a = self.node_connection.lock().unwrap().clone();
//                             a
//                         } {
//                             match key.code {
//                                 KeyCode::Char(_) => {
//                                     self.text_input_bar.input(key);
//                                 }
//                                 KeyCode::Backspace => {
//                                     self.text_input_bar.delete_char();
//                                 }
//                                 KeyCode::Left => self.text_input_bar.move_cursor(CursorMove::Back),
//                                 KeyCode::Right => {
//                                     self.text_input_bar.move_cursor(CursorMove::Forward)
//                                 }
//                                 KeyCode::Enter => {
//                                     let mut input_text = String::new();
//                                     for lines in self.text_input_bar.lines() {
//                                         input_text.push_str(&lines);
//                                     }
//                                     if !input_text.is_empty() {
//                                         {
//                                             self.message_bar.lock().unwrap().append(
//                                                 format!("{}：{}", self.name, input_text).as_str(),
//                                             );
//                                         }
//                                         self.text_input_bar.move_cursor(CursorMove::Head);
//                                         self.text_input_bar.delete_line_by_end();
//                                         let mut send = connection.open_uni().await?;
//                                         send.write_all(input_text.as_bytes()).await?;
//                                         send.finish().await?;
//                                     }
//                                 }
//                                 _ => (),
//                             }
//                         }
//                     }
//                 },
//                 _ => (),
//             },
//             _ => (),
//         }
//         Ok(())
//     }
//     async fn accept_connect(&mut self) -> Result<()> {
//         //注册节点
//         let (mut send, _) = self.root_node_connection.open_bi().await?;
//         send.write_all(&rmp_serde::to_vec(&DataPacket::RegisterNode {
//             name: self.name.clone(),
//             cert: self.cert.clone(),
//         })?)
//         .await?;
//         send.finish().await?;
//         {
//             self.message_bar.lock().unwrap().append("等待连接...");
//         }
//         {
//             *self.menu_bar_state.lock().unwrap() = MenuBarState::WaitConnectMenu;
//         }
//         {
//             let mut menu_bar = self.menu_bar.lock().unwrap();
//             menu_bar.set_items(MenuBarState::WaitConnectMenu.into());
//             menu_bar.select(Some(0));
//         }
//         //接收打洞信号
//         self.hole_punching_task = Some(tokio::spawn({
//             let endpoint = self.endpoint.clone();
//             let root_node_connection = self.root_node_connection.clone();
//             let node_name = self.node_name.clone();
//             async move {
//                 let (mut send, mut recv) = root_node_connection.accept_bi().await?;
//                 match rmp_serde::from_slice::<DataPacket>(&recv.read_to_end(usize::MAX).await?)? {
//                     DataPacket::Request(RequestDataPacket::HolePunching {
//                         node_name: name,
//                         ip_addr,
//                     }) => {
//                         *node_name.lock().unwrap() = Some(name);
//                         let _ = endpoint.connect(ip_addr, "_")?.await;
//                         send.write_all(&rmp_serde::to_vec(&DataPacket::Response(
//                             ResponseDataPacket::HolePunching,
//                         ))?)
//                         .await?;
//                         send.finish().await?;
//                     }
//                     _ => (),
//                 }
//                 eyre::Ok(())
//             }
//         }));
//         //接收连接
//         self.recv_connect_task = Some(tokio::spawn({
//             let endpoint = self.endpoint.clone();
//             let message_bar = self.message_bar.clone();
//             let node_connection = self.node_connection.clone();
//             let menu_bar = self.menu_bar.clone();
//             let root_node_connection = self.root_node_connection.clone();
//             let menu_bar_state = self.menu_bar_state.clone();
//             let node_name = self.node_name.clone();
//             async move {
//                 if let Some(connecting) = endpoint.accept().await {
//                     let connection = connecting.await?;
//                     let (mut send, _) = root_node_connection.open_bi().await?;
//                     send.write_all(&rmp_serde::to_vec(&DataPacket::UnRegisterNode)?)
//                         .await?;
//                     send.finish().await?;
//                     Self::connection_handling(
//                         connection,
//                         {
//                             let a = node_name.lock().unwrap().as_ref().unwrap().clone();
//                             a
//                         },
//                         node_connection,
//                         message_bar,
//                         menu_bar,
//                         menu_bar_state,
//                     );
//                 }
//                 eyre::Ok(())
//             }
//         }));
//         Ok(())
//     }
//     async fn get_all_registered_node_name(&self) -> Result<Vec<String>> {
//         //从根节点获取注册的节点
//         let (mut send, mut recv) = self.root_node_connection.open_bi().await?;
//         send.write_all(&rmp_serde::to_vec(&DataPacket::Request(
//             RequestDataPacket::GetAllRegisteredNodeName,
//         ))?)
//         .await?;
//         send.finish().await?;
//         match rmp_serde::from_slice::<DataPacket>(&recv.read_to_end(usize::MAX).await?)? {
//             DataPacket::Response(ResponseDataPacket::GetAllRegisteredNodeName {
//                 all_registered_node_name,
//             }) => {
//                 return Ok(all_registered_node_name);
//             }
//             _ => (),
//         }
//         return Err(eyre!("从根节点获取全部注册的节点失败"));
//     }
//     async fn connect(&mut self, node_name: String) -> Result<()> {
//         //通过根节点连接节点
//         let (mut send, mut recv) = self.root_node_connection.open_bi().await?;
//         send.write_all(&rmp_serde::to_vec(&DataPacket::Request(
//             RequestDataPacket::GetRegisteredNodeIPAddrAndCert {
//                 name: self.name.clone(),
//                 node_name: node_name.clone(),
//             },
//         ))?)
//         .await?;
//         send.finish().await?;
//         match rmp_serde::from_slice::<DataPacket>(&recv.read_to_end(usize::MAX).await?)? {
//             DataPacket::Response(ResponseDataPacket::GetRegisteredNodeIPAddrAndCert(Some(
//                 node_addr_and_cert,
//             ))) => {
//                 //连接节点
//                 let mut node_cert_store = rustls::RootCertStore::empty();
//                 node_cert_store.add(&rustls::Certificate(node_addr_and_cert.cert.clone()))?;
//                 match self
//                     .endpoint
//                     .connect_with(
//                         ClientConfig::with_root_certificates(node_cert_store),
//                         node_addr_and_cert.ip_addr,
//                         &x509_dns_name_from_der(&node_addr_and_cert.cert)?,
//                     )?
//                     .await
//                 {
//                     Ok(connection) => {
//                         Self::connection_handling(
//                             connection,
//                             node_name,
//                             self.node_connection.clone(),
//                             self.message_bar.clone(),
//                             self.menu_bar.clone(),
//                             self.menu_bar_state.clone(),
//                         );
//                     }
//                     Err(err) => {
//                         {
//                             self.message_bar
//                                 .lock()
//                                 .unwrap()
//                                 .append(format!("节点连接失败，原因：{}", err).as_str());
//                         }
//                         {
//                             *self.menu_bar_state.lock().unwrap() = MenuBarState::MainMenu;
//                         }
//                         {
//                             let mut menu_bar = self.menu_bar.lock().unwrap();
//                             menu_bar.set_items(MenuBarState::MainMenu.into());
//                             menu_bar.select(Some(0));
//                         }
//                     }
//                 };
//             }
//             _ => {
//                 {
//                     self.message_bar.lock().unwrap().append("没有找到节点");
//                 }
//                 {
//                     *self.menu_bar_state.lock().unwrap() = MenuBarState::MainMenu;
//                 }
//                 {
//                     let mut menu_bar = self.menu_bar.lock().unwrap();
//                     menu_bar.set_items(MenuBarState::MainMenu.into());
//                     menu_bar.select(Some(0));
//                 }
//             }
//         }
//         Ok(())
//     }
//     fn connection_handling(
//         connection: Connection,
//         node_name: String,
//         node_connection: Arc<Mutex<Option<Connection>>>,
//         message_bar: Arc<Mutex<MessageBar>>,
//         menu_bar: Arc<Mutex<MenuBar>>,
//         menu_bar_state: Arc<Mutex<MenuBarState>>,
//     ) {
//         {
//             message_bar
//                 .lock()
//                 .unwrap()
//                 .append(format!("[{}]连接成功", node_name).as_str());
//         }
//         //接收数据
//         tokio::spawn({
//             let connection = connection.clone();
//             let message_bar = message_bar.clone();
//             let menu_bar = menu_bar.clone();
//             let menu_bar_state = menu_bar_state.clone();
//             async move {
//                 loop {
//                     match connection.accept_uni().await {
//                         Ok(mut recv) => {
//                             let msg = recv.read_to_end(usize::MAX).await?;
//                             {
//                                 message_bar.lock().unwrap().append(
//                                     format!("{}：{}", node_name, String::from_utf8(msg)?).as_str(),
//                                 );
//                             }
//                         }
//                         Err(err) => {
//                             {
//                                 message_bar.lock().unwrap().append(
//                                     format!("[{}]断开连接，原因：{}", node_name, err).as_str(),
//                                 );
//                             }
//                             {
//                                 *menu_bar_state.lock().unwrap() = MenuBarState::MainMenu;
//                             }
//                             {
//                                 let mut menu_bar = menu_bar.lock().unwrap();
//                                 menu_bar.set_items(MenuBarState::MainMenu.into());
//                                 menu_bar.select(Some(0));
//                             }
//                             break;
//                         }
//                     }
//                 }
//                 eyre::Ok(())
//             }
//         });
//         {
//             *node_connection.lock().unwrap() = Some(connection);
//         }
//         {
//             *menu_bar_state.lock().unwrap() = MenuBarState::ChatMenu;
//         }
//         {
//             let mut menu_bar = menu_bar.lock().unwrap();
//             menu_bar.set_items(MenuBarState::ChatMenu.into());
//             menu_bar.select(Some(0));
//         }
//     }
// }

mod system;

use eyre::Result;
use ratatui::{
    layout::{Constraint, Layout},
    style::Modifier,
};
use share::ArcMutex;
use system::{
    widget::{
        menu_bar::{MenuBar, MenuBarInfo},
        message_bar::{MessageBar, MessageBarInfo},
        title_bar::{TitleBar, TitleBarInfo},
        Widget, WidgetLayout,
    },
    System,
};

#[tokio::main]
async fn main() -> Result<()> {
    System::new(vec![ArcMutex::new(Box::new(Widget::new(WidgetLayout {
        layout: Layout::vertical([Constraint::Length(3), Constraint::Min(0)]),
        widgets: vec![(
            Box::new(TitleBar::new(TitleBarInfo {
                title: "这是标题"
            })),
            0,
        )],
        sub_widget_layout: Some((
            Box::new(WidgetLayout {
                layout: Layout::horizontal([Constraint::Length(20), Constraint::Min(0)]),
                widgets: vec![
                    (
                        Box::new(MenuBar::new(MenuBarInfo {
                            title: "主菜单",
                            title_modifier: Modifier::REVERSED,
                            items: vec!["接收连接", "主动连接", "退出程序"],
                            items_state: Some(0),
                        })),
                        0,
                    ),
                    (
                        Box::new(MessageBar::new(MessageBarInfo {
                            title: "消息栏",
                            ..Default::default()
                        })),
                        1,
                    ),
                ],
                sub_widget_layout: None,
            }),
            1,
        )),
    })?))])
    .run()?;
    Ok(())
}
