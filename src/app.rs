use crate::core::backend::{CrontabBackend, SystemCrontab};
use crate::core::backup::BackupManager;
use crate::core::conflict::CrontabState;
use crate::core::{expression, parser, writer};
use crate::gui::theme::{font_size, spacing};
use crate::model::crontab::CrontabLine;
use crate::model::job::{CronJob, JobDraft, JobId};
use crate::model::schedule::ScheduleFields;

use iced::widget::{
    button, checkbox, column, container, horizontal_rule, horizontal_space, pick_list, row,
    scrollable, text, text_input, vertical_space,
};
use iced::{Center, Element, Fill, Task, Theme};
use tracing::{error, info, warn};

/// Which view is currently displayed.
#[derive(Debug, Clone, PartialEq)]
pub enum View {
    JobList,
    CreateJob,
    EditJob(JobId),
}

/// Editor input mode for cron expression.
#[derive(Debug, Clone, PartialEq)]
pub enum ExpressionMode {
    Raw,
    Visual,
}

/// Status bar message with severity level.
#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub level: StatusLevel,
}

#[derive(Debug, Clone)]
pub enum StatusLevel {
    Info,
    Success,
    Error,
}

/// Top-level application state.
pub struct App {
    // Data — Vec<CrontabLine> is the single source of truth
    crontab_lines: Vec<CrontabLine>,
    jobs: Vec<CronJob>,
    crontab_state: Option<CrontabState>,

    // View state
    current_view: View,

    // Job list state
    search_query: String,
    tag_filter: Option<String>,
    filtered_job_indices: Vec<usize>,
    all_tags: Vec<String>,

    // Editor state
    draft: JobDraft,
    editing_job_id: Option<JobId>,
    expression_mode: ExpressionMode,
    schedule_fields: ScheduleFields,

    // Expression preview
    description_preview: String,
    next_executions: Vec<String>,
    validation_error: Option<String>,

    // System state
    status_message: Option<StatusMessage>,
    show_delete_confirm: Option<JobId>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            crontab_lines: Vec::new(),
            jobs: Vec::new(),
            crontab_state: None,
            current_view: View::JobList,
            search_query: String::new(),
            tag_filter: None,
            filtered_job_indices: Vec::new(),
            all_tags: Vec::new(),
            draft: JobDraft::default(),
            editing_job_id: None,
            expression_mode: ExpressionMode::Raw,
            schedule_fields: ScheduleFields::default(),
            description_preview: String::new(),
            next_executions: Vec::new(),
            validation_error: None,
            status_message: None,
            show_delete_confirm: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    // Crontab I/O
    LoadCrontab,
    CrontabLoaded(Result<String, String>),
    SaveCrontab,
    CrontabSaved(Result<(), String>),

    // Navigation
    NavigateTo(View),

    // Job list
    SearchChanged(String),
    TagFilterChanged(String),
    ClearTagFilter,
    ToggleJobEnabled(JobId),
    RequestDeleteJob(JobId),
    ConfirmDeleteJob(JobId),
    CancelDelete,

    // Job editor
    DraftCommandChanged(String),
    DraftCommentChanged(String),
    DraftTagsChanged(String),
    DraftEnabledToggled(bool),

    // Expression input
    ExpressionRawChanged(String),
    SwitchExpressionMode(ExpressionMode),

    // Visual builder
    MinuteFieldChanged(String),
    HourFieldChanged(String),
    DayOfMonthFieldChanged(String),
    MonthFieldChanged(String),
    DayOfWeekFieldChanged(String),

    // Editor actions
    SaveJob,
    CancelEdit,
}

impl App {
    pub fn new() -> (Self, Task<Message>) {
        (Self::default(), Task::done(Message::LoadCrontab))
    }

    pub fn title(&self) -> String {
        "Cron Manager".into()
    }

    pub fn theme(&self) -> Theme {
        crate::gui::theme::app_theme()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::LoadCrontab => {
                info!("Loading crontab");
                Task::perform(
                    async {
                        let backend = SystemCrontab;
                        backend
                            .read()
                            .map_err(|e| e.to_string())
                    },
                    Message::CrontabLoaded,
                )
            }

            Message::CrontabLoaded(result) => {
                match result {
                    Ok(raw) => {
                        self.crontab_state = Some(CrontabState::new(&raw));
                        self.crontab_lines = parser::parse_crontab(&raw);
                        self.jobs = parser::extract_jobs(&self.crontab_lines);
                        self.all_tags = self.collect_all_tags();
                        self.apply_filters();
                        self.status_message = Some(StatusMessage {
                            text: format!("{} jobs loaded", self.jobs.len()),
                            level: StatusLevel::Success,
                        });
                        info!(jobs = self.jobs.len(), "Crontab loaded");
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to load crontab");
                        self.status_message = Some(StatusMessage {
                            text: format!("Failed to load: {}", e),
                            level: StatusLevel::Error,
                        });
                    }
                }
                Task::none()
            }

            Message::SaveCrontab => {
                let content = writer::serialize_crontab(&self.crontab_lines);

                // Conflict detection — if it fails, abort save entirely
                if let Some(ref state) = self.crontab_state {
                    let backend = SystemCrontab;
                    match state.detect_conflict(&backend) {
                        Ok(Some(_)) => {
                            warn!("Conflict detected — crontab modified externally, reloading");
                            self.status_message = Some(StatusMessage {
                                text: "Crontab was modified externally! Reloading...".into(),
                                level: StatusLevel::Error,
                            });
                            // Rollback: reload from system
                            return self.update(Message::LoadCrontab);
                        }
                        Err(e) => {
                            error!(error = %e, "Conflict detection failed, aborting save");
                            self.status_message = Some(StatusMessage {
                                text: format!("Save aborted: conflict check failed ({})", e),
                                level: StatusLevel::Error,
                            });
                            // Rollback: reload from system
                            return self.update(Message::LoadCrontab);
                        }
                        Ok(None) => {}
                    }
                }

                // Backup the LIVE crontab before overwriting (not our new content)
                if let Some(ref state) = self.crontab_state {
                    match BackupManager::new() {
                        Ok(mgr) => {
                            if let Err(e) = mgr.create_backup(&state.content) {
                                warn!(error = %e, "Backup failed, proceeding with save anyway");
                            }
                        }
                        Err(e) => {
                            warn!(error = %e, "Could not initialize backup manager");
                        }
                    }
                }

                let content_clone = content.clone();
                Task::perform(
                    async move {
                        let backend = SystemCrontab;
                        backend.write(&content_clone).map_err(|e| e.to_string())
                    },
                    Message::CrontabSaved,
                )
            }

            Message::CrontabSaved(result) => {
                match result {
                    Ok(()) => {
                        // Update conflict state
                        let content = writer::serialize_crontab(&self.crontab_lines);
                        if let Some(ref mut state) = self.crontab_state {
                            state.update(&content);
                        }
                        self.status_message = Some(StatusMessage {
                            text: "Crontab saved successfully".into(),
                            level: StatusLevel::Success,
                        });
                        info!("Crontab saved");
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to save crontab, reloading to sync state");
                        self.status_message = Some(StatusMessage {
                            text: format!("Save failed: {}. Reloading...", e),
                            level: StatusLevel::Error,
                        });
                        // Rollback: reload from system to get back in sync
                        return self.update(Message::LoadCrontab);
                    }
                }
                Task::none()
            }

            Message::NavigateTo(view) => {
                match &view {
                    View::CreateJob => {
                        self.draft = JobDraft {
                            enabled: true,
                            ..Default::default()
                        };
                        self.editing_job_id = None;
                        self.expression_mode = ExpressionMode::Raw;
                        self.schedule_fields = ScheduleFields::default();
                        self.description_preview.clear();
                        self.next_executions.clear();
                        self.validation_error = None;
                    }
                    View::EditJob(id) => {
                        if let Some(job) = self.jobs.iter().find(|j| j.id == *id) {
                            self.draft = JobDraft::from_job(job);
                            self.editing_job_id = Some(*id);
                            // Always default to Raw mode — visual builder is opt-in
                            // to avoid lossy round-trips for expressions it can't fully represent
                            self.expression_mode = ExpressionMode::Raw;
                            if let Some(fields) = expression::decompose(&job.expression) {
                                self.schedule_fields = fields;
                            }
                            let expr = job.expression.clone();
                            self.update_expression_preview(&expr);
                        }
                    }
                    View::JobList => {
                        self.show_delete_confirm = None;
                    }
                }
                self.current_view = view;
                Task::none()
            }

            // Job list actions
            Message::SearchChanged(query) => {
                self.search_query = query;
                self.apply_filters();
                Task::none()
            }

            Message::TagFilterChanged(tag) => {
                self.tag_filter = if tag.is_empty() { None } else { Some(tag) };
                self.apply_filters();
                Task::none()
            }

            Message::ClearTagFilter => {
                self.tag_filter = None;
                self.apply_filters();
                Task::none()
            }

            Message::ToggleJobEnabled(id) => {
                if let Some(job) = self.jobs.iter().find(|j| j.id == id) {
                    let line_idx = job.line_index;
                    if let Some(line) = self.crontab_lines.get_mut(line_idx) {
                        line.toggle_enabled();
                    }
                    self.jobs = parser::extract_jobs(&self.crontab_lines);
                    self.apply_filters();
                    return self.update(Message::SaveCrontab);
                }
                Task::none()
            }

            Message::RequestDeleteJob(id) => {
                self.show_delete_confirm = Some(id);
                Task::none()
            }

            Message::ConfirmDeleteJob(id) => {
                if let Some(job) = self.jobs.iter().find(|j| j.id == id) {
                    let range = writer::job_removal_range(&self.crontab_lines, job.line_index);
                    let start = *range.start();
                    let end = *range.end();
                    self.crontab_lines.drain(start..=end);
                    self.jobs = parser::extract_jobs(&self.crontab_lines);
                    self.all_tags = self.collect_all_tags();
                    self.apply_filters();
                    self.show_delete_confirm = None;
                    info!(job_id = id.0, "Job deleted");
                    return self.update(Message::SaveCrontab);
                }
                Task::none()
            }

            Message::CancelDelete => {
                self.show_delete_confirm = None;
                Task::none()
            }

            // Draft editing
            Message::DraftCommandChanged(cmd) => {
                self.draft.command = cmd;
                Task::none()
            }

            Message::DraftCommentChanged(comment) => {
                self.draft.comment = comment;
                Task::none()
            }

            Message::DraftTagsChanged(tags) => {
                self.draft.tags_input = tags;
                Task::none()
            }

            Message::DraftEnabledToggled(enabled) => {
                self.draft.enabled = enabled;
                Task::none()
            }

            Message::ExpressionRawChanged(expr) => {
                self.draft.expression = expr.clone();
                self.update_expression_preview(&expr);
                Task::none()
            }

            Message::SwitchExpressionMode(mode) => {
                match mode {
                    ExpressionMode::Visual => {
                        if let Some(fields) = expression::decompose(&self.draft.expression) {
                            self.schedule_fields = fields;
                        } else {
                            self.schedule_fields = ScheduleFields::default();
                        }
                    }
                    ExpressionMode::Raw => {
                        let expr = self.schedule_fields.to_expression();
                        self.draft.expression = expr.clone();
                        self.update_expression_preview(&expr);
                    }
                }
                self.expression_mode = mode;
                Task::none()
            }

            Message::MinuteFieldChanged(val) => {
                self.update_visual_field(0, &val);
                Task::none()
            }
            Message::HourFieldChanged(val) => {
                self.update_visual_field(1, &val);
                Task::none()
            }
            Message::DayOfMonthFieldChanged(val) => {
                self.update_visual_field(2, &val);
                Task::none()
            }
            Message::MonthFieldChanged(val) => {
                self.update_visual_field(3, &val);
                Task::none()
            }
            Message::DayOfWeekFieldChanged(val) => {
                self.update_visual_field(4, &val);
                Task::none()
            }

            Message::SaveJob => {
                // Validate
                if self.draft.command.trim().is_empty() {
                    self.validation_error = Some("Command cannot be empty".into());
                    return Task::none();
                }
                if self.expression_mode == ExpressionMode::Visual {
                    self.draft.expression = self.schedule_fields.to_expression();
                }
                if self.draft.expression.trim().is_empty() {
                    self.validation_error = Some("Expression cannot be empty".into());
                    return Task::none();
                }
                if let Err(e) = expression::validate(&self.draft.expression) {
                    self.validation_error = Some(e.user_message());
                    return Task::none();
                }

                match self.editing_job_id {
                    Some(id) => {
                        // Update existing job
                        if let Some(job) = self.jobs.iter().find(|j| j.id == id) {
                            let (start, new_lines) =
                                writer::update_job_from_draft(&self.crontab_lines, job.line_index, &self.draft);
                            let end = job.line_index;
                            self.crontab_lines.drain(start..=end);
                            for (i, line) in new_lines.into_iter().enumerate() {
                                self.crontab_lines.insert(start + i, line);
                            }
                        }
                    }
                    None => {
                        // Create new job — append to end
                        let new_lines = writer::build_lines_from_draft(&self.draft);
                        for line in new_lines {
                            self.crontab_lines.push(line);
                        }
                    }
                }

                self.jobs = parser::extract_jobs(&self.crontab_lines);
                self.all_tags = self.collect_all_tags();
                self.apply_filters();
                self.current_view = View::JobList;
                self.validation_error = None;
                info!("Job saved");

                self.update(Message::SaveCrontab)
            }

            Message::CancelEdit => {
                self.current_view = View::JobList;
                self.validation_error = None;
                Task::none()
            }
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let header = self.view_header();

        let content: Element<Message> = match &self.current_view {
            View::JobList => self.view_job_list(),
            View::CreateJob | View::EditJob(_) => self.view_job_editor(),
        };

        let status = self.view_status_bar();

        column![header, content, status]
            .spacing(0)
            .into()
    }

    // ─── Private helpers ───

    fn view_header(&self) -> Element<'_, Message> {
        let title = text("Cron Manager").size(font_size::TITLE);

        let nav_buttons = match &self.current_view {
            View::JobList => row![
                button("Reload").on_press(Message::LoadCrontab),
                button("+ New Job")
                    .on_press(Message::NavigateTo(View::CreateJob))
                    .style(button::primary),
            ]
            .spacing(spacing::SMALL),

            View::CreateJob | View::EditJob(_) => row![
                button("Cancel").on_press(Message::CancelEdit),
                button("Save")
                    .on_press(Message::SaveJob)
                    .style(button::primary),
            ]
            .spacing(spacing::SMALL),
        };

        container(
            row![title, horizontal_space(), nav_buttons]
                .align_y(Center)
                .padding(spacing::LARGE),
        )
        .style(container::rounded_box)
        .width(Fill)
        .into()
    }

    fn view_status_bar(&self) -> Element<'_, Message> {
        if let Some(ref msg) = self.status_message {
            let color = match msg.level {
                StatusLevel::Info => [0.6, 0.6, 0.6],
                StatusLevel::Success => [0.3, 0.8, 0.3],
                StatusLevel::Error => [0.9, 0.3, 0.3],
            };
            container(
                text(&msg.text)
                    .size(font_size::SMALL)
                    .color(color),
            )
            .padding(spacing::SMALL)
            .width(Fill)
            .into()
        } else {
            vertical_space().height(0).into()
        }
    }

    fn view_job_list(&self) -> Element<'_, Message> {
        // Search bar
        let search = text_input("Search jobs...", &self.search_query)
            .on_input(Message::SearchChanged)
            .padding(spacing::SMALL);

        // Tag filter
        let mut tag_options = vec!["All tags".to_string()];
        tag_options.extend(self.all_tags.clone());

        let tag_filter_display = self.tag_filter.clone().unwrap_or("All tags".to_string());
        let tag_filter = pick_list(
            tag_options,
            Some(tag_filter_display),
            |selected| {
                if selected == "All tags" {
                    Message::ClearTagFilter
                } else {
                    Message::TagFilterChanged(selected)
                }
            },
        );

        let filter_row = row![search, tag_filter]
            .spacing(spacing::SMALL)
            .padding(spacing::MEDIUM);

        // Job cards
        let mut job_list = column![].spacing(spacing::SMALL).padding(spacing::MEDIUM);

        if self.filtered_job_indices.is_empty() && !self.jobs.is_empty() {
            job_list = job_list.push(
                container(text("No jobs match your search").size(font_size::NORMAL))
                    .padding(spacing::XLARGE)
                    .center_x(Fill),
            );
        } else if self.jobs.is_empty() {
            job_list = job_list.push(
                container(
                    column![
                        text("No cron jobs found").size(font_size::LARGE),
                        vertical_space().height(spacing::SMALL),
                        text("Click '+ New Job' to create one").size(font_size::NORMAL),
                    ]
                    .align_x(Center),
                )
                .padding(spacing::XLARGE)
                .center_x(Fill),
            );
        }

        for &idx in &self.filtered_job_indices {
            if let Some(job) = self.jobs.get(idx) {
                job_list = job_list.push(self.view_job_card(job));
            }
        }

        // Summary
        let active = self.jobs.iter().filter(|j| j.enabled).count();
        let disabled = self.jobs.len() - active;
        let summary = text(format!(
            "{} jobs ({} active, {} disabled)",
            self.jobs.len(),
            active,
            disabled
        ))
        .size(font_size::SMALL)
        .color([0.5, 0.5, 0.5]);

        column![
            filter_row,
            horizontal_rule(1),
            scrollable(job_list).height(Fill),
            horizontal_rule(1),
            container(summary).padding(spacing::SMALL),
        ]
        .into()
    }

    fn view_job_card<'a>(&self, job: &'a CronJob) -> Element<'a, Message> {
        let enabled_checkbox = checkbox("", job.enabled)
            .on_toggle(move |_| Message::ToggleJobEnabled(job.id));

        let expr_text = text(&job.expression)
            .size(font_size::NORMAL);

        let cmd_text = text(&job.command)
            .size(font_size::NORMAL)
            .color([0.7, 0.7, 0.7]);

        let desc = expression::describe(&job.expression).unwrap_or_default();
        let desc_text = text(desc)
            .size(font_size::SMALL)
            .color([0.5, 0.7, 0.9]);

        let mut tags_row = row![].spacing(spacing::TINY);
        for tag in &job.tags {
            tags_row = tags_row.push(
                container(text(tag).size(font_size::SMALL))
                    .padding([2, 6])
                    .style(container::rounded_box),
            );
        }

        if let Some(ref description) = job.description {
            tags_row = tags_row.push(
                text(description)
                    .size(font_size::SMALL)
                    .color([0.6, 0.6, 0.6]),
            );
        }

        let job_id = job.id;
        let edit_btn = button("Edit")
            .on_press(Message::NavigateTo(View::EditJob(job_id)))
            .style(button::secondary);

        let delete_btn = if self.show_delete_confirm == Some(job_id) {
            row![
                button("Confirm Delete")
                    .on_press(Message::ConfirmDeleteJob(job_id))
                    .style(button::danger),
                button("Cancel")
                    .on_press(Message::CancelDelete),
            ]
            .spacing(spacing::TINY)
        } else {
            row![
                button("Delete")
                    .on_press(Message::RequestDeleteJob(job_id))
                    .style(button::danger),
            ]
        };

        let top_row = row![
            enabled_checkbox,
            expr_text,
            horizontal_space(),
            cmd_text,
        ]
        .spacing(spacing::SMALL)
        .align_y(Center);

        let bottom_row = row![
            desc_text,
            tags_row,
            horizontal_space(),
            edit_btn,
            delete_btn,
        ]
        .spacing(spacing::SMALL)
        .align_y(Center);

        container(
            column![top_row, bottom_row]
                .spacing(spacing::TINY)
                .padding(spacing::MEDIUM),
        )
        .style(container::rounded_box)
        .width(Fill)
        .into()
    }

    fn view_job_editor(&self) -> Element<'_, Message> {
        let is_editing = self.editing_job_id.is_some();
        let title_text = if is_editing { "Edit Job" } else { "New Job" };

        let title = text(title_text).size(font_size::LARGE);

        // Command input
        let command_input = column![
            text("Command").size(font_size::SMALL),
            text_input("/usr/bin/my-script.sh --arg", &self.draft.command)
                .on_input(Message::DraftCommandChanged)
                .padding(spacing::SMALL),
        ]
        .spacing(spacing::TINY);

        // Description
        let comment_input = column![
            text("Description").size(font_size::SMALL),
            text_input("What does this job do?", &self.draft.comment)
                .on_input(Message::DraftCommentChanged)
                .padding(spacing::SMALL),
        ]
        .spacing(spacing::TINY);

        // Tags
        let tags_input = column![
            text("Tags (comma-separated)").size(font_size::SMALL),
            text_input("backup, daily, system", &self.draft.tags_input)
                .on_input(Message::DraftTagsChanged)
                .padding(spacing::SMALL),
        ]
        .spacing(spacing::TINY);

        // Enabled toggle
        let enabled = checkbox("Enabled", self.draft.enabled)
            .on_toggle(Message::DraftEnabledToggled);

        // Expression section
        let mode_toggle = row![
            button(if self.expression_mode == ExpressionMode::Raw {
                "> Raw Expression"
            } else {
                "  Raw Expression"
            })
            .on_press(Message::SwitchExpressionMode(ExpressionMode::Raw))
            .style(if self.expression_mode == ExpressionMode::Raw {
                button::primary
            } else {
                button::secondary
            }),
            button(if self.expression_mode == ExpressionMode::Visual {
                "> Visual Builder"
            } else {
                "  Visual Builder"
            })
            .on_press(Message::SwitchExpressionMode(ExpressionMode::Visual))
            .style(if self.expression_mode == ExpressionMode::Visual {
                button::primary
            } else {
                button::secondary
            }),
        ]
        .spacing(spacing::SMALL);

        let expression_editor: Element<Message> = match self.expression_mode {
            ExpressionMode::Raw => {
                column![
                    text_input("*/5 * * * *", &self.draft.expression)
                        .on_input(Message::ExpressionRawChanged)
                        .padding(spacing::SMALL),
                ]
                .into()
            }
            ExpressionMode::Visual => self.view_visual_builder(),
        };

        // Preview section
        let mut preview = column![].spacing(spacing::TINY);
        if !self.description_preview.is_empty() {
            preview = preview.push(
                text(format!("Schedule: {}", self.description_preview))
                    .size(font_size::NORMAL)
                    .color([0.5, 0.8, 0.5]),
            );
        }

        if !self.next_executions.is_empty() {
            preview = preview.push(
                text("Next executions:")
                    .size(font_size::SMALL)
                    .color([0.6, 0.6, 0.6]),
            );
            for exec in &self.next_executions {
                preview = preview.push(
                    text(format!("  {}", exec))
                        .size(font_size::SMALL)
                        .color([0.6, 0.6, 0.6]),
                );
            }
        }

        // Validation error
        let error_display: Element<Message> = if let Some(ref err) = self.validation_error {
            text(err)
                .size(font_size::NORMAL)
                .color([0.9, 0.3, 0.3])
                .into()
        } else {
            vertical_space().height(0).into()
        };

        let generated_expr: Element<Message> = if self.expression_mode == ExpressionMode::Visual {
            let expr = self.schedule_fields.to_expression();
            container(
                text(format!("Generated: {}", expr))
                    .size(font_size::NORMAL)
                    .color([0.7, 0.7, 0.4]),
            )
            .into()
        } else {
            vertical_space().height(0).into()
        };

        scrollable(
            container(
                column![
                    title,
                    vertical_space().height(spacing::MEDIUM),
                    command_input,
                    vertical_space().height(spacing::SMALL),
                    comment_input,
                    vertical_space().height(spacing::SMALL),
                    tags_input,
                    vertical_space().height(spacing::SMALL),
                    enabled,
                    vertical_space().height(spacing::MEDIUM),
                    horizontal_rule(1),
                    vertical_space().height(spacing::MEDIUM),
                    text("Cron Expression").size(font_size::LARGE),
                    vertical_space().height(spacing::SMALL),
                    mode_toggle,
                    vertical_space().height(spacing::SMALL),
                    expression_editor,
                    generated_expr,
                    vertical_space().height(spacing::MEDIUM),
                    preview,
                    error_display,
                ]
                .spacing(0)
                .padding(spacing::LARGE)
                .max_width(700),
            )
            .center_x(Fill),
        )
        .height(Fill)
        .into()
    }

    fn view_visual_builder(&self) -> Element<'_, Message> {
        let minute = self.view_field_picker(
            "Minute",
            &self.schedule_fields.minute.to_string(),
            &Self::field_options(0),
            |v| Message::MinuteFieldChanged(v),
        );
        let hour = self.view_field_picker(
            "Hour",
            &self.schedule_fields.hour.to_string(),
            &Self::field_options(1),
            |v| Message::HourFieldChanged(v),
        );
        let dom = self.view_field_picker(
            "Day of Month",
            &self.schedule_fields.day_of_month.to_string(),
            &Self::field_options(2),
            |v| Message::DayOfMonthFieldChanged(v),
        );
        let month = self.view_field_picker(
            "Month",
            &self.schedule_fields.month.to_string(),
            &Self::field_options(3),
            |v| Message::MonthFieldChanged(v),
        );
        let dow = self.view_field_picker(
            "Day of Week",
            &self.schedule_fields.day_of_week.to_string(),
            &Self::field_options(4),
            |v| Message::DayOfWeekFieldChanged(v),
        );

        column![minute, hour, dom, month, dow]
            .spacing(spacing::SMALL)
            .into()
    }

    fn view_field_picker<'a>(
        &self,
        label: &'a str,
        current: &str,
        options: &[String],
        on_select: impl Fn(String) -> Message + 'a,
    ) -> Element<'a, Message> {
        let label_text = text(label).size(font_size::SMALL).width(120);

        let picker = pick_list(
            options.to_vec(),
            Some(current.to_string()),
            on_select,
        )
        .placeholder("Select...");

        row![label_text, picker]
            .spacing(spacing::SMALL)
            .align_y(Center)
            .into()
    }

    fn field_options(field_index: usize) -> Vec<String> {
        let mut opts = vec!["*".to_string()];
        match field_index {
            0 => {
                // Minute: *, */2, */5, */10, */15, */30, 0-59
                for step in [2, 5, 10, 15, 30] {
                    opts.push(format!("*/{}", step));
                }
                for i in 0..60 {
                    opts.push(i.to_string());
                }
            }
            1 => {
                // Hour: *, */2, */3, */4, */6, */8, */12, 0-23
                for step in [2, 3, 4, 6, 8, 12] {
                    opts.push(format!("*/{}", step));
                }
                for i in 0..24 {
                    opts.push(i.to_string());
                }
            }
            2 => {
                // Day of month: *, 1-31
                for i in 1..=31 {
                    opts.push(i.to_string());
                }
            }
            3 => {
                // Month: *, 1-12
                for i in 1..=12 {
                    opts.push(i.to_string());
                }
            }
            4 => {
                // Day of week: *, 0-6, 1-5 (weekdays)
                opts.push("1-5".to_string());
                opts.push("0,6".to_string());
                for i in 0..=6 {
                    opts.push(i.to_string());
                }
            }
            _ => {}
        }
        opts
    }

    fn update_expression_preview(&mut self, expr: &str) {
        if expr.is_empty() {
            self.description_preview.clear();
            self.next_executions.clear();
            self.validation_error = None;
            return;
        }

        match expression::describe(expr) {
            Ok(desc) => {
                self.description_preview = desc;
                self.validation_error = None;
            }
            Err(e) => {
                self.description_preview.clear();
                self.validation_error = Some(e.user_message());
            }
        }

        match expression::next_occurrences(expr, 5) {
            Ok(times) => {
                self.next_executions = times
                    .iter()
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string())
                    .collect();
            }
            Err(_) => {
                self.next_executions.clear();
            }
        }
    }

    fn update_visual_field(&mut self, field_index: usize, value: &str) {
        use crate::model::schedule::FieldValue;
        if let Some(fv) = FieldValue::parse(value) {
            match field_index {
                0 => self.schedule_fields.minute = fv,
                1 => self.schedule_fields.hour = fv,
                2 => self.schedule_fields.day_of_month = fv,
                3 => self.schedule_fields.month = fv,
                4 => self.schedule_fields.day_of_week = fv,
                _ => {}
            }
            let expr = self.schedule_fields.to_expression();
            self.draft.expression = expr.clone();
            self.update_expression_preview(&expr);
        }
    }

    fn apply_filters(&mut self) {
        let query = self.search_query.to_lowercase();
        self.filtered_job_indices = self
            .jobs
            .iter()
            .enumerate()
            .filter(|(_, job)| {
                if !query.is_empty() {
                    let matches = job.command.to_lowercase().contains(&query)
                        || job.expression.to_lowercase().contains(&query)
                        || job
                            .description
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&query)
                        || job.tags.iter().any(|t| t.to_lowercase().contains(&query));
                    if !matches {
                        return false;
                    }
                }
                if let Some(ref tag) = self.tag_filter {
                    if !job.tags.iter().any(|t| t == tag) {
                        return false;
                    }
                }
                true
            })
            .map(|(i, _)| i)
            .collect();
    }

    fn collect_all_tags(&self) -> Vec<String> {
        let mut tags: Vec<String> = self
            .jobs
            .iter()
            .flat_map(|j| j.tags.iter().cloned())
            .collect();
        tags.sort();
        tags.dedup();
        tags
    }
}
