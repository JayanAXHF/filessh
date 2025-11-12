use crate::files::FileDataSlice;
use crate::files::FileEntry;
use crate::files::JoinablePaths;
use crate::files::ProgressDataSlice;
use crate::par_dir_traversal::WalkParallel;
use crate::par_dir_traversal::WalkState;
use crate::patched_line_gauge::LineGauge;
use crate::ssh::Session;

use super::AppEvent;
use super::Global;

use color_eyre::Report as Error;
use color_eyre::eyre;
use color_eyre::eyre::Result;
use rat_focus::impl_has_focus;
use rat_focus::match_focus;
use rat_ftable::Table;
use rat_ftable::TableState;
use rat_ftable::event::ct_event;
use rat_ftable::event::try_flow;
use rat_ftable::selection::NoSelection;
//use rat_ftable::event::try_flow;
use async_lock::Mutex as AsyncMutex;
use rat_ftable::selection::RowSelection;
use rat_ftable::selection::rowselection;
use rat_ftable::textdata::Cell;

use rat_salsa::tasks::Cancel;
use rat_salsa::{Control, SalsaContext};
use rat_widget::event::TextOutcome;
use rat_widget::event::{HandleEvent, Regular};
use rat_widget::paragraph::Paragraph;
use rat_widget::paragraph::ParagraphState;
use rat_widget::scrolled::Scroll;
use rat_widget::text_input::TextInput;
use rat_widget::text_input::TextInputState;
use ratatui::buffer::Buffer;
use ratatui::crossterm;
use ratatui::crossterm::ExecutableCommand;
use ratatui::crossterm::terminal::EnterAlternateScreen;
use ratatui::crossterm::terminal::disable_raw_mode;
use ratatui::crossterm::terminal::enable_raw_mode;
use ratatui::layout::Flex;
use ratatui::layout::Margin;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::symbols;
use ratatui::symbols::line::HORIZONTAL;

use ratatui::symbols::line::ROUNDED_TOP_LEFT;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::Padding;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget;
use ratatui::widgets::block;
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::FileType;
use std::collections::VecDeque;
use std::f64;
use std::io::stdout;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use tachyonfx::EffectManager;
use tachyonfx::EffectTimer;
use tachyonfx::Interpolation;
use tachyonfx::fx;
use throbber_widgets_tui::Throbber;

use throbber_widgets_tui::ThrobberState;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

use tracing::debug;
use tracing::{error, info};
use tui_logger::TuiLoggerLevelOutput;
use tui_logger::TuiLoggerWidget;
use tui_logger::TuiWidgetState;

const CHARSET: symbols::line::Set = symbols::line::Set {
    top_left: "#",
    top_right: "#",
    bottom_left: "#",
    bottom_right: "#",
    horizontal: "#",
    vertical: "│",
    vertical_left: "│",
    vertical_right: "│",
    cross: "┼",
    horizontal_up: "┴",
    horizontal_down: "┬",
};

pub struct MainUI {
    pub current_path: String,
    pub table_state: TableState<RowSelection>,
    pub current_file_entries: Vec<FileEntry>,
    pub input_state: TextInputState,
    pub input_mode: InputMode,
    pub sftp: Arc<SftpSession>,
    pub session: Arc<AsyncMutex<Session>>,
    pub log_state: TuiWidgetState,
    pub throbber: ThrobberState,
    pub is_downloading: bool,
    pub filtered_file_entries: Vec<FileEntry>,
    pub total_files_to_download: usize,
    pub downloaded_files: usize,
    pub download_progress: f64,
    pub next_five_files: VecDeque<FileEntry>,
    pub throbber_cancel: Option<Cancel>,
    pub effects: EffectManager<()>,
    pub elapsed: Instant,
    pub details_para_state: ParagraphState,
    pub detail_window_mode: DetailWindowMode,
    pub current_file_content: Option<String>,
    pub in_editor: bool,
}
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    #[default]
    Filter,
    DownloadPath,
    ConfirmDelete,
    MoveEntry,
    _CopyEntry,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum DetailWindowMode {
    #[default]
    Details,
    Content,
}

impl MainUI {
    pub fn new(
        current_path: String,
        sftp: Arc<SftpSession>,
        session: Arc<AsyncMutex<Session>>,
    ) -> Self {
        let mut effects: EffectManager<()> = EffectManager::default();
        let fx = fx::expand(
            fx::ExpandDirection::Vertical,
            Style::default(),
            EffectTimer::new(tachyonfx::Duration::from_millis(250), Interpolation::Linear),
        );
        effects.add_effect(fx);
        Self {
            current_path,
            table_state: TableState::default(),
            current_file_entries: Vec::new(),
            input_state: TextInputState::default(),
            input_mode: InputMode::default(),
            sftp,
            log_state: TuiWidgetState::new(),
            throbber: ThrobberState::default(),
            is_downloading: false,
            download_progress: 0.0,
            filtered_file_entries: Vec::new(),
            session,
            total_files_to_download: 0,
            downloaded_files: 0,
            next_five_files: VecDeque::new(),
            throbber_cancel: None,
            effects,
            elapsed: Instant::now(),
            details_para_state: ParagraphState::default(),
            detail_window_mode: DetailWindowMode::default(),
            current_file_content: None,
            in_editor: false,
        }
    }

    pub fn get_file_entries(&self) -> &Vec<FileEntry> {
        if self.filtered_file_entries.is_empty() {
            &self.current_file_entries
        } else {
            &self.filtered_file_entries
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

    if state.detail_window_mode == DetailWindowMode::Details {
        state.current_file_content = None;
    }

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
    Clear.render(right, buf);

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

    let &[rb_top, rb_bottom] = Layout::new(
        Direction::Vertical,
        [
            Constraint::Fill(1),
            Constraint::Length(if state.is_downloading { 9 } else { 4 }),
        ],
    )
    .split(right_bottom)
    .as_ref() else {
        unreachable!()
    };
    let log_widget = TuiLoggerWidget::default()
        .style_error(Style::default().fg(Color::Red))
        .style_debug(Style::default().fg(Color::Cyan))
        .style_warn(Style::default().fg(Color::Yellow))
        .style_trace(Style::default().fg(Color::Magenta))
        .style_info(Style::default().fg(Color::Green))
        .output_timestamp(Some("%H:%M:%S".to_string()))
        .output_level(Some(TuiLoggerLevelOutput::Long))
        .output_target(true)
        .output_file(false)
        .output_line(false)
        .state(&state.log_state)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .style(ctx.theme.container_border())
                .title_top(Line::raw("Log")),
        );
    log_widget.render(rb_top, buf);

    let gauge_block = Block::default()
        .borders(Borders::ALL)
        .padding(Padding::horizontal(1))
        .style(ctx.theme.container_border())
        .border_type(BorderType::Rounded);
    let el = state.elapsed.elapsed();
    if state.is_downloading {
        let &[rb_bottom_1, rb_bottom_2] = Layout::new(
            Direction::Vertical,
            [Constraint::Length(3), Constraint::Length(6)],
        )
        .split(rb_bottom)
        .as_ref() else {
            unreachable!()
        };
        let throbber = Throbber::default()
            .label(format!(
                "Downloaded {:.0}/{}  ",
                state.downloaded_files, state.total_files_to_download
            ))
            .throbber_set(throbber_widgets_tui::ASCII);

        let progress_area = gauge_block.inner(rb_bottom_1);
        let para_area = gauge_block.inner(rb_bottom_2);
        let gauge = LineGauge::default()
            .filled_style(Style::default().fg(Color::Black).on_green())
            .unfilled_style(ctx.theme.container_base())
            // .block(gauge_block.clone().padding(Padding::horizontal(1)))
            .ratio(state.download_progress)
            .label(throbber)
            .line_set(CHARSET);
        let next_five_files = state.next_five_files.clone();
        let data = ProgressDataSlice(&Vec::from(next_five_files));
        let table = Table::<NoSelection>::new()
            .data(data)
            .widths([Constraint::Percentage(70), Constraint::Percentage(30)])
            .column_spacing(1)
            .block(Block::default().padding(Padding::ZERO))
            .header(rat_ftable::textdata::Row::new([
                Cell::from("Name"),
                Cell::from("Size"),
            ]))
            .styles(ctx.theme.table_style());

        table.render(para_area, buf, &mut TableState::<NoSelection>::default());
        gauge_block.title("Progress").render(rb_bottom, buf);

        gauge.render(progress_area, buf);
        state.elapsed = Instant::now();
        state
            .effects
            .process_effects(el.mul_f64(7.0).into(), buf, rb_bottom_1);
        state
            .effects
            .process_effects(el.mul_f64(7.0).into(), buf, rb_bottom_2);
        state
            .effects
            .process_effects(el.mul_f64(7.0).into(), buf, rb_bottom);
    } else {
        let hints = [
            keybind("Tab", "Focus  "),
            keybind("h/j/k/l", "Naviagte Table  "),
            keybind("d", "Download  "),
            keybind("f", "Filter  "),
        ]
        .iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
        let hints_2 = [
            keybind("Enter", "View Content  "),
            keybind("x", "Delete  "),
            keybind("m", "Move  "),
            keybind("Enter e", "Edit  "),
        ]
        .iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
        Paragraph::new(vec![Line::from(hints), Line::from(hints_2)])
            .styles(ctx.theme.paragraph_style())
            .alignment(ratatui::layout::Alignment::Center)
            .block(
                gauge_block
                    .style(ctx.theme.container_border())
                    .title("Keybinds"),
            )
            .render(rb_bottom, buf, &mut ParagraphState::default());
        //       state
        //           .effects
        //           .process_effects(el.mul_f64(2.0).into(), buf, rb_bottom);
    }

    if let Some(row) = state.table_state.selected() {
        let file = state.get_file_entries().get(row);
        if let Some(file) = file {
            let paragraph = if let Some(content) = &state.current_file_content
                && state.detail_window_mode == DetailWindowMode::Content
            {
                Paragraph::new(content.clone())
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .title_top("[2] Content")
                            .style(ctx.theme.container_border())
                            .padding(Padding::uniform(1)),
                    )
                    .scroll(Scroll::new())
                    .styles(ctx.theme.paragraph_style())
            } else {
                Clear.render(right_top, buf);
                Paragraph::from(file.clone())
                    .block(
                        Block::bordered()
                            .border_type(BorderType::Rounded)
                            .style(ctx.theme.container_border())
                            .title_top("[2] Details")
                            .padding(Padding::uniform(1)),
                    )
                    .scroll(Scroll::new())
                    .styles(ctx.theme.paragraph_style())
            };
            paragraph.render(right_top, buf, &mut state.details_para_state);
        }
    } else {
        Block::bordered()
            .border_type(BorderType::Rounded)
            .style(ctx.theme.container_border())
            .title_top("[2] Details")
            .render(right_top, buf);
    }
    let input_block_title = match state.input_mode {
        InputMode::Filter => "[3] Filter".to_string(),
        InputMode::DownloadPath => {
            let current_item = state.table_state.selected_checked().unwrap_or_default();
            let file = state.get_file_entries()[current_item].clone();

            format!(
                "[3] Download [{}/{}] to Path",
                state.current_path,
                file.name()
            )
        }
        InputMode::ConfirmDelete => {
            let current_item = state.table_state.selected_checked().unwrap_or_default();
            let file = state.get_file_entries()[current_item].clone();

            format!("[3] rm -rf [{}/{}]", state.current_path, file.name())
        }
        InputMode::MoveEntry => {
            let current_item = state.table_state.selected_checked().unwrap_or_default();
            let file = state.get_file_entries()[current_item].clone();

            format!("[3] mv [{}/{}] to Path", state.current_path, file.name())
        }
        _ => String::new(),
    };
    let input = TextInput::new().style(ctx.theme.container_base()).block(
            Block::bordered()
                .style(ctx.theme.container_border())
                .border_type(BorderType::Rounded)
                .border_style(match_focus!(state.input_state => ctx.theme.container_border().fg(ratatui::style::Color::Yellow), else => ctx.theme.container_border()))
                .title_top(input_block_title)
                .padding(Padding::horizontal(1)),
        );
    input.render(left_bottom, buf, &mut state.input_state);

    let data = FileDataSlice(&state.get_file_entries().clone());

    let table_style = ctx.theme.table_style();
    let table = Table::<RowSelection>::default()
        .block(
            Block::bordered()
                .padding(Padding::horizontal(1))
                .border_type(block::BorderType::Rounded)
                .title_top("[1]")
                .border_style(ctx.theme.container_border()),
        )
        .data(data)
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

impl_has_focus!(table_state, input_state, details_para_state for MainUI);

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
        chan.send(Ok(Control::Event(AppEvent::UpdateCurrentPath(full_path))))
            .await?;
        chan.send(Ok(Control::Event(AppEvent::UpdateFiles(files))))
            .await?;
        Ok(Control::Event(AppEvent::AsyncTick(300)))
    });

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
            try_flow!(match event {
                ct_event!(key press CONTROL-'q') => {
                    if let Some(cancel) = state.throbber_cancel.take() {
                        cancel.cancel();
                    }
                    Control::Quit
                }
                ct_event!(keycode press Esc) => {
                    ctx.focus().focus(&state.table_state);
                    state.filtered_file_entries.clear();
                    state.input_state.clear();
                    Control::Changed
                }
                _ => Control::Continue,
            });
            try_flow!(if let Some(focused) = ctx.focus().focused()
                && focused != state.input_state.focus
            {
                match event {
                    ct_event!(key press '1') => {
                        ctx.focus().focus(&state.table_state);
                        Control::Changed
                    }
                    ct_event!(key press '2') => {
                        ctx.focus().focus(&state.details_para_state);
                        Control::Changed
                    }
                    _ => Control::Continue,
                }
            } else {
                Control::Continue
            });
            try_flow!(state.details_para_state.handle(event, Regular));

            try_flow!(match_focus!(
                state.table_state => {
                try_flow!(
                    match rowselection::handle_events(
                        &mut state.table_state,
                        true,
                        event
                    ) {
                    rat_ftable::event::TableOutcome::Selected => {
                        state.detail_window_mode = DetailWindowMode::Details;
                        Control::Changed
                    }
                    v => v.into(),
                }
                );
                    match event {
                        ct_event!(key press 'j') => {
                            state.current_file_content = None;
                            state.detail_window_mode = DetailWindowMode::Details;

                            state.table_state.move_down(1);
                            Control::<AppEvent>::Changed
                        }
                        ct_event!(key press 'k') => {
                            state.current_file_content = None;
                        state.detail_window_mode = DetailWindowMode::Details;
                            state.table_state.move_up(1);
                            Control::Changed
                        }
                        ct_event!(key press 'x') => {
                            state.input_mode = InputMode::ConfirmDelete;
                            state.input_state.set_value("Delete file [Y/n]?".to_string());
                            ctx.focus().focus(&state.input_state);
                            Control::Changed
                        }
                        ct_event!(key press 'm') => {
                            state.input_mode = InputMode::MoveEntry;
                            ctx.focus().focus(&state.input_state);
                            Control::Changed
                        }
                        ct_event!(key press 'e') => {
                            Control::Event(AppEvent::SpawnExternalEditor(state.current_path.clone().join(state.get_file_entries()[state.table_state.selected_checked().unwrap_or_default()].name())))
                        }
                        ct_event!(keycode press Enter) => {
                        if let Some(row_idx) = state.table_state.selected() && let Some(row) = state.get_file_entries().get(row_idx) && row.is_file() {

                                let sftp = Arc::clone(&state.sftp);
                                let current_path = state.current_path.clone();
                                let row = row.clone();
                                ctx.spawn_async_ext(async move |_| {
                                    let mut file = sftp
                                        .open(current_path.clone().join(row.name()))
                                    .await?;
                                    let mut buf = Vec::new();
                                    file.read_to_end(&mut buf).await?;
                                    let content = String::from_utf8(buf).ok();

                                    Ok(Control::Event(AppEvent::UpdateContent(content)))
                                });

                                state.detail_window_mode = DetailWindowMode::Content;
                            }
                            Control::Continue
                        }
                        ct_event!(keycode press Left ) | ct_event!(key press 'h')=> {

                        state.detail_window_mode = DetailWindowMode::Details;
                            let path = PathBuf::from(state.current_path.clone());
                            let parent = path.parent();
                            if let Some(parent) = parent {
                                let parent = parent.display();
                                state.current_path = parent.to_string();
                                state.filtered_file_entries.clear();
                                Control::Event(AppEvent::ChangeDir(parent.to_string()))
                            } else {
                                Control::Continue
                            }
                        }

                        ct_event!(key press 'l') | ct_event!(keycode press Right) => {
                        state.detail_window_mode = DetailWindowMode::Details;
                            let path = PathBuf::from(state.current_path.clone());
                            let selected = state.table_state.selected();
                            if let Some(selected) = selected {
                                let Some(file) = state.get_file_entries().get(selected) else {
                                    return Ok(Control::Continue);
                                };
                                if file.is_dir() {
                                    let path = path.join(file.name());
                                    state.current_path = path.display().to_string();
                                    state.filtered_file_entries.clear();
                                    return Ok(Control::Event(AppEvent::ChangeDir(path.display().to_string())));
                                }
                            }
                            Control::Continue
                        }
                        ct_event!(key press 'd') => {
                            state.input_mode = InputMode::DownloadPath;
                            state.input_state.clear();
                            ctx.focus().focus(&state.input_state);

                            Control::Changed
                        }
                        ct_event!(key press 'f') => {
                            state.input_mode = InputMode::Filter;
                            ctx.focus().focus(&state.input_state);
                            Control::Changed
                        }
                        _ => Control::Continue
                    }
                },
                state.input_state => {
                    match state.input_mode {
                        InputMode::ConfirmDelete => {
                        try_flow!(
                            match event {
                                ct_event!(key press 'y') => {
                                    if let Some(idx) = state.table_state.clone().selected() && let Some(file) = state.get_file_entries().get(idx){
                                        let file = file.clone();
                                        state.input_state.clear();
                                        ctx.focus().first();
                                        Control::Event(AppEvent::DeleteEntry(file.clone()))
                                    } else {
                                        Control::Continue
                                    }

                                }
                                ct_event!(key press 'n') => {
                                    state.input_state.clear();
                                    ctx.focus().focus(&state.table_state);
                                    state.input_mode = InputMode::Filter;
                                    Control::Changed
                                }

                                _ => Control::Continue
                            }
                        )
                        }
                        InputMode::DownloadPath => {
                        match event {
                            ct_event!(keycode press Enter) => {
                                let path:String = state.input_state.value();
                                let path = PathBuf::from(path);
                                std::fs::create_dir_all(path.clone())?;
                                let path = path.canonicalize()?;
                                let selected = state.table_state.selected();
                                if let Some(selected) = selected {
                                    let Some(file) = state.get_file_entries().get(selected) else {
                                        return Ok(Control::Continue);
                                    };
                                    let path = path.join(file.name());
                                    let name = state.current_path.clone().join(file.name());
                                    if file.is_dir() {
                                        return Ok(Control::Event(AppEvent::DownloadFolder(name, path)));
                                    }
                                    return Ok(Control::Event(AppEvent::DownloadFile(name, path, Some(file.name().clone()))));
                                }
                            }
                            _ => {}
                        }
                        }
                        InputMode::MoveEntry => {
                            match event {
                                ct_event!(keycode press Enter) => {
                                    let old_path = state.current_path.clone().join(state.get_file_entries()[state.table_state.selected_checked().unwrap_or_default()].name());
                                    let new_path:String = state.current_path.clone().join(&state.input_state.value::<String>());
                                    ctx.focus().first();
                                    state.input_state.clear();
                                    state.input_mode = InputMode::default();
                                    return Ok(Control::Event(AppEvent::MoveEntry(old_path, new_path)));

                                }
                                _ => {}
                            }
                        }
                        _ => {}
                    }

                    Control::Continue
                },

                else => Control::Continue
            ));
            try_flow!(match state.input_state.handle(event, Regular) {
                TextOutcome::TextChanged => {
                    if state.input_mode == InputMode::Filter {
                        let filter: String = state.input_state.value();
                        state.filtered_file_entries = state
                            .current_file_entries
                            .iter()
                            .filter(|file| file.name().contains(&filter))
                            .cloned()
                            .collect();
                    }
                    Control::Changed
                }
                v => v.into(),
            });
            Control::Continue
        }
        AppEvent::AsyncMsg(s) => {
            // receive result from async operation
            Control::Event(AppEvent::Message(s.clone()))
        }
        AppEvent::Throb => {
            debug!("Throbber");
            state.throbber.calc_next();
            Control::Changed
        }
        AppEvent::SpawnExternalEditor(_) => {
            if let Some(template) = state.current_file_content.clone() {
                state.in_editor = true;
                stdout().execute(crossterm::terminal::LeaveAlternateScreen)?;
                disable_raw_mode()?;
                let edited = edit::edit(template)?;
                let session = Arc::clone(&state.session);
                let current_path = state.current_path.clone();
                let current_file = state.get_file_entries()
                    [state.table_state.selected_checked().unwrap_or_default()]
                .clone();

                let path = current_path.join(current_file.name());
                ctx.spawn_async_ext(async move |chan| {
                    chan.send(Ok(Control::Event(AppEvent::UpdateContent(Some(
                        edited.clone(),
                    )))))
                    .await?;
                    let mut session = session.lock().await;
                    let sftp = session.sftp().await?;
                    let mut f = sftp.create(path.clone()).await?;
                    f.write_all(edited.as_bytes()).await?;
                    sftp.close().await?;
                    Ok(Control::Event(AppEvent::AsyncTick(300)))
                });
                stdout().execute(EnterAlternateScreen)?;
                enable_raw_mode()?;
                ctx.terminal().borrow_mut().clear()?;
                state.in_editor = false;
            }
            Control::Changed
        }
        AppEvent::DownloadStart => {
            state.is_downloading = true;
            let cancel = ctx.spawn_ext(|cancel, send| {
                loop {
                    if cancel.is_canceled() {
                        break;
                    }
                    send.send(Ok(Control::Event(AppEvent::Throb)))?;
                    send.send(Ok(Control::Changed))?;
                    thread::sleep(Duration::from_millis(500));
                }
                Ok(Control::Changed)
            })?;
            state.throbber_cancel = Some(cancel.0);
            Control::Changed
        }
        AppEvent::UpdateContent(content) => {
            state.current_file_content = content.clone();
            Control::Changed
        }
        AppEvent::DownloadEnd => {
            state.is_downloading = false;
            if let Some(cancel) = state.throbber_cancel.take() {
                cancel.cancel();
            }
            Control::Changed
        }
        AppEvent::UpdateNextFiveFiles(files) => {
            state.next_five_files = files.to_vec().into();
            Control::Changed
        }
        AppEvent::Gauge(progress) => {
            state.download_progress = *progress;
            state.downloaded_files += 1;
            Control::Changed
        }
        AppEvent::MoveEntry(oldpath, newpath) => {
            let session = Arc::clone(&state.session);
            let oldpath = oldpath.clone();
            let newpath = newpath.clone();
            let current_path = state.current_path.clone();
            ctx.spawn_async_ext(|_| async move {
                let mut session = session.lock().await;
                let sftp = session.sftp().await?;
                let newpath = sftp.canonicalize(newpath.clone()).await.unwrap_or(newpath);
                info!(oldpath, newpath, "Moving");
                sftp.rename(oldpath, newpath).await?;
                Ok(Control::Event(AppEvent::ChangeDir(current_path)))
            });
            Control::Changed
        }
        AppEvent::DownloadFile(name, path, filename) => {
            state.throbber.calc_next();
            info!(name, path = ?path.display(), filename = ?filename.clone(), "File Details");
            let session = Arc::clone(&state.session);
            let path = path.clone();
            let name = name.clone();

            info!(name, path = ?path.display(), "File Details");
            ctx.spawn_async_ext(|_| async move {
                let sftp = {
                    let mut session = session.lock().await;
                    session.sftp().await?
                };
                let mut remote_file = sftp.open(name.clone()).await?;
                let mut buf = Vec::new();
                remote_file.read_to_end(&mut buf).await?;
                info!(len = ?buf.len(), "Read file");
                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                info!(path = ?path.display(),"Writing file to" );
                let mut file = tokio::fs::File::create(path).await?;
                let _ = file.write(&buf).await?;
                file.flush().await?;
                file.sync_all().await?;
                Ok(Control::Event(AppEvent::AsyncTick(300)))
            });
            Control::Continue
        }
        AppEvent::DownloadFolder(file, path) => {
            ctx.queue_event(AppEvent::DownloadStart);

            info!("Downloading folder {}", file);

            let session = Arc::clone(&state.session);
            let dirname = file.split('/').next_back().unwrap_or("");
            std::fs::create_dir_all(path.clone())?;
            let path = path.clone().canonicalize()?;
            info!(path =?path.display(), dirname, "Path and dirname");

            let file = file.clone();
            ctx.spawn_async_ext(|chan| async move {
                    let sftp = {
                        let mut session = session.lock().await;
                        session.sftp().await?
                    };
                    let walker = WalkParallel {
                        filter: Arc::new(|_| true),
                        path: file.clone().into(),
                        max_depth: Some(3),
                        min_depth: None,
                        threads: 4,
                        sftp: sftp.into(),
                    };
                    let collected = Arc::new(Mutex::new(Vec::<FileEntry>::new()));
                    let collected_ref = Arc::clone(&collected);
                    walker
                        .run(|| {
                            // This closure is called once per worker thread.
                            let collected = collected_ref.clone();
                            Box::new(move |entry_res: Result<FileEntry>| -> WalkState {
                                match entry_res {
                                    Ok(entry) => {
                                        // Push this FileEntry into the shared vector
                                        info!(entry =?entry.name(), "Visited");
                                        let mut vec = collected.lock().unwrap();
                                        vec.push(entry);
                                    }
                                    Err(err) => {
                                        error!("Error visiting entry: {:?}", err);
                                    }
                                }
                                WalkState::Continue
                            })
                        })
                        .await;
                    let collected_snapshot = {
                        let lock = collected.lock().unwrap();
                        lock.clone()
                    }; // lock dropped here

                    // 2️⃣ Process outside of the lock
                    let tx = start_sftp_worker(session.clone());
                    let total = collected_snapshot.len() as f64;
                    chan.send(Ok(Control::Event(AppEvent::SetTotalFilesToDownload(total as usize)))).await?;
                    let mut progress = 0.0;
                    let mut windows = collected_snapshot.windows(5);
                    let last_window = if let Some(window) = windows.clone().last() {
                        window.to_vec()
                    } else {
                        collected_snapshot.to_vec()
                    };


                    for entry in collected_snapshot.clone() {
                        let file = file.clone();
                        let filename = entry.name().strip_prefix(&file).unwrap_or(entry.name()).replacen("/", "", 1).to_string();
                        let target_path = path.join(&filename);
                        info!(file, filename, target_path = ?target_path.display().to_string(), "Downloading");

                        let (reply_tx, reply_rx) = oneshot::channel();
                        tx.send(SftpCmd::ReadFile {
                            remote_path: file.clone().join(&filename),
                            local_path: target_path,
                            reply: reply_tx,
                        })?;
                        let _ = reply_rx.await.unwrap();
                        progress += 1.0;
                        if let Some(window) = windows.next() {
                            chan.send(Ok(Control::Event(AppEvent::UpdateNextFiveFiles(window.to_vec())))).await?;
                        } else {
                            let mut vec = last_window.to_vec();
                            vec.remove(0);
                            chan.send(Ok(Control::Event(AppEvent::UpdateNextFiveFiles(vec)))).await?;
                        }
                        chan.send(Ok(Control::Event(AppEvent::Gauge(progress/ total)))).await?;


                        //sleep(Duration::from_millis(50)).await;
                    }
                    chan.send(Ok(Control::Event(AppEvent::DownloadEnd))).await?;
                    Ok(Control::Event(AppEvent::AsyncTick(300)))
                });
            Control::Continue
        }
        AppEvent::DeleteEntry(file) => {
            let session = Arc::clone(&state.session);
            let file = file.clone();
            let curr_path = state.current_path.clone();
            let fname = curr_path.join(file.name());
            ctx.spawn_async_ext(|chan| async move {
                let mut session = session.lock().await;
                let sftp = session.sftp().await?;
                info!(fname, "Deleting");
                match file.type_() {
                    FileType::File => {
                        sftp.remove_file(fname).await?;
                    }
                    FileType::Dir => {
                        remove_dir_recursive(&sftp, &fname).await?;
                    }
                    _ => {}
                }
                chan.send(Ok(Control::Event(AppEvent::ChangeDir(curr_path.clone()))))
                    .await?;

                Ok(Control::Changed)
            });

            Control::Changed
        }
        AppEvent::ChangeDir(path) => {
            let path = if !path.is_empty() {
                path.clone()
            } else {
                ".".to_string()
            };
            info!("changing dir to {}", path);
            let sftp = Arc::clone(&state.sftp);
            state.input_state.clear();
            state.input_mode = InputMode::default();

            ctx.spawn_async_ext(|chan| async move {
                let files = sftp.read_dir(path).await?;
                let files = files.into_iter().map(FileEntry::from).collect::<Vec<_>>();
                chan.send(Ok(Control::Event(AppEvent::UpdateFiles(files))))
                    .await?;

                Ok(Control::Event(AppEvent::AsyncTick(300)))
            });
            Control::Continue
        }
        AppEvent::SetTotalFilesToDownload(total) => {
            state.total_files_to_download = *total;
            Control::Changed
        }
        AppEvent::UpdateFiles(files) => {
            state.current_file_entries = files.to_vec();
            state.input_state.clear();
            state.input_mode = InputMode::default();
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
use tokio::sync::{mpsc, oneshot};

enum SftpCmd {
    ReadFile {
        remote_path: String,
        local_path: PathBuf,
        reply: oneshot::Sender<Result<()>>,
    },
}

fn start_sftp_worker(session: Arc<AsyncMutex<Session>>) -> mpsc::UnboundedSender<SftpCmd> {
    let (tx, mut rx) = mpsc::unbounded_channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let _: Result<()> = rt.block_on(async move {
            while let Some(cmd) = rx.recv().await {
                match cmd {
                    SftpCmd::ReadFile {
                        remote_path,
                        local_path,
                        reply,
                    } => {
                        let result = async {
                            let mut session = session.lock().await;
                            info!("Opening remote file {:?}", remote_path);
                            let sftp = session.sftp().await?;
                            info!("Got SFTP channel");
                            let mut remote_file = sftp.open(&remote_path).await;
                            info!("Opened remote file");
                            let mut buf = Vec::new();
                            let read = remote_file
                                .as_mut()
                                .ok()
                                .unwrap()
                                .read_to_end(&mut buf)
                                .await;

                            if read.is_err() {
                                error!("Read result: {:?}", read);
                            } else {
                                info!("Read result: {:?}", read);
                            }
                            let mut file = tokio::fs::File::create(local_path).await?;
                            file.write_all(&buf).await?;
                            file.flush().await?;
                            file.sync_all().await?;
                            eyre::Ok(())
                        }
                        .await;
                        let _ = reply.send(result);
                    }
                }
            }
            eyre::Ok(())
        });
        eyre::Ok(())
    });
    tx
}

#[inline]
fn keybind<'a>(key: &'a str, description: &str) -> Vec<Span<'a>> {
    vec![
        Span::styled("<", Style::default().fg(Color::White)),
        Span::styled(key, Style::default().fg(Color::LightYellow)),
        Span::styled(
            format!("> {}", description),
            Style::default().fg(Color::White),
        ),
    ]
}

async fn remove_dir_recursive(sftp: &SftpSession, root: &str) -> Result<()> {
    let mut stack = vec![root.to_string()];

    while let Some(path) = stack.pop() {
        let entries = sftp.read_dir(&path).await?;
        for entry in entries {
            let attrs = entry.metadata();
            let name = entry.file_name();
            let child_path = format!("{}/{}", path, name);
            if attrs.is_dir() {
                // Push directory for later deletion
                stack.push(child_path.clone());
            } else {
                sftp.remove_file(&child_path).await?;
            }
        }
        // Once children are deleted, remove dir
        sftp.remove_dir(&path).await?;
    }

    Ok(())
}
