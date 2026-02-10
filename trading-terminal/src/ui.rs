use crate::app::{App, CurrentScreen, Theme};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line as TextLine, Span},
    widgets::{
        Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph, Row, Table, Tabs,
        canvas::{Canvas, Line, Rectangle},
    },
};

pub fn ui(f: &mut Frame, app: &App) {
    let (bg_color, fg_color, border_color) = match app.theme {
        Theme::Light => (Color::White, Color::Black, Color::Black),
        Theme::Dark => (Color::Rgb(20, 20, 25), Color::White, Color::DarkGray),
    };

    let base_style = Style::default().bg(bg_color).fg(fg_color);
    let size = f.area();
    f.render_widget(Block::default().style(base_style), size);

    // Vertical Split: Navbar (Top) vs Main Content (Bottom)
    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Navbar height
            Constraint::Min(0),    // Main content
        ])
        .split(size);

    render_navbar(f, app, vertical_layout[0], border_color, fg_color);

    let content_area = vertical_layout[1];

    match app.current_screen {
        CurrentScreen::Home => render_home(f, app, content_area, border_color, fg_color),
        CurrentScreen::TokenDetails => {
            render_token_details(f, app, content_area, border_color, fg_color)
        }
    }

    if app.show_search_modal {
        render_search_modal(f, app, size, border_color, fg_color);
    }
}

fn render_home(f: &mut Frame, app: &App, area: Rect, border: Color, text: Color) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(area);

    // Render columns
    render_token_column(
        f,
        app,
        "New Items",
        &app.new_tokens,
        0,
        chunks[0],
        border,
        text,
    );
    render_token_column(
        f,
        app,
        "Almost Bonded",
        &app.bonding_tokens,
        1,
        chunks[1],
        border,
        text,
    );
    render_token_column(
        f,
        app,
        "Migrated",
        &app.migrated_tokens,
        2,
        chunks[2],
        border,
        text,
    );
}

fn render_token_column(
    f: &mut Frame,
    app: &App,
    title: &str,
    tokens: &[crate::app::Token],
    col_idx: usize,
    area: Rect,
    border: Color,
    text: Color,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .title(title);

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    // Fixed height per card
    let card_height = 5;
    let max_visible = (inner_area.height / card_height) as usize;

    let start_index = if app.home_selected_col == col_idx {
        if app.home_selected_row >= max_visible {
            app.home_selected_row.saturating_sub(max_visible) + 1
        } else {
            0
        }
    } else {
        0
    };

    for (i, token) in tokens
        .iter()
        .skip(start_index)
        .take(max_visible)
        .enumerate()
    {
        let abs_index = start_index + i;
        let is_selected = app.home_selected_col == col_idx && app.home_selected_row == abs_index;

        let card_area = Rect {
            x: inner_area.x,
            y: inner_area.y + (i as u16 * card_height),
            width: inner_area.width,
            height: card_height,
        };

        render_token_card(f, token, is_selected, card_area, border, text);
    }
}

fn render_token_card(
    f: &mut Frame,
    token: &crate::app::Token,
    is_selected: bool,
    area: Rect,
    border: Color,
    text: Color,
) {
    let border_style = if is_selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(border)
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(if is_selected {
            BorderType::Double
        } else {
            BorderType::Rounded
        })
        .border_style(border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);

    // Layout: Image (Left) | Info (Right)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(8), // Image box width
            Constraint::Min(0),    // Content
        ])
        .split(inner);

    // Image Placeholder
    let img_bg = match token.symbol.chars().next().unwrap_or('A') {
        'A'..='F' => Color::Red,
        'G'..='L' => Color::Blue,
        'M'..='R' => Color::Green,
        _ => Color::Magenta,
    };

    let image_placeholder = Paragraph::new(token.image_asc.clone())
        .block(Block::default().borders(Borders::NONE))
        .style(
            Style::default()
                .bg(img_bg)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(image_placeholder, chunks[0]);

    // Info Area
    let info_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(chunks[1]);

    let price_color = if token.change_24h >= 0.0 {
        Color::Green
    } else {
        Color::Red
    };

    // Row 1
    let row1 = TextLine::from(vec![
        Span::styled(
            format!(" {} ", token.symbol),
            Style::default().fg(text).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("{} ", token.name), Style::default().fg(Color::Gray)),
        Span::raw(" "),
        Span::styled(
            format!("${:.1}K", token.market_cap / 1000.0),
            Style::default().fg(Color::Cyan),
        ),
    ]);
    f.render_widget(Paragraph::new(row1), info_chunks[0]);

    // Row 2
    let row2 = TextLine::from(vec![
        Span::raw(format!(" ${:.6} ", token.price)),
        Span::styled(
            format!("{:.1}% ", token.change_24h),
            Style::default().fg(price_color),
        ),
    ]);
    f.render_widget(Paragraph::new(row2), info_chunks[1]);

    // Row 3
    let row3 = TextLine::from(vec![
        Span::raw(format!(" Vol: ${:.1}K ", token.volume / 1000.0)),
        Span::raw(format!("Tx: {} ", token.txns)),
        Span::raw(format!("Bond: {:.0}%", token.bonding)),
    ]);
    f.render_widget(Paragraph::new(row3), info_chunks[2]);
}

fn render_token_details(
    f: &mut Frame,
    app: &App,
    area: Rect,
    border_color: Color,
    fg_color: Color,
) {
    // Main Content Layout (Horizontal Split)
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(app.col_constraints[0]),
            Constraint::Percentage(app.col_constraints[1]),
            Constraint::Percentage(app.col_constraints[2]),
        ])
        .split(area);

    render_left_sidebar(f, app, main_layout[0], border_color, fg_color);

    // Center
    let center_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(app.row_constraints[0]),
            Constraint::Percentage(app.row_constraints[1]),
        ])
        .split(main_layout[1]);

    render_chart_area(f, app, center_layout[0], border_color, fg_color);
    render_bottom_panel(f, app, center_layout[1], border_color, fg_color);

    render_right_sidebar(f, app, main_layout[2], border_color, fg_color);
}

fn render_navbar(f: &mut Frame, app: &App, area: Rect, border: Color, text: Color) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // Title
            Constraint::Percentage(60), // Search Bar
            Constraint::Percentage(20), // Connect Wallet
        ])
        .split(area);

    // 1. Title
    let title = Paragraph::new(Span::styled(
        "Trading Terminal",
        Style::default()
            .fg(text)
            .add_modifier(Modifier::BOLD | Modifier::ITALIC),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border)),
    );
    f.render_widget(title, chunks[0]);

    // 2. Search Bar
    let search_text = if app.search_input.is_empty() {
        "Search tokens...".to_string()
    } else {
        app.search_input.clone()
    };
    let search = Paragraph::new(search_text)
        .style(if app.search_input.is_empty() {
            Style::default().fg(Color::Gray)
        } else {
            Style::default().fg(text)
        })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Search")
                .border_style(Style::default().fg(border)),
        );
    f.render_widget(search, chunks[1]);

    // 3. Connect Wallet
    let wallet_text = if let Some(pubkey) = app.wallet_pubkey {
        let pk_str = pubkey.to_string();
        format!("{}...{}", &pk_str[0..4], &pk_str[pk_str.len() - 4..])
    } else {
        "Connect Wallet".to_string()
    };

    let wallet_style = if app.wallet_pubkey.is_some() {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Yellow)
    };

    let wallet_btn = Paragraph::new(Span::styled(wallet_text, wallet_style)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border)),
    );
    f.render_widget(wallet_btn, chunks[2]);
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
    let bonding_curve_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(border))
                .title("Bonding Curve"),
        )
        .gauge_style(Style::default().fg(Color::Yellow))
        .percent(app.token_info.bonding_curve as u16)
        .label(Span::styled(
            format!("{}% (Graduates at $61k)", app.token_info.bonding_curve),
            Style::default().fg(text).add_modifier(Modifier::BOLD),
        ));
    f.render_widget(bonding_curve_gauge, chunks[1]);

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
        .x_bounds([app.chart_x_offset, app.chart_x_offset + 50.0])
        .y_bounds([0.0035 + app.chart_y_offset, 0.0045 + app.chart_y_offset])
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

fn render_bottom_panel(f: &mut Frame, app: &App, area: Rect, border: Color, text: Color) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Tabs
            Constraint::Min(0),    // Content
        ])
        .split(area);

    // 1. Tabs
    let titles: Vec<TextLine> = vec![
        "Transactions",
        "Positions",
        "Orders",
        "Holders",
        "History",
        "Dev Tokens",
    ]
    .iter()
    .map(|t| {
        let (first, rest) = t.split_at(1);
        TextLine::from(vec![
            Span::styled(first, Style::default().fg(Color::Yellow)),
            Span::styled(rest, Style::default().fg(text)),
        ])
    })
    .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::BOTTOM | Borders::TOP)
                .border_style(Style::default().fg(border)),
        )
        .select(app.bottom_tab_index)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .bg(Color::DarkGray),
        );
    f.render_widget(tabs, chunks[0]);

    // 2. Content
    match app.bottom_tab_index {
        0 => render_transactions(f, app, chunks[1], border, text),
        3 => render_holders_list(f, app, chunks[1], border, text),
        _ => {
            let p = Paragraph::new("Coming soon...").block(Block::default().borders(Borders::NONE));
            f.render_widget(p, chunks[1]);
        }
    }
}

fn render_transactions(f: &mut Frame, app: &App, area: Rect, _border: Color, _text: Color) {
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
            .style(Style::default().fg(Color::Yellow))
            .bottom_margin(1),
    )
    .block(Block::default().borders(Borders::NONE));

    f.render_widget(table, area);
}

fn render_holders_list(f: &mut Frame, app: &App, area: Rect, _border: Color, text: Color) {
    let rows: Vec<Row> = app
        .holders
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let color = if h.is_dev { Color::Green } else { text };
            Row::new(vec![
                format!("{}", i + 1),
                h.address.clone(),
                format!("{:.2}%", h.balance),
                format!("${:.2}", h.value),
            ])
            .style(Style::default().fg(color))
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(10),
            Constraint::Percentage(40),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ],
    )
    .header(
        Row::new(vec!["Rank", "Address", "% Held", "Value"])
            .style(Style::default().fg(Color::Yellow))
            .bottom_margin(1),
    )
    .block(Block::default().borders(Borders::NONE));

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

fn render_search_modal(f: &mut Frame, app: &App, area: Rect, border: Color, text: Color) {
    // Vertically center (Larger area for list)
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60), // Height 60%
            Constraint::Percentage(20),
        ])
        .split(area);

    // Horizontally center
    let center_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40), // Width 40%
            Constraint::Percentage(30),
        ])
        .split(popup_layout[1]);

    let chunk = center_layout[1];

    // Clear background
    f.render_widget(Clear, chunk);

    // Main Block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(border))
        .title("Select Token");
    f.render_widget(block.clone(), chunk);

    // Inner layout for Input and List
    let inner_area = block.inner(chunk);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input box
            Constraint::Min(0),    // List
        ])
        .split(inner_area);

    // Input
    let search_input = Paragraph::new(app.search_input.clone())
        .style(Style::default().fg(text))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Yellow))
                .title("Search"),
        );
    f.render_widget(search_input, chunks[0]);

    // Token List
    let items: Vec<ListItem> = app
        .filtered_tokens
        .iter()
        .enumerate()
        .map(|(i, token)| {
            let style = if i == app.search_select_index {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(text)
            };

            let content = format!(
                "{:<10} {:<20} ${:.4}",
                token.symbol, token.name, token.price
            );
            ListItem::new(content).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::TOP)) // Separator
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));

    // We handle selection manualy via style above, but List widget also supports state.
    // For simplicity of rendering "selected" background on the item itself, the manual map above works well.
    f.render_widget(list, chunks[1]);
}
