use std::borrow::Cow;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

use super::release_notes::release_notes_close_button_rect;
use super::scrollbar::{release_notes_scrollbar_rect, render_scrollbar};
use super::widgets::{
    modal_stack_areas, panel_contrast_fg, render_action_button, render_modal_header,
    render_modal_shell,
};
use crate::app::AppState;
use crate::i18n::{tr, TranslationKey};

pub(super) type HelpEntry = (String, Cow<'static, str>);
pub(super) type HelpGroup = (&'static str, Vec<HelpEntry>);

fn help_entry(key: impl Into<String>, label: &'static str) -> HelpEntry {
    (key.into(), Cow::Borrowed(label))
}

fn keybind_label(bindings: &crate::config::ActionKeybinds) -> String {
    bindings
        .label()
        .unwrap_or_else(|| tr(TranslationKey::Unset).to_string())
}

fn indexed_label(bindings: &[crate::config::IndexedKeybind]) -> String {
    if bindings.is_empty() {
        return tr(TranslationKey::Unset).to_string();
    }

    let mut parts = Vec::new();
    let mut index = 0;
    while index < bindings.len() {
        if let Some(prefix) = indexed_range_prefix(&bindings[index..]) {
            parts.push(format!("{prefix}1..9"));
            index += 9;
        } else {
            parts.push(bindings[index].label.clone());
            index += 1;
        }
    }

    parts.join(" / ")
}

fn indexed_range_prefix(bindings: &[crate::config::IndexedKeybind]) -> Option<&str> {
    let run = bindings.get(..9)?;
    let prefix = run[0].label.strip_suffix('1')?;
    for (offset, binding) in run.iter().enumerate() {
        let digit = char::from(b'1' + offset as u8);
        if binding.label.strip_suffix(digit) != Some(prefix) {
            return None;
        }
    }
    Some(prefix)
}

pub(super) fn keybind_help_groups(app: &AppState) -> Vec<HelpGroup> {
    let kb = &app.keybinds;
    let mut groups = Vec::new();

    groups.push((
        tr(TranslationKey::GlobalGroup),
        vec![
            help_entry(
                crate::config::format_key_combo((app.prefix_code, app.prefix_mods)),
                tr(TranslationKey::PrefixMode),
            ),
            help_entry(keybind_label(&kb.help), tr(TranslationKey::KeybindsLabel)),
            help_entry(keybind_label(&kb.settings), tr(TranslationKey::Settings)),
            help_entry(keybind_label(&kb.detach), tr(TranslationKey::Detach)),
            help_entry(
                keybind_label(&kb.reload_config),
                tr(TranslationKey::ReloadConfig),
            ),
            help_entry(
                keybind_label(&kb.open_notification_target),
                tr(TranslationKey::OpenNotificationTarget),
            ),
        ],
    ));

    groups.push((
        tr(TranslationKey::NavigationGroup),
        vec![
            help_entry("esc", tr(TranslationKey::Back)),
            help_entry(
                format!(
                    "{} / {}",
                    keybind_label(&kb.navigate.workspace_up),
                    keybind_label(&kb.navigate.workspace_down)
                ),
                tr(TranslationKey::WorkspaceList),
            ),
            help_entry(
                format!(
                    "{} / {} / {} / {} / left / right",
                    keybind_label(&kb.navigate.pane_left),
                    keybind_label(&kb.navigate.pane_down),
                    keybind_label(&kb.navigate.pane_up),
                    keybind_label(&kb.navigate.pane_right)
                ),
                tr(TranslationKey::MoveFocus),
            ),
            help_entry("tab / shift+tab", tr(TranslationKey::CyclePane)),
            help_entry("enter", tr(TranslationKey::OpenWorkspace)),
            help_entry("1..9", tr(TranslationKey::SwitchWorkspace)),
        ],
    ));

    let workspace_tab = vec![
        help_entry(
            keybind_label(&kb.workspace_picker),
            tr(TranslationKey::WorkspaceNavigation),
        ),
        help_entry(
            keybind_label(&kb.goto),
            tr(TranslationKey::SessionNavigator),
        ),
        help_entry(
            keybind_label(&kb.new_workspace),
            tr(TranslationKey::NewWorkspace),
        ),
        help_entry(
            keybind_label(&kb.new_worktree),
            tr(TranslationKey::NewWorktree),
        ),
        help_entry(
            keybind_label(&kb.open_worktree),
            tr(TranslationKey::OpenWorktree),
        ),
        help_entry(
            keybind_label(&kb.remove_worktree),
            tr(TranslationKey::DeleteWorktreeCheckout),
        ),
        help_entry(
            keybind_label(&kb.rename_workspace),
            tr(TranslationKey::RenameWorkspace),
        ),
        help_entry(
            keybind_label(&kb.close_workspace),
            tr(TranslationKey::CloseWorkspace),
        ),
        help_entry(
            keybind_label(&kb.previous_workspace),
            tr(TranslationKey::PreviousWorkspace),
        ),
        help_entry(
            keybind_label(&kb.next_workspace),
            tr(TranslationKey::NextWorkspace),
        ),
        help_entry(
            indexed_label(&kb.switch_workspace),
            tr(TranslationKey::SwitchWorkspace19),
        ),
        help_entry(
            keybind_label(&kb.previous_agent),
            tr(TranslationKey::PreviousAgent),
        ),
        help_entry(keybind_label(&kb.next_agent), tr(TranslationKey::NextAgent)),
        help_entry(
            indexed_label(&kb.focus_agent),
            tr(TranslationKey::FocusAgent19),
        ),
        help_entry(keybind_label(&kb.new_tab), tr(TranslationKey::NewTab)),
        help_entry(keybind_label(&kb.rename_tab), tr(TranslationKey::RenameTab)),
        help_entry(
            keybind_label(&kb.previous_tab),
            tr(TranslationKey::PreviousTab),
        ),
        help_entry(keybind_label(&kb.next_tab), tr(TranslationKey::NextTab)),
        help_entry(
            indexed_label(&kb.switch_tab),
            tr(TranslationKey::SwitchTab19),
        ),
        help_entry(keybind_label(&kb.close_tab), tr(TranslationKey::CloseTab)),
    ];
    groups.push((tr(TranslationKey::WorkspacesTabsGroup), workspace_tab));

    let panes = vec![
        help_entry(
            keybind_label(&kb.split_vertical),
            tr(TranslationKey::SplitVertical),
        ),
        help_entry(
            keybind_label(&kb.split_horizontal),
            tr(TranslationKey::SplitHorizontal),
        ),
        help_entry(keybind_label(&kb.close_pane), tr(TranslationKey::ClosePane)),
        help_entry(
            keybind_label(&kb.rename_pane),
            tr(TranslationKey::RenamePane),
        ),
        help_entry(
            keybind_label(&kb.edit_scrollback),
            tr(TranslationKey::EditScrollback),
        ),
        help_entry(keybind_label(&kb.copy_mode), tr(TranslationKey::CopyMode)),
        help_entry(keybind_label(&kb.zoom), tr(TranslationKey::ZoomPane)),
        help_entry(
            keybind_label(&kb.resize_mode),
            tr(TranslationKey::ResizeMode),
        ),
        help_entry(
            keybind_label(&kb.toggle_sidebar),
            tr(TranslationKey::ToggleSidebar),
        ),
        help_entry(
            keybind_label(&kb.focus_pane_left),
            tr(TranslationKey::FocusPaneLeft),
        ),
        help_entry(
            keybind_label(&kb.focus_pane_down),
            tr(TranslationKey::FocusPaneDown),
        ),
        help_entry(
            keybind_label(&kb.focus_pane_up),
            tr(TranslationKey::FocusPaneUp),
        ),
        help_entry(
            keybind_label(&kb.focus_pane_right),
            tr(TranslationKey::FocusPaneRight),
        ),
        help_entry(
            keybind_label(&kb.cycle_pane_next),
            tr(TranslationKey::CyclePaneNext),
        ),
        help_entry(
            keybind_label(&kb.cycle_pane_previous),
            tr(TranslationKey::CyclePanePrevious),
        ),
        help_entry(keybind_label(&kb.last_pane), tr(TranslationKey::LastPane)),
    ];
    groups.push((tr(TranslationKey::PanesGroup), panes));

    if !kb.custom_commands.is_empty() {
        groups.push((
            tr(TranslationKey::CustomGroup),
            kb.custom_commands
                .iter()
                .map(|binding| {
                    (
                        binding.label.clone(),
                        binding
                            .description
                            .clone()
                            .map(Cow::Owned)
                            .unwrap_or_else(|| {
                                Cow::Owned(tr(TranslationKey::CustomCommand).to_string())
                            }),
                    )
                })
                .collect(),
        ));
    }

    groups
}

fn filter_keybind_help_groups(groups: Vec<HelpGroup>, query: &str) -> Vec<HelpGroup> {
    if query.is_empty() {
        return groups;
    }

    let query = query.to_lowercase();
    groups
        .into_iter()
        .filter_map(|(group, entries)| {
            let entries = entries
                .into_iter()
                .filter(|(key, label)| {
                    key.to_lowercase().contains(&query) || label.to_lowercase().contains(&query)
                })
                .collect::<Vec<_>>();
            (!entries.is_empty()).then_some((group, entries))
        })
        .collect()
}

pub(crate) fn keybind_help_lines(app: &AppState) -> Vec<(usize, Line<'static>)> {
    let heading_style = Style::default()
        .fg(app.palette.accent)
        .add_modifier(Modifier::BOLD);
    let key_style = Style::default()
        .fg(app.palette.mauve)
        .add_modifier(Modifier::BOLD);
    let label_style = Style::default().fg(app.palette.text);

    let groups = filter_keybind_help_groups(keybind_help_groups(app), &app.keybind_help.query);
    let key_width = groups
        .iter()
        .flat_map(|(_, entries)| {
            entries
                .iter()
                .map(|(key, _)| crate::ui::display_width_u16(key))
        })
        .max()
        .unwrap_or(8);

    let mut lines = Vec::new();

    if groups.is_empty() {
        let label = tr(TranslationKey::NoMatchingKeybinds);
        let style = Style::default().fg(app.palette.overlay1);
        return vec![(
            crate::ui::display_width_u16(label) as usize + 1,
            Line::from(vec![Span::styled(" ", style), Span::styled(label, style)]),
        )];
    }

    for (group, entries) in groups {
        lines.push((
            crate::ui::display_width_u16(group) as usize + 1,
            Line::from(vec![Span::styled(format!(" {group}"), heading_style)]),
        ));
        for (key, label) in entries {
            let padding = key_width.saturating_sub(crate::ui::display_width_u16(&key));
            let padded_key = format!(" {}{} ", key, " ".repeat(padding as usize));
            let label = label.into_owned();
            let width = crate::ui::display_width_u16(&padded_key) as usize
                + crate::ui::display_width_u16(&label) as usize;
            lines.push((
                width,
                Line::from(vec![
                    Span::styled(padded_key, key_style),
                    Span::styled(label, label_style),
                ]),
            ));
        }
        lines.push((0, Line::raw("")));
    }

    lines
}

pub(super) fn render_keybind_help_overlay(app: &AppState, frame: &mut Frame) {
    super::dim_background(frame, frame.area());

    let Some(inner) = render_modal_shell(frame, frame.area(), 76, 22, &app.palette) else {
        return;
    };
    if inner.height < 6 || inner.width < 20 {
        return;
    }

    let stack = modal_stack_areas(inner, 2, 1, 0, 1);
    let header_rows =
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas::<2>(stack.header);

    render_modal_header(
        frame,
        header_rows[0],
        tr(TranslationKey::KeybindsLabel),
        &app.palette,
    );
    render_action_button(
        frame,
        release_notes_close_button_rect(header_rows[0]),
        Some("esc"),
        if app.keybind_help.search_focused {
            tr(TranslationKey::Back)
        } else {
            tr(TranslationKey::Close)
        },
        Style::default()
            .fg(panel_contrast_fg(&app.palette))
            .bg(app.palette.accent)
            .add_modifier(Modifier::BOLD),
    );
    let search_line = if app.keybind_help.search_focused {
        Line::from(vec![
            Span::styled(
                " / ",
                Style::default()
                    .fg(app.palette.accent)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                app.keybind_help.query.as_str(),
                Style::default()
                    .fg(app.palette.text)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    } else {
        Line::from(Span::styled(
            format!(" {}", tr(TranslationKey::PressSlashToFilter)),
            Style::default().fg(app.palette.overlay0),
        ))
    };
    frame.render_widget(Paragraph::new(search_line), header_rows[1]);

    let body_area = stack.content;
    let metrics = crate::pane::ScrollMetrics {
        offset_from_bottom: app
            .keybind_help_max_scroll()
            .saturating_sub(app.keybind_help.scroll) as usize,
        max_offset_from_bottom: app.keybind_help_max_scroll() as usize,
        viewport_rows: body_area.height.max(1) as usize,
    };
    let track = release_notes_scrollbar_rect(body_area, metrics);
    let text_area = track
        .map(|_| {
            Rect::new(
                body_area.x,
                body_area.y,
                body_area.width.saturating_sub(1),
                body_area.height,
            )
        })
        .unwrap_or(body_area);

    let body = Paragraph::new(
        keybind_help_lines(app)
            .into_iter()
            .map(|(_, line)| line)
            .collect::<Vec<_>>(),
    )
    .wrap(Wrap { trim: false })
    .scroll((app.keybind_help.scroll, 0));
    frame.render_widget(body, text_area);
    if let Some(track) = track {
        render_scrollbar(
            frame,
            metrics,
            track,
            app.palette.overlay0,
            app.palette.overlay1,
            "▐",
        );
    }

    let footer = if app.keybind_help.search_focused {
        Line::from(vec![
            Span::styled(
                format!(" {} ", tr(TranslationKey::FilterLabel)),
                Style::default().fg(app.palette.overlay0),
            ),
            Span::styled("type/backspace", Style::default().fg(app.palette.text)),
            Span::styled(" · ", Style::default().fg(app.palette.overlay0)),
            Span::styled(
                format!("{} ", tr(TranslationKey::ClearLabel)),
                Style::default().fg(app.palette.overlay0),
            ),
            Span::styled("ctrl+u", Style::default().fg(app.palette.text)),
            Span::styled(" · ", Style::default().fg(app.palette.overlay0)),
            Span::styled(
                format!("{} ", tr(TranslationKey::ScrollLabel)),
                Style::default().fg(app.palette.overlay0),
            ),
            Span::styled("↑↓/pgup/pgdn", Style::default().fg(app.palette.text)),
            Span::styled(" · ", Style::default().fg(app.palette.overlay0)),
            Span::styled(
                format!("{} ", tr(TranslationKey::Back)),
                Style::default().fg(app.palette.overlay0),
            ),
            Span::styled("esc", Style::default().fg(app.palette.text)),
        ])
    } else {
        Line::from(vec![
            Span::styled(
                format!(" {} ", tr(TranslationKey::SearchLabel)),
                Style::default().fg(app.palette.overlay0),
            ),
            Span::styled("/", Style::default().fg(app.palette.text)),
            Span::styled(" · ", Style::default().fg(app.palette.overlay0)),
            Span::styled(
                format!("{} ", tr(TranslationKey::ScrollLabel)),
                Style::default().fg(app.palette.overlay0),
            ),
            Span::styled("j/k/↑↓/pgup/pgdn", Style::default().fg(app.palette.text)),
            Span::styled(" · ", Style::default().fg(app.palette.overlay0)),
            Span::styled(
                format!("{} ", tr(TranslationKey::Close)),
                Style::default().fg(app.palette.overlay0),
            ),
            Span::styled("esc/enter", Style::default().fg(app.palette.text)),
        ])
    };
    frame.render_widget(Paragraph::new(footer), stack.footer.unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn groups() -> Vec<HelpGroup> {
        vec![
            (
                "workspaces / tabs",
                vec![
                    help_entry("w", "workspace navigation"),
                    help_entry("c", "new tab"),
                ],
            ),
            (
                "panes",
                vec![
                    help_entry("v", "split vertical"),
                    help_entry("x", "close pane"),
                ],
            ),
        ]
    }

    #[test]
    fn keybind_help_filter_matches_labels_case_insensitively() {
        let filtered = filter_keybind_help_groups(groups(), "WoRk");

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "workspaces / tabs");
        assert_eq!(filtered[0].1.len(), 1);
        assert_eq!(filtered[0].1[0].1, "workspace navigation");
    }

    #[test]
    fn keybind_help_filter_matches_shortcuts_without_matching_group_headings() {
        let filtered = filter_keybind_help_groups(groups(), "x");

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "panes");
        assert_eq!(filtered[0].1.len(), 1);
        assert_eq!(filtered[0].1[0].1, "close pane");

        assert!(filter_keybind_help_groups(groups(), "panes").is_empty());
    }
}
