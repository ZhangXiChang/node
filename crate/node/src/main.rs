use std::{io::stdout, time::Duration};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout},
    style::{Modifier, Style, Stylize},
    widgets::{Block, Borders, List, ListDirection, ListState, Paragraph},
    Terminal,
};
use serde::{Deserialize, Serialize};

enum Focus {
    MenuBar,
    ChatBar,
}

enum MenuBarState {
    MainMenu,
}
impl Default for MenuBarState {
    fn default() -> Self {
        Self::MainMenu
    }
}

#[derive(Serialize, Deserialize)]
struct RootNodeConfig {
    ip_addr: String,
    dns_name: String,
    cert_file_name: String,
    default_connect: bool,
}
#[derive(Serialize, Deserialize)]
struct Config {
    you_name: String,
    root_node_list: Vec<RootNodeConfig>,
}

#[derive(Default)]
struct TitleBar {
    title: String,
}
#[derive(Default)]
struct MenuBar {
    title: String,
    title_modifier: Modifier,
    items: Vec<String>,
    state: MenuBarState,
    items_state: ListState,
}
#[derive(Default)]
struct MessageBar {
    title: String,
    title_modifier: Modifier,
    items: Vec<String>,
}
struct App {
    focus: Focus,
    title_bar: TitleBar,
    menu_bar: MenuBar,
    message_bar: MessageBar,
}
impl App {
    fn new() -> Self {
        Self {
            focus: Focus::MenuBar,
            title_bar: TitleBar {
                title: "欢迎使用节点网络，根节点[北方通信]为您服务".to_string(),
            },
            menu_bar: MenuBar {
                title: "主菜单".to_string(),
                title_modifier: Modifier::REVERSED,
                items_state: {
                    let mut list_state = ListState::default();
                    list_state.select(Some(0));
                    list_state
                },
                ..Default::default()
            },
            message_bar: MessageBar {
                title: "消息栏".to_string(),
                ..Default::default()
            },
        }
    }
    fn run(mut self) -> Result<()> {
        //终端界面
        stdout().execute(EnterAlternateScreen)?;
        enable_raw_mode()?;
        let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
        let mut quit = false;
        while !quit {
            match self.menu_bar.state {
                MenuBarState::MainMenu => {
                    self.menu_bar.items = vec![
                        "接收连接".to_string(),
                        "主动连接".to_string(),
                        "退出程序".to_string(),
                    ];
                }
            }
            terminal.draw(|frame| {
                let [title_area, menu_area] =
                    Layout::vertical([Constraint::Length(3), Constraint::Min(0)])
                        .areas(frame.size());
                frame.render_widget(
                    Paragraph::new(self.title_bar.title.clone())
                        .block(Block::new().borders(Borders::ALL))
                        .alignment(Alignment::Center),
                    title_area,
                );
                let [menu_area, chat_area] =
                    Layout::horizontal([Constraint::Length(20), Constraint::Min(0)])
                        .areas(menu_area);
                frame.render_stateful_widget(
                    List::new(self.menu_bar.items.clone())
                        .block(
                            Block::new().borders(Borders::ALL).title(
                                self.menu_bar
                                    .title
                                    .clone()
                                    .add_modifier(self.menu_bar.title_modifier),
                            ),
                        )
                        .highlight_style(Style::new().add_modifier(Modifier::BOLD))
                        .highlight_symbol(">> "),
                    menu_area,
                    &mut self.menu_bar.items_state,
                );
                frame.render_widget(
                    List::new(self.message_bar.items.clone())
                        .direction(ListDirection::BottomToTop)
                        .block(
                            Block::new().borders(Borders::ALL).title(
                                self.message_bar
                                    .title
                                    .clone()
                                    .add_modifier(self.message_bar.title_modifier),
                            ),
                        ),
                    chat_area,
                );
            })?;
            if event::poll(Duration::ZERO)? {
                match event::read()? {
                    Event::Key(key) => match key.kind {
                        KeyEventKind::Press => match self.focus {
                            Focus::MenuBar => match key.code {
                                KeyCode::Up => {
                                    if let Some(index) = self.menu_bar.items_state.selected() {
                                        self.menu_bar
                                            .items_state
                                            .select(Some(index.saturating_sub(1)));
                                    }
                                }
                                KeyCode::Down => {
                                    if let Some(index) = self.menu_bar.items_state.selected() {
                                        self.menu_bar.items_state.select(Some(
                                            index
                                                .saturating_add(1)
                                                .clamp(0, self.menu_bar.items.len() - 1),
                                        ));
                                    }
                                }
                                KeyCode::Enter => {
                                    if let Some(index) = self.menu_bar.items_state.selected() {
                                        match index {
                                            0 => (),
                                            1 => (),
                                            2 => quit = true,
                                            _ => (),
                                        }
                                    }
                                }
                                KeyCode::Tab => {
                                    self.focus = Focus::ChatBar;
                                    self.menu_bar.title_modifier = Modifier::default();
                                    self.message_bar.title_modifier = Modifier::REVERSED;
                                }
                                _ => (),
                            },
                            Focus::ChatBar => match key.code {
                                KeyCode::Tab => {
                                    self.focus = Focus::MenuBar;
                                    self.message_bar.title_modifier = Modifier::default();
                                    self.menu_bar.title_modifier = Modifier::REVERSED;
                                }
                                _ => (),
                            },
                        },
                        _ => (),
                    },
                    _ => (),
                }
            }
        }
        disable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    App::new().run()?;
    Ok(())
    // //设置路径
    // let config_file_path = PathBuf::from("./config.json");
    // let cert_dir_path = PathBuf::from("./certs");
    // //解析配置文件
    // let mut config = Config {
    //     you_name: "无名氏".to_string(),
    //     root_node_list: vec![RootNodeConfig {
    //         ip_addr: "47.122.9.167:10270".to_string(),
    //         dns_name: "root_node".to_string(),
    //         cert_file_name: "root_node".to_string(),
    //         default_connect: true,
    //     }],
    // };
    // match File::open(config_file_path.clone()) {
    //     Ok(mut config_file) => {
    //         let mut config_bytes = Vec::new();
    //         config_file.read_to_end(&mut config_bytes)?;
    //         config = serde_json::from_slice(&config_bytes)?;
    //     }
    //     Err(_) => {
    //         config.serialize(&mut serde_json::Serializer::with_formatter(
    //             File::create(config_file_path)?,
    //             serde_json::ser::PrettyFormatter::with_indent(b"    "),
    //         ))?;
    //     }
    // }
    // //创建证书
    // let certificate =
    //     rcgen::Certificate::from_params(rcgen::CertificateParams::new(vec!["node".to_string()]))?;
    // //创建节点
    // let mut transport_config = TransportConfig::default();
    // transport_config.keep_alive_interval(Some(Duration::from_secs(5)));
    // let mut endpoint = Endpoint::server(
    //     ServerConfig::with_single_cert(
    //         vec![rustls::Certificate(certificate.serialize_der()?)],
    //         rustls::PrivateKey(certificate.serialize_private_key_der()),
    //     )?
    //     .transport_config(Arc::new(transport_config))
    //     .clone(),
    //     "0.0.0.0:0".parse()?,
    // )?;
    // //加载根节点证书设置为默认信任证书
    // let mut default_connect_root_node = None;
    // let mut root_node_cert_store = rustls::RootCertStore::empty();
    // for root_node_config in config.root_node_list {
    //     let mut root_node_cert = Vec::new();
    //     match File::open(cert_dir_path.join(root_node_config.cert_file_name.clone() + ".cer")) {
    //         Ok(mut cert_file) => {
    //             cert_file.read_to_end(&mut root_node_cert)?;
    //         }
    //         Err(_) => {
    //             return Err(anyhow!(
    //                 "证书[{}]不存在",
    //                 root_node_config.cert_file_name.clone()
    //             ))
    //         }
    //     }
    //     root_node_cert_store.add(&rustls::Certificate(root_node_cert))?;
    //     if root_node_config.default_connect && default_connect_root_node.is_none() {
    //         default_connect_root_node = Some(root_node_config);
    //     }
    // }
    // endpoint.set_default_client_config(ClientConfig::with_root_certificates(root_node_cert_store));
    // //连接根节点
    // let root_node_connection;
    // if let Some(root_node_config) = default_connect_root_node {
    //     root_node_connection = endpoint
    //         .connect(
    //             root_node_config.ip_addr.parse()?,
    //             &root_node_config.dns_name,
    //         )?
    //         .await?;
    // } else {
    //     return Err(anyhow!("没有默认连接的根节点"));
    // }
}
