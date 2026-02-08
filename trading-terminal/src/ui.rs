use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line as TextLine, Span},
    widgets::{
        Block, BorderType, Borders, Paragraph, Row, Table,
        canvas::{Canvas, Line, Rectangle},
    },
};

use crate::app::{App, Theme};

pub fn ui(f: &mut Frame, app: &App) {
    let (bg_color, fg_color, border_color) = match app.theme {
        Theme::Light => (Color::White, Color::Black, Color::Black),
        Theme::Dark => (Color::Rgb(20, 20, 25), Color::White, Color::DarkGray),
    };

    let base_style = Style::default().bg(bg_color).fg(fg_color);

    // Fill background
    let size = f.size();
    f.render_widget(Block::default().style(base_style), size);

    // Top-level Layout
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(size);

    render_left_sidebar(f, app, main_layout[0], border_color, fg_color);

    // Center
    let center_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(main_layout[1]);

    render_chart_area(f, app, center_layout[0], border_color, fg_color);
    render_trade_history(f, app, center_layout[1], border_color, fg_color);

    render_right_sidebar(f, app, main_layout[2], border_color, fg_color);
}

fn render_left_sidebar(f: &mut Frame, app: &App, area: Rect, border: Color, text: Color) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // Top Metrics (MC, FDV, Liq, Holders, Fees, Org)
            Constraint::Length(3),  // Bonding Curve
            Constraint::Length(12), // Stats Grid (Timeframes, Vol, Net Vol, etc)
            Constraint::Min(0),     // Deltas (Vol %, Liq %, etc)
        ])
        .split(area);

    // --- Top Metrics ---
    // Two columns for top metrics
    let top_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    let col1_text = vec![
        TextLine::from(vec![
            Span::raw("MC"),
            Span::styled(
                format!(" ${:.2}K", app.token_info.market_cap),
                Style::default().fg(text).add_modifier(Modifier::BOLD),
            ),
        ]),
        TextLine::from(""),
        TextLine::from(vec![
            Span::raw("FDV"),
            Span::styled(
                format!(" ${:.2}K", app.token_info.fdv),
                Style::default().fg(text).add_modifier(Modifier::BOLD),
            ),
        ]),
        TextLine::from(""),
        TextLine::from(vec![
            Span::raw("Liquidity"),
            Span::styled(
                format!(" ${:.5}", app.token_info.liquidity),
                Style::default().fg(Color::Red),
            ),
        ]),
        TextLine::from(""),
        TextLine::from(vec![
            Span::raw("Holders"),
            Span::styled(
                format!(" {}", app.token_info.holders),
                Style::default().fg(text).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];

    let col2_text = vec![
        TextLine::from(vec![
            Span::raw("Fees Paid"),
            Span::styled(
                format!(" SOL {:.3}", app.token_info.fees_paid),
                Style::default().fg(text),
            ),
        ]),
        TextLine::from(""),
        TextLine::from(vec![
            Span::raw("Org Score"),
            Span::styled(
                format!(" {:.2}", app.token_info.org_score),
                Style::default().fg(text),
            ),
        ]),
    ];

    f.render_widget(
        Paragraph::new(col1_text).block(Block::default().borders(Borders::NONE)),
        top_layout[0],
    );
    f.render_widget(
        Paragraph::new(col2_text).block(Block::default().borders(Borders::NONE)),
        top_layout[1],
    );

    // --- Bonding Curve ---
    let bonding_curve_text = vec![TextLine::from(vec![
        Span::raw("Bonding Curve: "),
        Span::styled(
            format!("{}%", app.token_info.bonding_curve),
            Style::default().fg(Color::Yellow),
        ),
    ])];
    f.render_widget(
        Paragraph::new(bonding_curve_text).block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(border)),
        ),
        chunks[1],
    );

    // --- Stats Grid ---
    // --- Stats Grid ---
    // Pre-calculate strings to extend lifetime
    let change_5m = if let Some(v) = app.token_info.change_5m {
        format!("{:.2}%", v)
    } else {
        "-".to_string()
    };
    let change_1h = if let Some(v) = app.token_info.change_1h {
        format!("{:.2}%", v)
    } else {
        "-".to_string()
    };
    let change_6h = format!("{:.2}%", app.token_info.change_6h);
    let change_24h = format!("{:.2}%", app.token_info.change_24h);
    let vol_24h = format!("${:.2}K", app.token_info.vol_24h / 1000.0);
    let net_vol_24h = format!("${}", app.token_info.net_vol_24h);
    let sell_pressure = format!("{}%", app.token_info.sell_pressure);
    let traders_24h = format!("{}", app.token_info.traders_24h);
    let net_buyers = if let Some(v) = app.token_info.net_buyers {
        format!("{}", v)
    } else {
        "-".to_string()
    };
    let net_buy_trend = format!("${:.4}", app.token_info.net_buy_trend_24h);

    let stats_rows = vec![
        Row::new(vec!["5m", change_5m.as_str(), "1h", change_1h.as_str()]),
        Row::new(vec!["6h", change_6h.as_str(), "24h", change_24h.as_str()])
            .style(Style::default().fg(Color::Green)),
        Row::new(vec![
            "24h Vol",
            vol_24h.as_str(),
            "Net Vol",
            net_vol_24h.as_str(),
        ]),
        Row::new(vec!["Sell", sell_pressure.as_str(), "", ""])
            .style(Style::default().fg(Color::Red)),
        Row::new(vec![
            "Traders",
            traders_24h.as_str(),
            "Net Buyers",
            net_buyers.as_str(),
        ]),
        Row::new(vec!["Net Trend", net_buy_trend.as_str(), "", ""]),
    ];

    let stats_table = Table::new(
        stats_rows,
        [
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ],
    )
    .block(Block::default().borders(Borders::NONE));
    f.render_widget(stats_table, chunks[2]);

    // --- Deltas (Bottom) ---
    let delta_text = vec![
        TextLine::from(vec![
            Span::raw("Vol %Δ  "),
            Span::styled(format!("+16x"), Style::default().fg(Color::Green)),
        ]),
        TextLine::from(vec![
            Span::raw("Liq %Δ  "),
            Span::styled(format!("-100%"), Style::default().fg(Color::Red)),
        ]),
        TextLine::from(vec![Span::raw("Holders %Δ + token image")]),
    ];
    f.render_widget(
        Paragraph::new(delta_text).block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(border)),
        ),
        chunks[3],
    );

    // Wrap entire sidebar in a block
    f.render_widget(
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(Style::default().fg(border)),
        area,
    );
}

fn render_chart_area(f: &mut Frame, app: &App, area: Rect, border: Color, _text: Color) {
    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border))
                .title("Chart - RAN/SOL"),
        )
        .x_bounds([0.0, 50.0])
        .y_bounds([0.0035, 0.0045])
        .paint(|ctx| {
            for (i, candle) in app.candles.iter().enumerate() {
                let color = if candle.close >= candle.open {
                    Color::Green
                } else {
                    Color::Red
                };

                // Wick
                ctx.draw(&Line {
                    x1: i as f64,
                    y1: candle.low,
                    x2: i as f64,
                    y2: candle.high,
                    color,
                });

                // Body (Ratatui Line doesn't have thickness, so we use nice hack or just a Rectangle?)
                // Rectangle needs width/height.
                // Note: Rectangle x is left, y is bottom.
                let (bottom, top) = if candle.open < candle.close {
                    (candle.open, candle.close)
                } else {
                    (candle.close, candle.open)
                };

                // Draw a small rectangle for body
                // x range: [i - 0.3, i + 0.3]
                // y range: [bottom, top]
                // Rectangle is defined by x, y, width, height
                // Wait, ratatui Rectangle is struct { x, y, width, height, color }

                // Actually Canvas Rectangle might not be fillable in standard widgets easily without features.
                // Let's check imports. widgets::canvas::Rectangle is available.
                // But standard ratatui canvas shapes are points, lines, map, etc.
                // Rectangle is available in 0.26? Yes.

                ctx.draw(&Rectangle {
                    x: i as f64 - 0.2,
                    y: bottom,
                    width: 0.4,
                    height: (top - bottom).max(0.00001),
                    color,
                });
            }
        });
    f.render_widget(canvas, area);
}

fn render_trade_history(f: &mut Frame, app: &App, area: Rect, border: Color, _text: Color) {
    let rows: Vec<Row> = app
        .recent_trades
        .iter()
        .map(|t| {
            let color = if t.type_ == "Buy" {
                Color::Green
            } else {
                Color::Red
            };
            Row::new(vec![
                t.time.clone(),
                t.type_.clone(),
                format!("{:.7}", t.price),
                format!("{:.2}", t.volume),
                t.maker.clone(),
            ])
            .style(Style::default().fg(color))
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(10),
            Constraint::Percentage(10),
            Constraint::Percentage(30),
            Constraint::Percentage(20),
            Constraint::Percentage(30),
        ],
    )
    .header(
        Row::new(vec!["Age", "Type", "Price", "Vol", "Maker"])
            .style(Style::default().fg(Color::Yellow)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border))
            .title("Transactions"),
    );

    f.render_widget(table, area);
}

fn render_right_sidebar(f: &mut Frame, app: &App, area: Rect, border: Color, _text: Color) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(12), Constraint::Min(10)])
        .split(area);

    let swap_text = vec![
        TextLine::from(vec![
            Span::raw("Sell"),
            Span::styled(" [SOL] -> USDC", Style::default().fg(Color::Blue)),
        ]),
        TextLine::from(""),
        TextLine::from(vec![
            Span::raw("Amount: "),
            Span::styled(
                format!("{} SOL", app.swap_amount),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        TextLine::from(""),
        TextLine::from(vec![Span::styled(
            "[ENTER TO SWAP]",
            Style::default().bg(Color::Green).fg(Color::Black),
        )]),
    ];
    let swap_panel = Paragraph::new(swap_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border))
            .title("Swap"),
    );
    f.render_widget(swap_panel, chunks[0]);

    let profile_text = vec![
        TextLine::from("Safety Check:"),
        TextLine::from(vec![Span::styled(
            "Mint Auth: No",
            Style::default().fg(Color::Green),
        )]),
    ];
    let profile = Paragraph::new(profile_text).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border))
            .title("Info"),
    );
    f.render_widget(profile, chunks[1]);
}
