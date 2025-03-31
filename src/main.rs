// src/main.rs
use iced::{
    alignment, executor, Application, Command, Container, Element, Length, Settings, 
    Subscription, Theme, Color, Background, Text, Row, Column, Button, Scrollable, Space,
    alignment::Horizontal, window
};
use iced::widget::{button, column, container, row, scrollable, text};
use once_cell::sync::Lazy;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::sync::{Arc, Mutex};
use std::fs;
use chrono::Local;
use rfd::FileDialog;

mod csv_handler;
mod cloud_handler;
mod data_types;
mod ui;

use csv_handler::CSVHandler;
use cloud_handler::CloudHandler;
use data_types::{TableData, DataSource};
use ui::{Styles, DARK_THEME, LIGHT_THEME};

const VERSION: &str = "2.0.0-pre1";
const UPDATE_INTERVAL: Duration = Duration::from_secs(5);

// Static application state
static THEME: Lazy<Arc<Mutex<Styles>>> = Lazy::new(|| {
    Arc::new(Mutex::new(DARK_THEME.clone()))
});

pub fn main() -> iced::Result {
    ScoreViewer::run(Settings {
        window: window::Settings {
            size: (1024, 768),
            resizable: true,
            decorations: false, // For fullscreen-like appearance
            ..Default::default()
        },
        ..Settings::default()
    })
}

struct ScoreViewer {
    theme: Arc<Mutex<Styles>>,
    is_dark_mode: bool,
    data_source: Option<DataSource>,
    file_path: Option<PathBuf>,
    spreadsheet_url: String,
    sheet_name: String,
    last_data: Option<TableData>,
    last_check: Instant,
    last_modified: Option<std::time::SystemTime>,
    cloud_dialog_open: bool,
    cloud_url_input: String,
    cloud_sheet_input: String,
    result_column_index: Option<usize>,
    scroll_state: scrollable::State,
}

#[derive(Debug, Clone)]
enum Message {
    ToggleTheme,
    OpenLocalFile,
    FileSelected(Option<PathBuf>),
    ShowCloudDialog,
    CloseCloudDialog,
    ConnectToCloud,
    UpdateCloudUrl(String),
    UpdateSheetName(String),
    DataUpdated(TableData),
    CheckForUpdates,
    Exit,
}

impl Application for ScoreViewer {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            ScoreViewer {
                theme: THEME.clone(),
                is_dark_mode: true,
                data_source: None,
                file_path: None,
                spreadsheet_url: String::new(),
                sheet_name: String::new(),
                last_data: None,
                last_check: Instant::now(),
                last_modified: None,
                cloud_dialog_open: false,
                cloud_url_input: String::new(),
                cloud_sheet_input: String::new(),
                result_column_index: None,
                scroll_state: scrollable::State::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        format!("Score Viewer v{}", VERSION)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ToggleTheme => {
                self.is_dark_mode = !self.is_dark_mode;
                let mut theme = self.theme.lock().unwrap();
                *theme = if self.is_dark_mode {
                    DARK_THEME.clone()
                } else {
                    LIGHT_THEME.clone()
                };
                Command::none()
            }
            
            Message::OpenLocalFile => {
                Command::perform(
                    async {
                        let file = FileDialog::new()
                            .add_filter("CSV Files", &["csv"])
                            .pick_file();
                        file
                    },
                    Message::FileSelected,
                )
            }
            
            Message::FileSelected(path_opt) => {
                if let Some(path) = path_opt {
                    self.file_path = Some(path.clone());
                    self.data_source = Some(DataSource::Local(path.clone()));
                    self.last_modified = fs::metadata(&path).ok().map(|m| m.modified().unwrap_or_else(|_| std::time::SystemTime::now()));
                    
                    return Command::perform(
                        async move {
                            let csv_handler = CSVHandler::new();
                            csv_handler.read_csv(&path).await
                        },
                        Message::DataUpdated
                    );
                }
                Command::none()
            }
            
            Message::ShowCloudDialog => {
                self.cloud_dialog_open = true;
                Command::none()
            }
            
            Message::CloseCloudDialog => {
                self.cloud_dialog_open = false;
                Command::none()
            }
            
            Message::UpdateCloudUrl(url) => {
                self.cloud_url_input = url;
                Command::none()
            }
            
            Message::UpdateSheetName(name) => {
                self.cloud_sheet_input = name;
                Command::none()
            }
            
            Message::ConnectToCloud => {
                if !self.cloud_url_input.is_empty() {
                    self.spreadsheet_url = self.cloud_url_input.clone();
                    self.sheet_name = self.cloud_sheet_input.clone();
                    self.data_source = Some(DataSource::Cloud(
                        self.spreadsheet_url.clone(), 
                        self.sheet_name.clone()
                    ));
                    self.cloud_dialog_open = false;
                    
                    let url = self.spreadsheet_url.clone();
                    let sheet = self.sheet_name.clone();
                    
                    return Command::perform(
                        async move {
                            let cloud_handler = CloudHandler::new();
                            cloud_handler.fetch_data(&url, &sheet).await
                                .unwrap_or_else(|_| TableData::empty())
                        },
                        Message::DataUpdated
                    );
                }
                Command::none()
            }
            
            Message::DataUpdated(data) => {
                self.last_data = Some(data);
                // Find result column index
                if let Some(ref data) = self.last_data {
                    if !data.headers.is_empty() {
                        self.result_column_index = data.headers
                            .iter()
                            .position(|h| h.to_lowercase() == "result");
                    }
                }
                Command::none()
            }
            
            Message::CheckForUpdates => {
                if Instant::now().duration_since(self.last_check) >= UPDATE_INTERVAL {
                    self.last_check = Instant::now();
                    
                    match &self.data_source {
                        Some(DataSource::Local(path)) => {
                            if let Ok(metadata) = fs::metadata(path) {
                                if let Ok(modified) = metadata.modified() {
                                    if let Some(last_modified) = self.last_modified {
                                        if modified > last_modified {
                                            self.last_modified = Some(modified);
                                            let path_clone = path.clone();
                                            
                                            return Command::perform(
                                                async move {
                                                    let csv_handler = CSVHandler::new();
                                                    csv_handler.read_csv(&path_clone).await
                                                },
                                                Message::DataUpdated
                                            );
                                        }
                                    } else {
                                        self.last_modified = Some(modified);
                                    }
                                }
                            }
                        }
                        
                        Some(DataSource::Cloud(url, sheet)) => {
                            let url_clone = url.clone();
                            let sheet_clone = sheet.clone();
                            
                            return Command::perform(
                                async move {
                                    let cloud_handler = CloudHandler::new();
                                    cloud_handler.fetch_data(&url_clone, &sheet_clone).await
                                        .unwrap_or_else(|_| TableData::empty())
                                },
                                Message::DataUpdated
                            );
                        }
                        
                        None => {}
                    }
                }
                Command::none()
            }
            
            Message::Exit => {
                // Exit the application
                std::process::exit(0);
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        // Create a subscription that emits a CheckForUpdates message every second
        iced::time::every(Duration::from_secs(1))
            .map(|_| Message::CheckForUpdates)
    }

    fn view(&self) -> Element<Message> {
        let theme = self.theme.lock().unwrap();
        
        // Main content area with table
        let content = if let Some(ref data) = self.last_data {
            self.render_table(data, &theme)
        } else {
            container(
                text("No data loaded. Please select a local file or connect to Google Sheets.")
                    .size(24)
                    .color(theme.fg)
                    .horizontal_alignment(Horizontal::Center)
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .style(iced::theme::Container::Custom(Box::new(ContainerStyle { 
                bg: theme.bg,
            })))
            .into()
        };
        
        // Footer with buttons
        let footer = row![
            text(format!("Score Viewer © 2024-2025 Joona Holkko. All rights reserved. v{}", VERSION))
                .size(14)
                .color(theme.footer_fg),
            Space::with_width(Length::Fill),
            button(
                text("☁ Cloud")
                    .horizontal_alignment(Horizontal::Center)
                    .size(16)
                    .color(theme.footer_fg)
            )
            .on_press(Message::ShowCloudDialog)
            .style(iced::theme::Button::Custom(Box::new(ButtonStyle {
                bg: theme.footer_bg,
                fg: theme.footer_fg,
                hover_bg: Color::from_rgb(0.0, 0.26, 0.5),
            }))),
            Space::with_width(Length::Units(10)),
            button(
                text("📁 Local")
                    .horizontal_alignment(Horizontal::Center)
                    .size(16)
                    .color(theme.footer_fg)
            )
            .on_press(Message::OpenLocalFile)
            .style(iced::theme::Button::Custom(Box::new(ButtonStyle {
                bg: theme.footer_bg,
                fg: theme.footer_fg,
                hover_bg: Color::from_rgb(0.0, 0.26, 0.5),
            }))),
            Space::with_width(Length::Units(10)),
            button(
                text("💡")
                    .horizontal_alignment(Horizontal::Center)
                    .size(18)
                    .color(theme.footer_fg)
            )
            .on_press(Message::ToggleTheme)
            .style(iced::theme::Button::Custom(Box::new(ButtonStyle {
                bg: theme.footer_bg,
                fg: theme.footer_fg,
                hover_bg: Color::from_rgb(0.0, 0.26, 0.5),
            })))
        ]
        .spacing(5)
        .padding(10)
        .width(Length::Fill)
        .height(Length::Units(50))
        .style(iced::theme::Container::Custom(Box::new(ContainerStyle { 
            bg: theme.footer_bg,
        })));
        
        // Combine the main content and footer
        let main_content = column![
            content,
            footer
        ];
        
        // Overlay for cloud connection dialog
        if self.cloud_dialog_open {
            return self.cloud_dialog_view(&theme);
        }
        
        container(main_content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(iced::theme::Container::Custom(Box::new(ContainerStyle { 
                bg: theme.bg,
            })))
            .into()
    }
}

impl ScoreViewer {
    fn render_table(&self, data: &TableData, theme: &Styles) -> Element<Message> {
        let headers = Row::with_children(
            data.headers.iter().enumerate().map(|(i, header)| {
                container(
                    text(header)
                        .size(18)
                        .color(theme.header_fg)
                )
                .width(if i == 0 { Length::Units(150) } else { Length::Units(100) })
                .padding(5)
                .style(iced::theme::Container::Custom(Box::new(ContainerStyle { 
                    bg: theme.header_bg,
                })))
                .into()
            })
            .collect()
        )
        .spacing(1);
        
        let rows = data.rows.iter().map(|row| {
            Row::with_children(
                row.iter().enumerate().map(|(i, cell)| {
                    let is_result_column = self.result_column_index.map_or(false, |idx| idx == i);
                    
                    container(
                        text(cell)
                            .size(18)
                            .color(theme.fg)
                            .style(if is_result_column {
                                iced::theme::Text::Default
                            } else {
                                iced::theme::Text::Default
                            })
                    )
                    .width(if i == 0 { Length::Units(150) } else { Length::Units(100) })
                    .padding(5)
                    .style(iced::theme::Container::Custom(Box::new(ContainerStyle { 
                        bg: theme.bg,
                    })))
                    .into()
                })
                .collect()
            )
            .spacing(1)
        });
        
        let content = column![headers]
            .push(Column::with_children(rows.collect()))
            .spacing(1);
        
        scrollable(content)
            .height(Length::Fill)
            .into()
    }
    
    fn cloud_dialog_view(&self, theme: &Styles) -> Element<Message> {
        let dialog_content = column![
            text("Connect to Google Sheet")
                .size(24)
                .color(theme.fg),
            Space::with_height(Length::Units(20)),
            text("Google Sheet URL:")
                .size(16)
                .color(theme.fg),
            iced::widget::text_input(&self.cloud_url_input, "Enter Google Sheet URL")
                .padding(10)
                .width(Length::Units(400))
                .on_input(Message::UpdateCloudUrl),
            Space::with_height(Length::Units(10)),
            text("Sheet Name (optional):")
                .size(16)
                .color(theme.fg),
            iced::widget::text_input(&self.cloud_sheet_input, "Enter Sheet Name")
                .padding(10)
                .width(Length::Units(400))
                .on_input(Message::UpdateSheetName),
            Space::with_height(Length::Units(20)),
            row![
                button(text("Connect").size(16))
                    .on_press(Message::ConnectToCloud)
                    .padding(10)
                    .width(Length::Units(100)),
                Space::with_width(Length::Units(20)),
                button(text("Cancel").size(16))
                    .on_press(Message::CloseCloudDialog)
                    .padding(10)
                    .width(Length::Units(100))
            ]
        ]
        .spacing(10)
        .padding(20)
        .width(Length::Units(450))
        .height(Length::Units(300))
        .style(iced::theme::Container::Custom(Box::new(ContainerStyle { 
            bg: theme.bg,
        })));
        
        let dialog = container(dialog_content)
            .width(Length::Units(450))
            .height(Length::Units(300))
            .center_x()
            .center_y()
            .style(iced::theme::Container::Custom(Box::new(ContainerStyle { 
                bg: theme.bg,
            })));
        
        // Overlay dialog on top of dimmed background
        container(dialog)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x()
            .center_y()
            .style(iced::theme::Container::Custom(Box::new(OverlayStyle {})))
            .into()
    }
}

// Custom styles for containers and buttons
struct ContainerStyle {
    bg: Color,
}

impl container::StyleSheet for ContainerStyle {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(Background::Color(self.bg)),
            border_radius: 0.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            text_color: None,
        }
    }
}

struct OverlayStyle;

impl container::StyleSheet for OverlayStyle {
    fn style(&self) -> container::Style {
        container::Style {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.7))),
            border_radius: 0.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            text_color: None,
        }
    }
}

struct ButtonStyle {
    bg: Color,
    fg: Color,
    hover_bg: Color,
}

impl button::StyleSheet for ButtonStyle {
    fn active(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(self.bg)),
            border_radius: 4.0,
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            text_color: self.fg,
            shadow_offset: iced::Vector::new(0.0, 0.0),
            ..button::Style::default()
        }
    }

    fn hovered(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(self.hover_bg)),
            ..self.active()
        }
    }
}
