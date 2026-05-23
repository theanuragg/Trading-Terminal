use crate::app::App;
use crate::theme;
use chrono::{TimeZone, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line as TextLine, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

pub fn render_chart_area(f: &mut Frame, app: &mut App, area: Rect, _border: Color, _text: Color) {
    // If no candles, show placeholder
    if app.candles.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme::BORDER))
            .title("Chart - Loading...");

        let p = Paragraph::new("Fetching chart data from Jupiter API...")
            .alignment(ratatui::layout::Alignment::Center)
            .block(block);
        f.render_widget(p, area);
        return;
    }

    let interval_name = match app.chart_interval {
        crate::app::ChartInterval::OneMin => "1min",
        crate::app::ChartInterval::FiveMin => "5min",
        crate::app::ChartInterval::FifteenMin => "15min",
        crate::app::ChartInterval::OneHour => "1h",
        crate::app::ChartInterval::FourHour => "4h",
        crate::app::ChartInterval::OneDay => "1d",
    };

    // Calculate price bounds
    let (min_price, max_price): (f64, f64) =
        app.candles.iter().fold((f64::MAX, 0.0), |(min, max), c| {
            (min.min(c.low), max.max(c.high))
        });

    let price_range = (max_price - min_price).max(0.0001_f64);
    let y_padding = price_range * 0.05;
    let y_min = (min_price - y_padding).max(0.0_f64);
    let y_max = max_price + y_padding;

    // Layout: Y-axis | Chart | Price-scale
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(14),
            Constraint::Min(0),
            Constraint::Length(12),
        ])
        .split(area);
    let chart_with_title = horizontal[1];
    let price_scale_area = horizontal[2];

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(2)])
        .split(chart_with_title);
    let chart_area = vertical[0];
    let time_scale_area = vertical[1];

    // === RENDER PRICE SCALE (Right side, vertical) ===
    let mut price_lines = Vec::new();
    let num_price_ticks = (chart_area.height as usize).min(10);
    for i in 0..num_price_ticks {
        let ratio = 1.0 - (i as f64 / ((num_price_ticks - 1).max(1) as f64));
        let price = y_min + (y_max - y_min) * ratio;
        let price_str = if price < 0.001 {
            format!("{:.8}", price)
        } else if price < 1.0 {
            format!("{:.6}", price)
        } else if price < 100.0 {
            format!("{:.4}", price)
        } else {
            format!("{:.2}", price)
        };
        let formatted = if price_str.len() > 11 {
            price_str[..11].to_string()
        } else {
            format!("{:>11}", price_str)
        };
        price_lines.push(TextLine::from(Span::styled(
            formatted,
            Style::default().fg(theme::TEXT),
        )));
    }
    f.render_widget(
        Paragraph::new(price_lines)
            .alignment(ratatui::layout::Alignment::Right)
            .style(Style::default().fg(theme::TEXT)),
        price_scale_area,
    );

    // === RENDER TIME SCALE (Bottom) ===
    let visible_candles = (chart_area.width as usize / 3).min(40);
    let start_idx = app.candles.len().saturating_sub(visible_candles);
    let end_idx = app.candles.len();
    let candles_to_show = &app.candles[start_idx..end_idx];

    let mut time_labels = String::new();
    if !candles_to_show.is_empty() {
        let first_ts = candles_to_show[0].time as i64;
        let last_ts = candles_to_show[candles_to_show.len() - 1].time as i64;
        let first_dt = Utc
            .timestamp_opt(first_ts, 0)
            .single()
            .unwrap_or_else(|| Utc::now());
        let last_dt = Utc
            .timestamp_opt(last_ts, 0)
            .single()
            .unwrap_or_else(|| Utc::now());
        time_labels = format!(
            "  {}                                    {}",
            first_dt.format("%H:%M:%S"),
            last_dt.format("%H:%M:%S")
        );
    }
    f.render_widget(
        Paragraph::new(time_labels)
            .alignment(ratatui::layout::Alignment::Left)
            .style(Style::default().fg(theme::TEXT)),
        time_scale_area,
    );

    // === RENDER CANDLES (Main chart) ===
    let mut chart_lines: Vec<TextLine> = vec![];
    let chart_height = chart_area.height as usize;
    let chart_width = chart_area.width as usize;

    // Create a matrix to hold colored candle data for each position
    let mut candle_grid: Vec<Vec<(char, Color)>> =
        vec![vec![(' ', Color::White); chart_width]; chart_height];

    // Calculate candle spacing: 2 chars per candle + 1 space gap
    let candle_col_width = 3_usize; // 2 for body/wick + 1 for spacing
    let mut col = 0;

    for candle in candles_to_show {
        if col + 2 >= chart_width {
            break;
        }

        // Normalize prices to chart rows
        let low_normalized = ((candle.low - y_min) / (y_max - y_min)).clamp(0.0, 1.0);
        let high_normalized = ((candle.high - y_min) / (y_max - y_min)).clamp(0.0, 1.0);
        let open_normalized = ((candle.open - y_min) / (y_max - y_min)).clamp(0.0, 1.0);
        let close_normalized = ((candle.close - y_min) / (y_max - y_min)).clamp(0.0, 1.0);

        let low_row = ((1.0 - low_normalized) * (chart_height - 1) as f64) as usize;
        let high_row = ((1.0 - high_normalized) * (chart_height - 1) as f64) as usize;
        let open_row = ((1.0 - open_normalized) * (chart_height - 1) as f64) as usize;
        let close_row = ((1.0 - close_normalized) * (chart_height - 1) as f64) as usize;

        let is_bullish = candle.close >= candle.open;
        let candle_color = if is_bullish {
            theme::POSITIVE
        } else {
            theme::NEGATIVE
        };

        // Draw wick at column 0
        for row in high_row..=low_row {
            if row < chart_height {
                candle_grid[row][col] = ('│', candle_color);
            }
        }

        // Draw body at column 1 (open to close)
        let body_top = open_row.min(close_row);
        let body_bottom = open_row.max(close_row);
        for row in body_top..=body_bottom.min(chart_height - 1) {
            if col + 1 < chart_width {
                candle_grid[row][col + 1] = ('█', candle_color);
            }
        }

        // Column 2 is blank spacing
        col += candle_col_width;
    }

    // Convert grid to text lines with proper coloring
    for row_idx in 0..chart_height {
        let mut spans: Vec<Span> = vec![];
        let mut current_color = Color::White;
        let mut current_str = String::new();

        for col_idx in 0..chart_width {
            let (ch, color) = candle_grid[row_idx][col_idx];
            if color != current_color {
                if !current_str.is_empty() {
                    spans.push(Span::styled(
                        current_str.clone(),
                        Style::default().fg(current_color),
                    ));
                    current_str.clear();
                }
                current_color = color;
            }
            current_str.push(ch);
        }
        if !current_str.is_empty() {
            spans.push(Span::styled(
                current_str,
                Style::default().fg(current_color),
            ));
        }

        chart_lines.push(TextLine::from(spans));
    }

    f.render_widget(
        Paragraph::new(chart_lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(theme::BORDER))
                    .title(format!(
                        "CHART - {} | {} candles",
                        interval_name.to_uppercase(),
                        app.candles.len()
                    )),
            )
            .style(Style::default()),
        chart_area,
    );
}
