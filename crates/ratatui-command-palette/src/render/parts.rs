use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget, Wrap};
use ratatui_action::spec::Availability;

use crate::event::PaletteMode;
use crate::view::{PaletteRow, PaletteView};

pub(crate) fn render_query(area: Rect, buf: &mut Buffer, view: &PaletteView) {
    let query = if view.query().is_empty() {
        let placeholder = match view.mode() {
            PaletteMode::Searching => "type to filter",
            PaletteMode::CollectingInput { .. } => "choose a value",
        };
        Span::styled(placeholder, Style::new().fg(Color::DarkGray))
    } else {
        Span::raw(view.query())
    };

    let line = Line::from(vec![
        Span::styled(
            format!("{} ", view.prompt()),
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        query,
    ]);

    Paragraph::new(line).render(area, buf);
}

pub(crate) fn render_rows(area: Rect, buf: &mut Buffer, view: &PaletteView) {
    if view.rows().is_empty() {
        let message = match view.mode() {
            PaletteMode::Searching => "No actions match",
            PaletteMode::CollectingInput { .. } => "Type a value",
        };
        Paragraph::new(Line::from(Span::styled(
            message,
            Style::new().fg(Color::DarkGray),
        )))
        .render(area, buf);
        return;
    }

    let visible_start = visible_start(view.selected(), view.rows().len(), area.height as usize);

    for (index, row) in view
        .rows()
        .iter()
        .enumerate()
        .skip(visible_start)
        .take(area.height as usize)
    {
        let row_area = Rect {
            x: area.x,
            y: area.y + (index - visible_start) as u16,
            width: area.width,
            height: 1,
        };
        let is_selected = view.selected() == Some(index);
        let style = row_style(row.availability(), is_selected);

        buf.set_style(row_area, style);

        let text = row_text(row);
        let text = truncate_to_width(&text, row_area.width as usize);

        Paragraph::new(Line::from(Span::styled(text, style))).render(row_area, buf);
    }
}

pub(crate) fn render_footer(area: Rect, buf: &mut Buffer, view: &PaletteView) {
    let label = match view.mode() {
        PaletteMode::Searching => "Searching",
        PaletteMode::CollectingInput { .. } => "Collecting input",
    };

    let count = match (view.mode(), view.rows().len()) {
        (PaletteMode::CollectingInput { .. }, 0) => "text input".to_string(),
        (PaletteMode::CollectingInput { .. }, 1) => "1 choice".to_string(),
        (PaletteMode::CollectingInput { .. }, count) => format!("{count} choices"),
        (PaletteMode::Searching, 1) => "1 action".to_string(),
        (PaletteMode::Searching, count) => format!("{count} actions"),
    };

    let line = Line::from(vec![
        Span::styled(label, Style::new().fg(Color::Yellow)),
        Span::raw("  "),
        Span::styled(count, Style::new().fg(Color::DarkGray)),
    ]);

    Paragraph::new(line).render(area, buf);
}

pub(crate) fn render_selected_row_details(area: Rect, buf: &mut Buffer, view: &PaletteView) {
    let Some(selected) = view.selected().and_then(|index| view.rows().get(index)) else {
        Paragraph::new(Line::from(Span::styled(
            "No selection",
            Style::new().fg(Color::DarkGray),
        )))
        .render(area, buf);
        return;
    };

    let mut lines = vec![
        Line::from(Span::styled(
            selected.title(),
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            selected.action_id().as_str(),
            Style::new().fg(Color::DarkGray),
        )),
    ];

    if let Some(category) = selected.category() {
        lines.push(Line::from(vec![
            Span::styled("Category  ", Style::new().fg(Color::Yellow)),
            Span::raw(category),
        ]));
    }

    if let Some(shortcut) = selected.shortcut() {
        lines.push(Line::from(vec![
            Span::styled("Shortcut  ", Style::new().fg(Color::Yellow)),
            Span::raw(shortcut),
        ]));
    }

    match selected.availability() {
        Availability::Disabled { reason } => lines.push(Line::from(vec![
            Span::styled("Disabled  ", Style::new().fg(Color::Yellow)),
            Span::raw(reason.as_str()),
        ])),
        Availability::Enabled | Availability::Hidden => {
            if let Some(subtitle) = selected.subtitle() {
                lines.push(Line::from(""));
                lines.push(Line::from(subtitle));
            }
        }
    }

    Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .render(area, buf);
}

pub(crate) fn row_text(row: &PaletteRow) -> String {
    let detail = match row.availability() {
        Availability::Disabled { reason } => Some(reason.as_str()),
        Availability::Enabled | Availability::Hidden => row.subtitle(),
    };

    match (row.category(), detail, row.shortcut()) {
        (Some(category), Some(detail), Some(shortcut)) => {
            format!("{}  {category}  {detail}  {shortcut}", row.title())
        }
        (Some(category), Some(detail), None) => format!("{}  {category}  {detail}", row.title()),
        (Some(category), None, Some(shortcut)) => {
            format!("{}  {category}  {shortcut}", row.title())
        }
        (Some(category), None, None) => format!("{}  {category}", row.title()),
        (None, Some(detail), Some(shortcut)) => format!("{}  {detail}  {shortcut}", row.title()),
        (None, Some(detail), None) => format!("{}  {detail}", row.title()),
        (None, None, Some(shortcut)) => format!("{}  {shortcut}", row.title()),
        (None, None, None) => row.title().to_string(),
    }
}

pub(crate) fn truncate_to_width(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }

    match width {
        0 => String::new(),
        1..=3 => ".".repeat(width),
        width => {
            let mut truncated = text.chars().take(width - 3).collect::<String>();
            truncated.push_str("...");
            truncated
        }
    }
}

fn row_style(availability: &Availability, selected: bool) -> Style {
    match (availability, selected) {
        (Availability::Enabled, true) => Style::new()
            .fg(Color::Black)
            .bg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        (Availability::Enabled, false) => Style::new().fg(Color::White),
        (Availability::Disabled { .. }, true) => Style::new().fg(Color::Gray).bg(Color::DarkGray),
        (Availability::Disabled { .. }, false) => Style::new().fg(Color::DarkGray),
        (Availability::Hidden, _) => Style::new().fg(Color::DarkGray),
    }
}

fn visible_start(selected: Option<usize>, row_count: usize, height: usize) -> usize {
    if row_count == 0 || height == 0 {
        return 0;
    }

    let selected = selected.unwrap_or(0).min(row_count - 1);
    let last_start = row_count.saturating_sub(height);

    if selected >= height {
        (selected + 1).saturating_sub(height).min(last_start)
    } else {
        0
    }
}
