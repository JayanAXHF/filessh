use self::main_ui::MainUI;
use crate::cli::ResolvedConnectArgs;
use crate::config::Theme;
use crate::files::FileEntry;
use crate::ssh::Session;
use async_lock::Mutex;
use color_eyre::Report as Error;
use color_eyre::eyre::Result;
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
use ratatui::crossterm::cursor::Show;
use ratatui::crossterm::terminal::{LeaveAlternateScreen, disable_raw_mode};
use ratatui::crossterm::{self, ExecutableCommand};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::widgets::StatefulWidget;
use russh_sftp::client::SftpSession;
use std::io::stdout;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tracing::error;
pub mod main_ui;

pub fn tui(
    current_path: String,
    cli: ResolvedConnectArgs,
    rt: tokio::runtime::Runtime,
    sftp: Arc<SftpSession>,
    session: Arc<Mutex<Session>>,
    theme: &Theme,
) -> Result<(), Error> {
    let config = Config::new(cli);
    let theme = match theme {
        Theme::Custom(c) => c.clone().into(),
        Theme::Default(d) => create_theme(&d.to_string()).expect("theme"),
    };
    let mut global = Global::new(config, theme);
    let mut state = Scenery::new(current_path, sftp, session);

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
    pub(crate) cli: ResolvedConnectArgs,
}

impl Config {
    pub fn new(cli: ResolvedConnectArgs) -> Self {
        Self { cli }
    }
}

/// Application wide messages.
#[derive(Debug)]
#[allow(dead_code)]
pub enum AppEvent {
    Timer(TimeOut),
    Event(crossterm::event::Event),
    ChangeDir(String),
    DownloadStart,
    DownloadEnd,
    UpdateCurrentPath(String),
    Throb,
    Gauge(f64),
    SetTotalFilesToDownload(usize),
    UpdateContent(Option<String>),
    UpdateFiles(Vec<FileEntry>),
    SpawnExternalEditor(String),
    SpawnSSHCommand,
    DownloadFile(String, PathBuf, Option<String>),
    DownloadFolder(String, PathBuf),
    DeleteEntry(FileEntry),
    MoveEntry(String, String),
    Rendered,
    Message(String),
    Status(usize, String),
    AsyncMsg(String),
    UpdateNextFiveFiles(Vec<FileEntry>),
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
    pub fn new(current_path: String, sftp: Arc<SftpSession>, session: Arc<Mutex<Session>>) -> Self {
        Self {
            async1: MainUI::new(current_path, sftp, session),
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
    let remote_host_details = format!(
        "  Conntected to {}@{}:{}  ",
        ctx.cfg.cli.username.as_ref().map_or("root", |s| s.as_str()),
        ctx.cfg.cli.host.as_str(),
        ctx.cfg.cli.port
    );
    let len = remote_host_details.len();
    state.status.status(3, remote_host_details);

    StatusLine::new()
        .layout([
            Constraint::Fill(1),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(len as u16),
        ])
        .styles_ext(ctx.theme.statusline_style_ext())
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
    if state.async1.in_editor {
        return Ok(Control::Changed);
    }

    let mut r = match event {
        AppEvent::Event(event) => {
            let mut r = match &event {
                ct_event!(resized) => Control::Changed,
                ct_event!(key press CONTROL-'q') => {
                    if let Some(cancel) = state.async1.throbber_cancel.take() {
                        cancel.cancel();
                    }
                    let session = Arc::clone(&state.async1.session);
                    ctx.spawn_async_ext(async move |_| {
                        let mut session = session.lock().await;
                        session.close().await?;
                        Ok(Control::Quit)
                    });
                    Control::Quit
                }
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
    error!("{:?}", &*event);
    state.error_dlg.append(format!("{:?}", &*event).as_str());
    Ok(Control::Changed)
}

impl Drop for Scenery {
    fn drop(&mut self) {
        if let Some(cancel) = self.async1.throbber_cancel.take() {
            cancel.cancel();
        }
        disable_raw_mode().unwrap();
        stdout().execute(LeaveAlternateScreen).unwrap();
        stdout().execute(Show).unwrap();
    }
}
