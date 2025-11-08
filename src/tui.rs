use crate::cli::Cli;
use crate::files::FileEntry;

use self::main_ui::MainUI;
use color_eyre::Report as Error;
use rat_salsa::event::RenderedEvent;
use rat_salsa::poll::{PollCrossterm, PollRendered, PollTasks, PollTimers};
use rat_salsa::timer::TimeOut;
use rat_salsa::{Control, RunConfig, SalsaAppContext, SalsaContext, run_tui};
use rat_theme3::{SalsaTheme, create_theme};
use rat_widget::event::{ConsumedEvent, Dialog, HandleEvent, Regular, ct_event};
use rat_widget::focus::FocusBuilder;
use rat_widget::layout::layout_middle;
use rat_widget::msgdialog::{MsgDialog, MsgDialogState};
use rat_widget::statusline::{StatusLine, StatusLineState};
use ratatui::buffer::Buffer;
use ratatui::crossterm;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::StatefulWidget;
use russh_sftp::client::SftpSession;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

pub fn tui(
    current_path: String,
    cli: Cli,
    rt: tokio::runtime::Runtime,
    sftp: Arc<SftpSession>,
) -> Result<(), Error> {
    let config = Config::new(cli);
    let theme = create_theme("Imperial Dark").expect("theme");
    let mut global = Global::new(config, theme);
    let mut state = Scenery::new(current_path, sftp);

    run_tui(
        init, //
        render,
        event,
        error,
        &mut global,
        &mut state,
        RunConfig::default()?
            .poll(PollCrossterm)
            .poll(PollTimers::default())
            .poll(PollTasks::default())
            .poll(PollRendered)
            .poll(rat_salsa::poll::PollTokio::new(rt)),
    )?;

    Ok(())
}

/// Globally accessible data/state.
#[allow(dead_code)]
pub struct Global {
    pub ctx: SalsaAppContext<AppEvent, Error>,
    pub cfg: Config,
    pub theme: Box<dyn SalsaTheme>,
}

impl SalsaContext<AppEvent, Error> for Global {
    fn set_salsa_ctx(&mut self, app_ctx: SalsaAppContext<AppEvent, Error>) {
        self.ctx = app_ctx;
    }

    fn salsa_ctx(&self) -> &SalsaAppContext<AppEvent, Error> {
        &self.ctx
    }
}

impl Global {
    pub fn new(cfg: Config, theme: Box<dyn SalsaTheme>) -> Self {
        Self {
            ctx: Default::default(),
            cfg,
            theme,
        }
    }
}

/// Configuration.
#[derive(Debug, Default)]
pub struct Config {
    pub(crate) cli: Cli,
}

impl Config {
    pub fn new(cli: Cli) -> Self {
        Self { cli }
    }
}

/// Application wide messages.
#[derive(Debug)]
pub enum AppEvent {
    Timer(TimeOut),
    Event(crossterm::event::Event),
    ChangeDir(String),
    UpdateCurrentPath(String),
    UpdateFiles(Vec<FileEntry>),
    DownloadFile(String, PathBuf),
    DownloadFolder(String, PathBuf),
    Rendered,
    Message(String),
    Status(usize, String),
    AsyncMsg(String),
    AsyncTick(u32),
}

impl From<RenderedEvent> for AppEvent {
    fn from(_: RenderedEvent) -> Self {
        Self::Rendered
    }
}

impl From<TimeOut> for AppEvent {
    fn from(value: TimeOut) -> Self {
        Self::Timer(value)
    }
}

impl From<crossterm::event::Event> for AppEvent {
    fn from(value: crossterm::event::Event) -> Self {
        Self::Event(value)
    }
}

// #[derive(Debug, Default)]
pub struct Scenery {
    pub async1: MainUI,
    pub status: StatusLineState,
    pub error_dlg: MsgDialogState,
}

impl Scenery {
    pub fn new(current_path: String, sftp: Arc<SftpSession>) -> Self {
        Self {
            async1: MainUI::new(current_path, sftp),
            status: StatusLineState::default(),
            error_dlg: MsgDialogState::default(),
        }
    }
}

pub fn render(
    area: Rect,
    buf: &mut Buffer,
    state: &mut Scenery,
    ctx: &mut Global,
) -> Result<(), Error> {
    let t0 = SystemTime::now();

    // forward

    let layout = Layout::vertical([
        Constraint::Fill(1), //
        Constraint::Length(1),
    ])
    .split(area);
    main_ui::render(layout[0], buf, &mut state.async1, ctx)?;

    if state.error_dlg.active() {
        MsgDialog::new()
            .styles(ctx.theme.msg_dialog_style())
            .render(
                layout_middle(
                    layout[0],
                    Constraint::Percentage(20),
                    Constraint::Percentage(20),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ),
                buf,
                &mut state.error_dlg,
            );
    }

    let el = t0.elapsed().unwrap_or(Duration::from_nanos(0));
    state.status.status(1, format!("R {:.0?}", el).to_string());

    let status_layout = Layout::horizontal([
        Constraint::Fill(61), //
        Constraint::Fill(39),
    ])
    .split(layout[1]);

    StatusLine::new()
        .layout([
            Constraint::Fill(1),
            Constraint::Length(8),
            Constraint::Length(8),
        ])
        .styles(ctx.theme.statusline_style())
        .render(layout[1], buf, &mut state.status);

    Ok(())
}

pub fn init(state: &mut Scenery, ctx: &mut Global) -> Result<(), Error> {
    ctx.set_focus(FocusBuilder::build_for(&state.async1));
    main_ui::init(&mut state.async1, ctx)?;
    Ok(())
}

pub fn event(
    event: &AppEvent,
    state: &mut Scenery,
    ctx: &mut Global,
) -> Result<Control<AppEvent>, Error> {
    let t0 = SystemTime::now();

    let mut r = match event {
        AppEvent::Event(event) => {
            let mut r = match &event {
                ct_event!(resized) => Control::Changed,
                ct_event!(key press CONTROL-'q') => Control::Quit,
                _ => Control::Continue,
            };

            r = r.or_else(|| {
                if state.error_dlg.active() {
                    state.error_dlg.handle(event, Dialog).into()
                } else {
                    Control::Continue
                }
            });

            let f = ctx.focus_mut().handle(event, Regular);
            ctx.queue(f);

            r
        }
        AppEvent::Rendered => {
            ctx.set_focus(FocusBuilder::rebuild_for(&state.async1, ctx.take_focus()));
            Control::Continue
        }
        AppEvent::Message(s) => {
            state.error_dlg.append(s);
            Control::Changed
        }
        AppEvent::Status(n, s) => {
            state.status.status(*n, s);
            Control::Changed
        }
        _ => Control::Continue,
    };

    r = r.or_else_try(|| main_ui::event(event, &mut state.async1, ctx))?;

    let el = t0.elapsed()?;
    state.status.status(2, format!("E {:.0?}", el).to_string());

    Ok(r)
}

pub fn error(
    event: Error,
    state: &mut Scenery,
    _ctx: &mut Global,
) -> Result<Control<AppEvent>, Error> {
    state.error_dlg.append(format!("{:?}", &*event).as_str());
    Ok(Control::Changed)
}

pub mod main_ui {
    use crate::files::FileDataSlice;
    use crate::files::FileEntry;
    use crate::ssh::Session;

    use super::AppEvent;
    use super::Global;
    use ::futures::executor::block_on;
    use color_eyre::Report as Error;
    use color_eyre::eyre::Result;
    use rat_focus::impl_has_focus;
    use rat_focus::match_focus;
    use rat_ftable::Table;
    use rat_ftable::TableState;
    use rat_ftable::event::ct_event;
    use rat_ftable::event::try_flow;
    //use rat_ftable::event::try_flow;
    use rat_ftable::selection::RowSelection;
    use rat_ftable::selection::rowselection;
    use rat_ftable::textdata::Cell;
    use rat_salsa::{Control, SalsaContext};
    use rat_widget::event::{HandleEvent, MenuOutcome, Regular};
    use rat_widget::menu::{MenuLine, MenuLineState};
    use rat_widget::paragraph::Paragraph;
    use rat_widget::paragraph::ParagraphState;
    use rat_widget::scrolled::Scroll;
    use rat_widget::text_input::TextInput;
    use rat_widget::text_input::TextInputState;
    use ratatui::buffer::Buffer;
    use ratatui::crossterm;
    use ratatui::layout::Flex;
    use ratatui::layout::Margin;
    use ratatui::layout::{Constraint, Direction, Layout, Rect};
    use ratatui::symbols::line::HORIZONTAL;
    use ratatui::symbols::line::HORIZONTAL_UP;
    use ratatui::symbols::line::ROUNDED_TOP_LEFT;
    use ratatui::text::Line;
    use ratatui::widgets::Block;
    use ratatui::widgets::BorderType;
    use ratatui::widgets::Borders;
    use ratatui::widgets::Padding;
    use ratatui::widgets::StatefulWidget;
    use ratatui::widgets::Widget;
    use ratatui::widgets::block;
    use russh_sftp::client::SftpSession;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;
    use tokio::sync::futures;
    use tracing::info;

    pub struct MainUI {
        pub current_path: String,
        pub table_state: TableState<RowSelection>,
        pub current_file_entries: Vec<FileEntry>,
        pub input_state: TextInputState,
        pub input_mode: InputMode,
        pub sftp: Arc<SftpSession>,
    }
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
    pub enum InputMode {
        #[default]
        Filter,
        DownloadPath,
    }

    impl MainUI {
        pub fn new(current_path: String, sftp: Arc<SftpSession>) -> Self {
            Self {
                current_path,
                table_state: TableState::default(),
                current_file_entries: Vec::new(),
                input_state: TextInputState::default(),
                input_mode: InputMode::default(),
                sftp,
            }
        }
    }

    pub fn render(
        area: Rect,
        buf: &mut Buffer,
        state: &mut MainUI,
        ctx: &mut Global,
    ) -> Result<(), Error> {
        // TODO: repaint_mask
        let r = Layout::new(
            Direction::Vertical,
            [
                Constraint::Length(3), //
                Constraint::Fill(1),   //
            ],
        )
        .split(area);

        let current_path = state.current_path.clone();
        let current_path_line = Line::from(current_path).style(ctx.theme.container_base());
        let current_path_line_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(ctx.theme.container_border())
            .title_top("Current Path");
        let current_path_line_area = r[0].inner(Margin::new(1, 1));
        current_path_line.render(current_path_line_area, buf);
        current_path_line_block.render(r[0], buf);

        let &[left, right] = Layout::new(
            Direction::Horizontal,
            [Constraint::Percentage(60), Constraint::Percentage(40)],
        )
        .split(r[1])
        .as_ref() else {
            unreachable!()
        };

        let &[left_top, left_bottom] = Layout::new(
            Direction::Vertical,
            [Constraint::Fill(1), Constraint::Length(3)],
        )
        .split(left)
        .as_ref() else {
            unreachable!()
        };

        let &[right_top, right_bottom] = Layout::new(
            Direction::Vertical,
            [Constraint::Percentage(50), Constraint::Percentage(50)],
        )
        .split(right)
        .as_ref() else {
            unreachable!()
        };

        if let Some(row) = state.table_state.selected() {
            let file = state.current_file_entries.get(row);
            if let Some(file) = file {
                let paragraph = Paragraph::from(file.clone())
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .title_top("Details")
                            .padding(Padding::uniform(1)),
                    )
                    .styles(ctx.theme.paragraph_style());
                paragraph.render(right_top, buf, &mut ParagraphState::default());
            }
        } else {
            Block::bordered()
                .border_type(BorderType::Rounded)
                .title_top("Details")
                .render(right_top, buf);
        }
        let input_block_title = match state.input_mode {
            InputMode::Filter => "Filter",
            InputMode::DownloadPath => "Download Path",
        };
        let input = TextInput::new().style(ctx.theme.container_base()).block(
            Block::bordered()

                .border_type(BorderType::Rounded)
                .border_style(match_focus!(state.input_state => ctx.theme.container_border().fg(ratatui::style::Color::Yellow), else => ctx.theme.container_border()))
                .title_top(input_block_title)
                .padding(Padding::horizontal(2)),
        );
        input.render(left_bottom, buf, &mut state.input_state);

        let table_style = ctx.theme.table_style();
        let table = Table::<RowSelection>::default()
            .block(
                Block::bordered()
                    .border_type(block::BorderType::Rounded)
                    .border_style(ctx.theme.container_border()),
            )
            .data(FileDataSlice(&state.current_file_entries))
            .widths([
                Constraint::Length(12),
                Constraint::Length(40),
                Constraint::Length(15),
                Constraint::Length(30),
            ])
            .column_spacing(1)
            .header(rat_ftable::textdata::Row::new([
                Cell::from("Permissions"),
                Cell::from(ROUNDED_TOP_LEFT.to_string() + &HORIZONTAL.repeat(3) + "Path"),
                Cell::from("Size"),
                Cell::from("Modified At"),
            ]))
            .vscroll(Scroll::new())
            .flex(Flex::Start)
            .styles(table_style);
        table.render(left_top, buf, &mut state.table_state);

        Ok(())
    }

    impl_has_focus!(table_state, input_state for MainUI);

    pub fn init(
        state: &mut MainUI, //
        ctx: &mut Global,
    ) -> Result<(), Error> {
        let path = state.current_path.clone();
        let cli = ctx.cfg.cli.clone();
        let _ = ctx.spawn_async_ext(|chan| async move {
            info!("connecting to {}:{}", cli.host, cli.port);
            let mut ssh = Session::connect(
                cli.private_key,
                cli.username.unwrap_or("root".to_string()),
                cli.openssh_certificate,
                (cli.host, cli.port),
            )
            .await?;
            info!("Connected");

            let sftp = ssh.sftp().await?;
            let files = sftp.read_dir(path.clone()).await?;
            let files = files.into_iter().map(FileEntry::from).collect::<Vec<_>>();
            let full_path = sftp.canonicalize(path).await?;
            sftp.close().await?;
            ssh.close().await?;
            chan.send(Ok(Control::Event(AppEvent::UpdateCurrentPath(full_path))))
                .await?;
            chan.send(Ok(Control::Event(AppEvent::UpdateFiles(files))))
                .await?;
            Ok(Control::Event(AppEvent::AsyncTick(300)))
        });
        // let files = block_on(state.sftp.read_dir(&state.current_path))?;
        // let files = files.into_iter().map(FileEntry::from).collect::<Vec<_>>();
        // state.current_file_entries = files;

        ctx.focus().first();
        Ok(())
    }

    pub fn event(
        event: &AppEvent,
        state: &mut MainUI,
        ctx: &mut Global,
    ) -> Result<Control<AppEvent>, Error> {
        let r = match event {
            AppEvent::Event(event) => {
                try_flow!(state.input_state.handle(event, Regular));
                try_flow!(match_focus!(
                    state.table_state => {
                    try_flow!(
                        rowselection::handle_events(
                            &mut state.table_state,
                            true,
                            event
                        )
                    );
                        match event {
                            ct_event!(key press 'j') => {
                                state.table_state.move_down(1);
                                Control::<AppEvent>::Changed
                            }
                            ct_event!(key press 'k') => {
                                state.table_state.move_up(1);
                                Control::Changed
                            }
                            ct_event!(keycode press Left ) | ct_event!(key press 'h')=> {

                                let path = PathBuf::from(state.current_path.clone());
                                let parent = path.parent();
                                if let Some(parent) = parent {
                                    let parent = parent.display();
                                    state.current_path = parent.to_string();

                                    Control::Event(AppEvent::ChangeDir(parent.to_string()))
                                } else {
                                    Control::Continue
                                }
                            }

                            ct_event!(key press 'l') | ct_event!(keycode press Right) => {
                                let path = PathBuf::from(state.current_path.clone());
                                let selected = state.table_state.selected();
                                if let Some(selected) = selected {
                                    let Some(file) = state.current_file_entries.get(selected) else {
                                        return Ok(Control::Continue);
                                    };
                                    if file.is_dir() {
                                        let path = path.join(file.name());
                                        state.current_path = path.display().to_string();
                                        return Ok(Control::Event(AppEvent::ChangeDir(path.display().to_string())));
                                    }
                                }
                                Control::Continue
                            }
                            ct_event!(key press 'd') => {
                                state.input_mode = InputMode::DownloadPath;
                                ctx.focus().focus(&state.input_state);

                                Control::Changed
                            }
                            _ => Control::Continue
                        }
                    },
                    state.input_state => {
                        if state.input_mode == InputMode::DownloadPath {
                            match event {
                                ct_event!(keycode press Enter) => {
                                    let path:String = state.input_state.value();
                                    let path = PathBuf::from(path);
                                    let selected = state.table_state.selected();
                                    if let Some(selected) = selected {
                                        let Some(file) = state.current_file_entries.get(selected) else {
                                            return Ok(Control::Continue);
                                        };
                                        let name = state.current_path.clone() + "/" + file.name();
                                        if file.is_dir() {
                                            return Ok(Control::Event(AppEvent::DownloadFolder(name, path)));
                                        }
                                        return Ok(Control::Event(AppEvent::DownloadFile(name, path)));
                                    }
                                }
                                _ => {}
                            }
                        }
                        Control::Continue
                    },

                    else => Control::Continue
                ));
                Control::Continue
            }
            AppEvent::AsyncMsg(s) => {
                // receive result from async operation
                Control::Event(AppEvent::Message(s.clone()))
            }
            AppEvent::DownloadFile(name, path) => {
                let sftp = Arc::clone(&state.sftp);
                let path = path.clone();
                let name = name.clone();
                ctx.spawn_async_ext(|chan| async move {
                    let mut remote_file = sftp.open(name.clone()).await?;
                    let mut buf = Vec::new();
                    remote_file.read_to_end(&mut buf).await?;
                    let mut file = tokio::fs::File::create(path.join(name)).await?;
                    let _ = file.write(&buf).await?;
                    file.flush().await?;
                    file.sync_all().await?;
                    Ok(Control::Event(AppEvent::AsyncTick(300)))
                });
                Control::Continue
            }
            AppEvent::DownloadFolder(file, path) => {
                let sftp = Arc::clone(&state.sftp);
                let path = path.clone();
                let file = file.clone();
                ctx.spawn_async_ext(|_| async move {
                    let read_dir = sftp.read_dir(file.clone()).await?;
                    let files = read_dir
                        .into_iter()
                        .map(FileEntry::from)
                        .collect::<Vec<_>>();
                    Ok(Control::Event(AppEvent::AsyncTick(300)))
                });
                Control::Continue
            }
            AppEvent::ChangeDir(path) => {
                let path = if !path.is_empty() {
                    path.clone()
                } else {
                    ".".to_string()
                };
                info!("changing dir to {}", path);
                let sftp = Arc::clone(&state.sftp);

                ctx.spawn_async_ext(|chan| async move {
                    let files = sftp.read_dir(path).await?;
                    let files = files.into_iter().map(FileEntry::from).collect::<Vec<_>>();
                    chan.send(Ok(Control::Event(AppEvent::UpdateFiles(files))))
                        .await?;

                    Ok(Control::Event(AppEvent::AsyncTick(300)))
                });
                Control::Continue
            }
            AppEvent::UpdateFiles(files) => {
                state.current_file_entries = files.to_vec();
                Control::Changed
            }
            AppEvent::UpdateCurrentPath(path) => {
                state.current_path = path.clone();
                Control::Continue
            }
            _ => Control::Continue,
        };

        Ok(r)
    }
}
