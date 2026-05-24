use crate::i18n::Messages;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

const BAR_WIDTH: usize = 38;
const SCAN_WIDTH: usize = 44;

pub fn search_loader_widget<'a>(
    frame: usize,
    initial_date: &'a str,
    final_date: &'a str,
    text: &'static Messages,
) -> Paragraph<'a> {
    let stages = [
        text.search_loader_stage_query,
        text.search_loader_stage_pdf,
        text.search_loader_stage_json,
        text.search_loader_stage_match,
    ];
    let stage = stages[(frame / 8) % stages.len()];
    let spinner = ["|", "/", "-", "\\"][frame % 4];
    let progress = (frame * 3) % 100;

    Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                spinner,
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                text.searching_gmail,
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(stage, Style::default().fg(Color::LightGreen)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("{}: ", text.initial_date),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(initial_date, Style::default().fg(Color::Cyan)),
            Span::raw("   "),
            Span::styled(
                format!("{}: ", text.final_date),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(final_date, Style::default().fg(Color::Green)),
        ]),
        Line::from(""),
        meter_line(text.search_loader_meter, progress, frame),
        scan_line(text.search_loader_lane_pdf, frame),
        scan_line(text.search_loader_lane_json, frame + 11),
        scan_line(text.search_loader_lane_match, frame + 22),
        Line::from(""),
        Line::from(text.search_loader_footer).style(Style::default().fg(Color::DarkGray)),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Blue))
            .title(format!(" {} ", text.search_loader_title)),
    )
    .wrap(Wrap { trim: true })
}

fn meter_line<'a>(label: &'static str, progress: usize, frame: usize) -> Line<'a> {
    let filled = (BAR_WIDTH * progress / 100).max(1);
    let mut spans = vec![lane_label(label)];

    spans.push(Span::raw("["));
    for index in 0..BAR_WIDTH {
        let style = if index < filled {
            let color = if (index + frame) % 5 == 0 {
                Color::LightYellow
            } else {
                Color::LightCyan
            };
            Style::default().fg(color).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(if index < filled { "#" } else { "." }, style));
    }
    spans.push(Span::raw(format!("] {:02}%", progress)));

    Line::from(spans)
}

fn scan_line<'a>(label: &'static str, frame: usize) -> Line<'a> {
    let head = frame % SCAN_WIDTH;
    let mut spans = vec![lane_label(label), Span::raw("[")];

    for index in 0..SCAN_WIDTH {
        let distance = head.abs_diff(index);
        let (character, style) = match distance {
            0 => (
                ">",
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            ),
            1 | 2 => ("=", Style::default().fg(Color::LightGreen)),
            3 | 4 => ("-", Style::default().fg(Color::Cyan)),
            _ => (".", Style::default().fg(Color::DarkGray)),
        };
        spans.push(Span::styled(character, style));
    }

    spans.push(Span::raw("]"));
    Line::from(spans)
}

fn lane_label(label: &'static str) -> Span<'static> {
    Span::styled(
        format!("{label:<10} "),
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )
}
