use std::{
    fs::{create_dir_all, File},
    io::{stdout, Read},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use eyre::{eyre, Result};
use quinn::{ClientConfig, Connection, Endpoint, ServerConfig, TransportConfig, VarInt};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout},
    style::{self, Modifier, Style, Stylize},
    widgets::{Block, Borders, List, ListDirection, ListState, Paragraph},
    Frame, Terminal,
};
use serde::{Deserialize, Serialize};
use share::{x509_dns_name_from_der, DataPacket, RequestDataPacket, ResponseDataPacket};
use tokio::task::JoinHandle;
use tui_textarea::{CursorMove, TextArea};

enum Focus {
    MenuBar,
    MessageBar,
}
#[derive(Clone, Copy)]
enum MenuBarState {
    MainMenu,
    WaitConnectMenu,
    NodeListMenu,
    ChatMenu,
}
impl MenuBarState {
    fn to_menu_items(&self) -> Vec<String> {
        match self {
            MenuBarState::MainMenu => vec![
                "接收连接".to_string(),
                "主动连接".to_string(),
                "退出程序".to_string(),
            ],
            MenuBarState::WaitConnectMenu => vec!["结束".to_string()],
            MenuBarState::ChatMenu => vec!["断开连接".to_string()],
            _ => vec![],
        }
    }
}

#[derive(Serialize, Deserialize)]
struct RootNodeConfig {
    ip_addr: String,
    dns_name: String,
}
#[derive(Serialize, Deserialize)]
struct Config {
    node_name: String,
    dns_name: String,
    root_node_config: RootNodeConfig,
}

struct TitleBar {
    title: String,
}
struct MenuBar {
    title: String,
    title_modifier: Modifier,
    items: Vec<String>,
    state: MenuBarState,
    items_state: ListState,
}
struct MessageBar {
    title: String,
    title_modifier: Modifier,
    items: Vec<String>,
}
struct App<'a> {
    focus: Focus,
    title_bar: TitleBar,
    menu_bar: Arc<Mutex<MenuBar>>,
    message_bar: Arc<Mutex<MessageBar>>,
    text_input_bar: TextArea<'a>,
    endpoint: Endpoint,
    root_node_connection: Connection,
    node_name: String,
    cert: Vec<u8>,
    node_connection: Arc<Mutex<Option<Connection>>>,
    hole_punching_task: Option<JoinHandle<Result<()>>>,
    recv_connect_task: Option<JoinHandle<Result<()>>>,
}
impl<'a> App<'a> {
    async fn new() -> Result<Self> {
        //设置路径
        let config_file_path = PathBuf::from("./config.json");
        let cert_dir_path = PathBuf::from("./certs/");
        //解析配置文件
        let mut config = Config {
            node_name: "无名氏".to_string(),
            dns_name: "node".to_string(),
            root_node_config: RootNodeConfig {
                ip_addr: "127.0.0.1:10270".to_string(),
                dns_name: "local_node".to_string(),
            },
        };
        match File::open(config_file_path.clone()) {
            Ok(mut config_file) => {
                let mut config_bytes = Vec::new();
                config_file.read_to_end(&mut config_bytes)?;
                config = serde_json::from_slice(&config_bytes)?;
            }
            Err(_) => {
                config.serialize(&mut serde_json::Serializer::with_formatter(
                    File::create(config_file_path)?,
                    serde_json::ser::PrettyFormatter::with_indent(b"    "),
                ))?;
            }
        }
        //创建证书
        let certificate =
            rcgen::Certificate::from_params(rcgen::CertificateParams::new(vec![config.dns_name]))?;
        //创建节点
        let mut transport_config = TransportConfig::default();
        transport_config.keep_alive_interval(Some(Duration::from_secs(5)));
        let mut endpoint = Endpoint::server(
            ServerConfig::with_single_cert(
                vec![rustls::Certificate(certificate.serialize_der()?)],
                rustls::PrivateKey(certificate.serialize_private_key_der()),
            )?
            .transport_config(Arc::new(transport_config))
            .clone(),
            "0.0.0.0:0".parse()?,
        )?;
        //加载根节点证书设置为默认信任证书
        let mut root_node_cert_store = rustls::RootCertStore::empty();
        create_dir_all(cert_dir_path.clone())?;
        for dir_entry in cert_dir_path.read_dir()? {
            if let Ok(dir_entry) = dir_entry {
                let path = dir_entry.path();
                if let Some(extension) = path.extension() {
                    if extension == "cer" {
                        let mut root_node_cert = Vec::new();
                        File::open(path)?.read_to_end(&mut root_node_cert)?;
                        root_node_cert_store.add(&rustls::Certificate(root_node_cert))?;
                    }
                }
            }
        }
        if root_node_cert_store.is_empty() {
            return Err(eyre!("没有找到证书"));
        }
        endpoint
            .set_default_client_config(ClientConfig::with_root_certificates(root_node_cert_store));
        //连接根节点
        let root_node_connection = endpoint
            .connect(
                config.root_node_config.ip_addr.parse()?,
                &config.root_node_config.dns_name,
            )?
            .await?;
        //获取根节点信息
        let (mut send, mut recv) = root_node_connection.open_bi().await?;
        send.write_all(&rmp_serde::to_vec(&DataPacket::Request(
            RequestDataPacket::GetRootNodeInfo,
        ))?)
        .await?;
        send.finish().await?;
        let (root_node_name, root_node_description) =
            match rmp_serde::from_slice::<DataPacket>(&recv.read_to_end(usize::MAX).await?)? {
                DataPacket::Response(ResponseDataPacket::GetRootNodeInfo { name, description }) => {
                    (name, description)
                }
                _ => return Err(eyre!("服务端返回了预料之外的数据包")),
            };
        Ok(Self {
            focus: Focus::MenuBar,
            title_bar: TitleBar {
                title: format!("欢迎使用节点网络，根节点[{}]为您服务", root_node_name),
            },
            menu_bar: Arc::new(Mutex::new(MenuBar {
                title: "主菜单".to_string(),
                title_modifier: Modifier::REVERSED,
                items: MenuBarState::MainMenu.to_menu_items(),
                state: MenuBarState::MainMenu,
                items_state: {
                    let mut items_state = ListState::default();
                    items_state.select(Some(0));
                    items_state
                },
            })),
            message_bar: Arc::new(Mutex::new(MessageBar {
                title: "消息栏".to_string(),
                title_modifier: Modifier::default(),
                items: vec![format!("{}：{}", root_node_name, root_node_description)],
            })),
            text_input_bar: {
                let mut text_input_bar = TextArea::default();
                text_input_bar.set_cursor_style(Style::default());
                text_input_bar.set_cursor_line_style(Style::default());
                text_input_bar.set_block(Block::new().borders(Borders::ALL));
                text_input_bar
            },
            endpoint,
            root_node_connection,
            node_name: config.node_name,
            cert: certificate.serialize_der()?,
            node_connection: Arc::new(Mutex::new(None)),
            hole_punching_task: None,
            recv_connect_task: None,
        })
    }
    async fn run(mut self) -> Result<()> {
        //终端界面
        stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        let mut quit = false;
        while !quit {
            terminal.draw(|frame| {
                self.draw(frame);
            })?;
            if event::poll(Duration::ZERO)? {
                self.input_handling(&mut quit).await?;
            }
        }
        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;
        Ok(())
    }
    fn draw(&self, frame: &mut Frame) {
        let [title_area, interactive_area] =
            Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).areas(frame.size());
        frame.render_widget(
            Paragraph::new(self.title_bar.title.clone())
                .block(Block::new().borders(Borders::ALL))
                .alignment(Alignment::Center),
            title_area,
        );

        let [menu_area, chat_area] =
            Layout::horizontal([Constraint::Length(20), Constraint::Min(0)])
                .areas(interactive_area);
        frame.render_stateful_widget(
            List::new({
                let a = self.menu_bar.lock().unwrap().items.clone();
                a
            })
            .block(Block::new().borders(Borders::ALL).title({
                let title_modifier = {
                    let a = self.menu_bar.lock().unwrap().title_modifier;
                    a
                };
                let a = self
                    .menu_bar
                    .lock()
                    .unwrap()
                    .title
                    .clone()
                    .add_modifier(title_modifier);
                a
            }))
            .highlight_style(Style::new().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> "),
            menu_area,
            &mut self.menu_bar.lock().unwrap().items_state,
        );

        let [message_area, text_input_area] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(3)]).areas(chat_area);
        frame.render_widget(
            List::new({
                let a = self.message_bar.lock().unwrap().items.clone();
                a
            })
            .direction(ListDirection::BottomToTop)
            .block(Block::new().borders(Borders::ALL).title({
                let title_modifier = {
                    let a = self.message_bar.lock().unwrap().title_modifier;
                    a
                };
                let a = self
                    .message_bar
                    .lock()
                    .unwrap()
                    .title
                    .clone()
                    .add_modifier(title_modifier);
                a
            })),
            message_area,
        );
        frame.render_widget(self.text_input_bar.widget(), text_input_area);
    }
    async fn input_handling(&mut self, quit: &mut bool) -> Result<()> {
        match event::read()? {
            Event::Key(key) => match key.kind {
                KeyEventKind::Press => match self.focus {
                    Focus::MenuBar => match key.code {
                        KeyCode::Up => {
                            if let Some(index) = {
                                let a = self.menu_bar.lock().unwrap().items_state.selected();
                                a
                            } {
                                self.menu_bar
                                    .lock()
                                    .unwrap()
                                    .items_state
                                    .select(Some(index.saturating_sub(1)));
                            }
                        }
                        KeyCode::Down => {
                            if let Some(index) = {
                                let a = self.menu_bar.lock().unwrap().items_state.selected();
                                a
                            } {
                                let menu_bar_items_len = {
                                    let a = self.menu_bar.lock().unwrap().items.len();
                                    a
                                };
                                self.menu_bar.lock().unwrap().items_state.select(Some(
                                    index.saturating_add(1).clamp(0, menu_bar_items_len - 1),
                                ));
                            }
                        }
                        KeyCode::Enter => match {
                            let a = self.menu_bar.lock().unwrap().state;
                            a
                        } {
                            MenuBarState::MainMenu => {
                                if let Some(index) = {
                                    let a = self.menu_bar.lock().unwrap().items_state.selected();
                                    a
                                } {
                                    match index {
                                        0 => {
                                            self.accept_connect().await?;
                                        }
                                        1 => {
                                            let all_registered_node_name =
                                                self.get_all_registered_node_name().await?;
                                            if !all_registered_node_name.is_empty() {
                                                let mut menu_bar = self.menu_bar.lock().unwrap();
                                                menu_bar.state = MenuBarState::NodeListMenu;
                                                menu_bar.items = all_registered_node_name;
                                                menu_bar.items_state.select(Some(0));
                                            } else {
                                                self.message_bar
                                                    .lock()
                                                    .unwrap()
                                                    .items
                                                    .insert(0, "没有注册的节点".to_string());
                                            }
                                        }
                                        2 => *quit = true,
                                        _ => (),
                                    }
                                }
                            }
                            MenuBarState::WaitConnectMenu => {
                                if let Some(index) = {
                                    let a = self.menu_bar.lock().unwrap().items_state.selected();
                                    a
                                } {
                                    match index {
                                        0 => {
                                            let (mut send, _) =
                                                self.root_node_connection.open_bi().await?;
                                            send.write_all(&rmp_serde::to_vec(
                                                &DataPacket::UnRegisterNode,
                                            )?)
                                            .await?;
                                            send.finish().await?;
                                            let mut menu_bar = self.menu_bar.lock().unwrap();
                                            menu_bar.state = MenuBarState::MainMenu;
                                            menu_bar.items = menu_bar.state.to_menu_items();
                                            menu_bar.items_state.select(Some(0));
                                            if let Some(hole_punching_task) =
                                                &self.hole_punching_task
                                            {
                                                hole_punching_task.abort();
                                            }
                                            if let Some(recv_connect_task) = &self.recv_connect_task
                                            {
                                                recv_connect_task.abort();
                                            }
                                        }
                                        _ => (),
                                    }
                                }
                            }
                            MenuBarState::NodeListMenu => {
                                if let Some(index) = {
                                    let a = self.menu_bar.lock().unwrap().items_state.selected();
                                    a
                                } {
                                    self.connect({
                                        let a = self.menu_bar.lock().unwrap().items[index].clone();
                                        a
                                    })
                                    .await?;
                                }
                            }
                            MenuBarState::ChatMenu => {
                                if let Some(index) = {
                                    let a = self.menu_bar.lock().unwrap().items_state.selected();
                                    a
                                } {
                                    match index {
                                        0 => {
                                            if let Some(connection) =
                                                &*self.node_connection.lock().unwrap()
                                            {
                                                connection.close(
                                                    VarInt::from_u32(0),
                                                    "主动关闭连接".as_bytes(),
                                                );
                                            }
                                        }
                                        _ => (),
                                    }
                                }
                            }
                        },
                        KeyCode::Esc => match {
                            let a = self.menu_bar.lock().unwrap().state;
                            a
                        } {
                            MenuBarState::NodeListMenu => {
                                let mut menu_bar = self.menu_bar.lock().unwrap();
                                menu_bar.state = MenuBarState::MainMenu;
                                menu_bar.items = menu_bar.state.to_menu_items();
                                menu_bar.items_state.select(Some(0));
                            }
                            _ => (),
                        },
                        KeyCode::Tab => {
                            self.focus = Focus::MessageBar;
                            self.menu_bar.lock().unwrap().title_modifier = Modifier::default();
                            self.message_bar.lock().unwrap().title_modifier = Modifier::REVERSED;
                            if self.node_connection.lock().unwrap().is_some() {
                                self.text_input_bar
                                    .set_cursor_style(Style::default().bg(style::Color::Black));
                            }
                        }
                        _ => (),
                    },
                    Focus::MessageBar => {
                        match key.code {
                            KeyCode::Tab => {
                                self.focus = Focus::MenuBar;
                                self.message_bar.lock().unwrap().title_modifier =
                                    Modifier::default();
                                self.menu_bar.lock().unwrap().title_modifier = Modifier::REVERSED;
                                self.text_input_bar.set_cursor_style(Style::default());
                            }
                            _ => (),
                        }
                        if let Some(connection) = &*self.node_connection.lock().unwrap() {
                            match key.code {
                                KeyCode::Char(_) => {
                                    self.text_input_bar.input(key);
                                }
                                KeyCode::Backspace => {
                                    self.text_input_bar.delete_char();
                                }
                                KeyCode::Left => self.text_input_bar.move_cursor(CursorMove::Back),
                                KeyCode::Right => {
                                    self.text_input_bar.move_cursor(CursorMove::Forward)
                                }
                                KeyCode::Enter => {
                                    let mut input_text = String::new();
                                    for lines in self.text_input_bar.lines() {
                                        input_text.push_str(&lines);
                                    }
                                    if !input_text.is_empty() {
                                        self.message_bar.lock().unwrap().items.insert(
                                            0,
                                            format!("{}：{}", self.node_name, input_text),
                                        );
                                        self.text_input_bar.move_cursor(CursorMove::Head);
                                        self.text_input_bar.delete_line_by_end();
                                        let mut send = connection.open_uni().await?;
                                        send.write_all(input_text.as_bytes()).await?;
                                        send.finish().await?;
                                    }
                                }
                                _ => (),
                            }
                        }
                    }
                },
                _ => (),
            },
            _ => (),
        }
        Ok(())
    }
    async fn accept_connect(&mut self) -> Result<()> {
        //注册节点
        let (mut send, _) = self.root_node_connection.open_bi().await?;
        send.write_all(&rmp_serde::to_vec(&DataPacket::RegisterNode {
            name: self.node_name.clone(),
            cert: self.cert.clone(),
        })?)
        .await?;
        send.finish().await?;
        self.message_bar
            .lock()
            .unwrap()
            .items
            .insert(0, "等待连接...".to_string());
        let mut menu_bar = self.menu_bar.lock().unwrap();
        menu_bar.state = MenuBarState::WaitConnectMenu;
        menu_bar.items = menu_bar.state.to_menu_items();
        menu_bar.items_state.select(Some(0));
        //接收打洞信号
        self.hole_punching_task = Some(tokio::spawn({
            let endpoint = self.endpoint.clone();
            let root_node_connection = self.root_node_connection.clone();
            async move {
                let (mut send, mut recv) = root_node_connection.accept_bi().await?;
                match rmp_serde::from_slice::<DataPacket>(&recv.read_to_end(usize::MAX).await?)? {
                    DataPacket::Request(RequestDataPacket::HolePunching { ip_addr }) => {
                        let _ = endpoint.connect(ip_addr, "_")?.await;
                        send.write_all(&rmp_serde::to_vec(&DataPacket::Response(
                            ResponseDataPacket::HolePunching,
                        ))?)
                        .await?;
                        send.finish().await?;
                    }
                    _ => (),
                }
                eyre::Ok(())
            }
        }));
        //接收连接
        self.recv_connect_task = Some(tokio::spawn({
            let endpoint = self.endpoint.clone();
            let message_bar = self.message_bar.clone();
            let node_connection = self.node_connection.clone();
            let menu_bar = self.menu_bar.clone();
            let root_node_connection = self.root_node_connection.clone();
            async move {
                if let Some(connecting) = endpoint.accept().await {
                    let connection = connecting.await?;
                    let (mut send, _) = root_node_connection.open_bi().await?;
                    send.write_all(&rmp_serde::to_vec(&DataPacket::UnRegisterNode)?)
                        .await?;
                    send.finish().await?;
                    Self::connection_handling(connection, message_bar, node_connection, menu_bar);
                }
                eyre::Ok(())
            }
        }));
        Ok(())
    }
    async fn get_all_registered_node_name(&self) -> Result<Vec<String>> {
        //从根节点获取注册的节点
        let (mut send, mut recv) = self.root_node_connection.open_bi().await?;
        send.write_all(&rmp_serde::to_vec(&DataPacket::Request(
            RequestDataPacket::GetAllRegisteredNodeName,
        ))?)
        .await?;
        send.finish().await?;
        match rmp_serde::from_slice::<DataPacket>(&recv.read_to_end(usize::MAX).await?)? {
            DataPacket::Response(ResponseDataPacket::GetAllRegisteredNodeName {
                all_registered_node_name,
            }) => {
                return Ok(all_registered_node_name);
            }
            _ => (),
        }
        return Err(eyre!("从根节点获取全部注册的节点失败"));
    }
    async fn connect(&self, node_name: String) -> Result<()> {
        //通过根节点连接节点
        let (mut send, mut recv) = self.root_node_connection.open_bi().await?;
        send.write_all(&rmp_serde::to_vec(&DataPacket::Request(
            RequestDataPacket::GetRegisteredNodeIPAddrAndCert { node_name },
        ))?)
        .await?;
        send.finish().await?;
        match rmp_serde::from_slice::<DataPacket>(&recv.read_to_end(usize::MAX).await?)? {
            DataPacket::Response(ResponseDataPacket::GetRegisteredNodeIPAddrAndCert(Some(
                node_addr_and_cert,
            ))) => {
                //连接节点
                let mut node_cert_store = rustls::RootCertStore::empty();
                node_cert_store.add(&rustls::Certificate(node_addr_and_cert.cert.clone()))?;
                match self
                    .endpoint
                    .connect_with(
                        ClientConfig::with_root_certificates(node_cert_store),
                        node_addr_and_cert.ip_addr,
                        &x509_dns_name_from_der(&node_addr_and_cert.cert)?,
                    )?
                    .await
                {
                    Ok(connection) => Self::connection_handling(
                        connection,
                        self.message_bar.clone(),
                        self.node_connection.clone(),
                        self.menu_bar.clone(),
                    ),
                    Err(err) => {
                        self.message_bar
                            .lock()
                            .unwrap()
                            .items
                            .insert(0, format!("节点连接失败，原因：{}", err));
                        let mut menu_bar = self.menu_bar.lock().unwrap();
                        menu_bar.state = MenuBarState::MainMenu;
                        menu_bar.items = menu_bar.state.to_menu_items();
                        menu_bar.items_state.select(Some(0));
                    }
                };
            }
            _ => {
                self.message_bar
                    .lock()
                    .unwrap()
                    .items
                    .insert(0, "没有找到节点".to_string());
                let mut menu_bar = self.menu_bar.lock().unwrap();
                menu_bar.state = MenuBarState::MainMenu;
                menu_bar.items = menu_bar.state.to_menu_items();
                menu_bar.items_state.select(Some(0));
            }
        }
        Ok(())
    }
    fn connection_handling(
        connection: Connection,
        message_bar: Arc<Mutex<MessageBar>>,
        node_connection: Arc<Mutex<Option<Connection>>>,
        menu_bar: Arc<Mutex<MenuBar>>,
    ) {
        {
            message_bar
                .lock()
                .unwrap()
                .items
                .insert(0, format!("[{}]连接成功", connection.remote_address()));
        }
        tokio::spawn({
            let connection = connection.clone();
            let message_bar = message_bar.clone();
            let menu_bar = menu_bar.clone();
            async move {
                loop {
                    match connection.accept_uni().await {
                        Ok(mut recv) => {
                            let msg = recv.read_to_end(usize::MAX).await?;
                            message_bar.lock().unwrap().items.insert(
                                0,
                                format!(
                                    "{}：{}",
                                    connection.remote_address(),
                                    String::from_utf8(msg)?
                                ),
                            );
                        }
                        Err(err) => {
                            message_bar.lock().unwrap().items.insert(
                                0,
                                format!("[{}]断开连接，原因：{}", connection.remote_address(), err),
                            );
                            let mut menu_bar = menu_bar.lock().unwrap();
                            menu_bar.state = MenuBarState::MainMenu;
                            menu_bar.items = menu_bar.state.to_menu_items();
                            menu_bar.items_state.select(Some(0));
                            break;
                        }
                    }
                }
                eyre::Ok(())
            }
        });
        *node_connection.lock().unwrap() = Some(connection);
        let mut menu_bar = menu_bar.lock().unwrap();
        menu_bar.state = MenuBarState::ChatMenu;
        menu_bar.items = menu_bar.state.to_menu_items();
        menu_bar.items_state.select(Some(0));
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    App::new().await?.run().await?;
    Ok(())
}
