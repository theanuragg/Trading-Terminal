use crate::app::{App, CurrentScreen, Theme};
use chrono::{TimeZone, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line as TextLine, Span},
    widgets::{
        Block, BorderType, Borders, Clear, Gauge, List, ListItem, Paragraph, Row, Table, Tabs,
    },
    Frame,
};

pub fn ui(f: &mut Frame, app: &mut App) {
    app.image_draw_requests.clear();
    // In Bloomberg mode we use yellow solely for text/accents;
    // borders should be a neutral dark gray to reduce visual noise.
    let (bg_color, fg_color, border_color) = match app.theme {
        Theme::Light => (Color::White, Color::Black, Color::Black),
        Theme::Dark => (Color::Rgb(20, 20, 25), Color::White, Color::DarkGray),
        Theme::Bloomberg => (crate::theme::BG, crate::theme::TEXT, crate::theme::HEADER),
    };

    let base_style = crate::theme::default_style();
    let size = f.area();
    f.render_widget(Block::default().style(base_style), size);

    // Vertical split: navbar / content area
    let vertical_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Navbar
            Constraint::Min(0),    // Content area for panels
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

    if app.show_connect_wallet_modal {
        render_wallet_connect_modal(f, app, size, border_color, fg_color);
    }
}

// ---------------------------------------------------------------------------

fn render_home(f: &mut Frame, app: &mut App, area: Rect, border: Color, text: Color) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(33),
        ])
        .split(area);

    // Render columns
    render_token_column(f, app, "New Items", 0, chunks[0], border, text);
    render_token_column(f, app, "Almost Bonded", 1, chunks[1], border, text);
    render_token_column(f, app, "Migrated", 2, chunks[2], border, text);
}

fn render_token_column(
    f: &mut Frame,
    app: &mut App,
    title: &str,
    col_idx: usize,
    area: Rect,
    border: Color,
    text: Color,
) {
    let tokens = match col_idx {
        0 => &app.new_tokens,
        1 => &app.bonding_tokens,
        2 => &app.migrated_tokens,
        _ => &app.all_tokens,
    };

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

    let tokens_to_render: Vec<_> = tokens
        .iter()
        .skip(start_index)
        .take(max_visible)
        .cloned()
        .collect();

    for (i, token) in tokens_to_render.iter().enumerate() {
        let abs_index = start_index + i;

        let is_selected = app.home_selected_col == col_idx && app.home_selected_row == abs_index;

        let card_area = Rect {
            x: inner_area.x,
            y: inner_area.y + (i as u16 * card_height),
            width: inner_area.width,
            height: card_height,
        };

        render_token_card(f, app, token, is_selected, card_area, border, text);
    }
}

fn render_token_card(
    f: &mut Frame,
    app: &mut App,
    token: &crate::app::Token,
    is_selected: bool,
    area: Rect,
    border: Color,
    text: Color,
) {
    let accent_color = match app.theme {
        Theme::Bloomberg => crate::theme::HIGHLIGHT,
        _ => Color::Cyan,
    };
    let border_style = if is_selected {
        crate::theme::highlight_border_style().add_modifier(Modifier::BOLD)
    } else {
        crate::theme::border_style()
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

    // Always use text placeholders on Home screen cards.
    // viuer's block-character fallback (▄█▀) writes directly to stdout,
    // bypassing ratatui's buffer, causing images to persist across scrolls.
    // The 8x3 cell area is also too small for meaningful pixel images.
    {
        let img_color = match token.symbol.chars().next().unwrap_or('A') {
            'A'..='E' => Color::Red,
            'F'..='J' => Color::Blue,
            'K'..='O' => Color::Green,
            'P'..='T' => Color::Yellow,
            _ => Color::Magenta,
        };

        let border_color_card = if is_selected {
            Color::Yellow
        } else {
            img_color
        };

        let icon = match token.symbol.to_uppercase().as_str() {
            "SOL" => "◎",
            "USDC" => "$",
            "BTC" => "₿",
            "ETH" => "Ξ",
            _ => {
                if !token.image_asc.is_empty() {
                    &token.image_asc
                } else {
                    "📦"
                }
            }
        };

        let char_box = Paragraph::new(vec![
            TextLine::from(""),
            TextLine::from(vec![Span::styled(
                format!(" {} ", icon),
                Style::default().fg(img_color).add_modifier(Modifier::BOLD),
            )]),
            TextLine::from(""),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color_card)),
        )
        .alignment(ratatui::layout::Alignment::Center);

        f.render_widget(char_box, chunks[0]);
    }

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
        crate::theme::POSITIVE
    } else {
        crate::theme::NEGATIVE
    };

    // Row 1
    let row1 = TextLine::from(vec![
        Span::styled(
            format!(" {} ", token.symbol.to_uppercase()),
            Style::default().fg(text).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{} ", token.name),
            Style::default().fg(crate::theme::HEADER),
        ),
        Span::raw(" "),
        Span::styled(
            format!("${:.1}K", token.market_cap / 1000.0),
            Style::default().fg(accent_color),
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
    app: &mut App,
    area: Rect,
    border_color: Color,
    fg_color: Color,
) {
    // choose accent color for this screen as well
    let accent_color = match app.theme {
        Theme::Bloomberg => crate::theme::HIGHLIGHT,
        _ => Color::Cyan,
    };
    // Main Content Layout (Horizontal Split)
    let main_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // Left sidebar (reduced from app.col_constraints[0])
            Constraint::Percentage(60), // Center area (expanded)
            Constraint::Percentage(20), // Right sidebar (reduced from app.col_constraints[2])
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

fn render_navbar(f: &mut Frame, app: &mut App, area: Rect, border: Color, text: Color) {
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
        "TRADING TERMINAL",
        Style::default()
            .fg(text)
            .add_modifier(Modifier::BOLD | Modifier::ITALIC | Modifier::UNDERLINED),
    ))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(crate::theme::border_style()),
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
                .border_style(crate::theme::border_style()),
        );
    f.render_widget(search, chunks[1]);

    // 3. Connect Wallet
    let wallet_style = if app.wallet_pubkey.is_some() {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Yellow)
    };

    let wallet_text_line = if let Some(pubkey) = app.wallet_pubkey {
        vec![
            TextLine::from(Span::styled(
                format!(
                    "{}...{}",
                    &pubkey.to_string()[0..4],
                    &pubkey.to_string()[pubkey.to_string().len() - 4..]
                ),
                wallet_style,
            )),
            TextLine::from(Span::styled(
                "(Connected)",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::ITALIC),
            )),
        ]
    } else {
        vec![
            TextLine::from(Span::styled("Connect Wallet", wallet_style)),
            TextLine::from(Span::styled(
                "Press 'c'",
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::ITALIC),
            )),
        ]
    };

    let wallet_btn = Paragraph::new(wallet_text_line).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(crate::theme::border_style()),
    );
    f.render_widget(wallet_btn, chunks[2]);
}

fn render_left_sidebar(f: &mut Frame, app: &mut App, area: Rect, border: Color, text: Color) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // Token Info (Name, Symbol, Creator)
            Constraint::Length(4),  // Links (Website, Twitter, Telegram)
            Constraint::Length(3),  // Bonding Curve
            Constraint::Length(12), // Stats Grid (Timeframes, Vol, Net Vol, etc)
            Constraint::Min(0),     // Deltas
        ])
        .split(area);

    // --- Token Info Section ---
    // Always use text icon — viuer's block-character fallback (▄█▀) overlaps
    // the token name/mint text and looks pixelated. Text icons are cleaner.
    let (icon_symbol, icon_style) = match app.token_info.symbol.to_uppercase().as_str() {
        "SOL" => ("◎ ", Style::default().fg(Color::Cyan)),
        "USDC" => ("$ ", Style::default().fg(Color::Green)),
        "BTC" => ("₿ ", Style::default().fg(Color::Yellow)),
        "ETH" => ("Ξ ", Style::default().fg(Color::Blue)),
        _ => {
            if !app.token_info.symbol.is_empty() {
                (
                    &app.token_info.symbol[0..1],
                    Style::default().fg(Color::Magenta),
                )
            } else {
                ("?", Style::default())
            }
        }
    };

    let token_info_text = vec![
        TextLine::from(vec![
            Span::styled(format!(" {} ", icon_symbol), icon_style),
            Span::raw("  "),
            Span::styled(
                format!(" {} ", app.token_info.symbol),
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                &app.token_info.name,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        TextLine::from(""),
        // MINT ADDRESS + COPY
        TextLine::from(vec![
            Span::styled("Mint:    ", Style::default().fg(crate::theme::HEADER)),
            Span::styled(
                format!(
                    "{}...{}",
                    &app.token_info.mint[..8.min(app.token_info.mint.len())],
                    if app.token_info.mint.len() > 8 {
                        &app.token_info.mint[app.token_info.mint.len() - 4..]
                    } else {
                        ""
                    },
                ),
                Style::default().fg(Color::LightGreen),
            ),
            Span::raw(" "),
            Span::styled(
                " [COPY] ",
                Style::default()
                    .bg(Color::Rgb(40, 40, 40))
                    .fg(Color::White)
                    .add_modifier(Modifier::DIM),
            ),
        ]),
        // CREATOR ADDRESS + COPY/LINK
        TextLine::from(vec![
            Span::styled("Creator: ", Style::default().fg(crate::theme::HEADER)),
            Span::styled(
                format!(
                    "{}...{}",
                    &app.token_info.creator_address[..8.min(app.token_info.creator_address.len())],
                    if app.token_info.creator_address.len() > 8 {
                        &app.token_info.creator_address[app.token_info.creator_address.len() - 4..]
                    } else {
                        ""
                    }
                ),
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::UNDERLINED),
            ),
            Span::raw(" "),
            Span::styled(
                " [COPY/OPEN] ",
                Style::default()
                    .bg(Color::Rgb(40, 40, 40))
                    .fg(Color::White)
                    .add_modifier(Modifier::DIM),
            ),
        ]),
        TextLine::from(""),
        TextLine::from(if app.token_info.description.is_empty() {
            "No description available".to_string()
        } else {
            app.token_info.description.clone()
        }),
    ];

    let info_block = Paragraph::new(token_info_text)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(crate::theme::border_style())
                .title(format!(" Mint Details (Press 'y' to copy) ")),
        )
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(info_block, chunks[0]);

    // --- Links Section ---
    let links_text = vec![
        TextLine::from(vec![
            Span::styled("🔗 ", Style::default().fg(Color::Blue)),
            Span::styled(
                "Links",
                Style::default().fg(text).add_modifier(Modifier::BOLD),
            ),
        ]),
        if !app.token_info.website.is_empty() {
            TextLine::from(vec![
                Span::raw("🌐 "),
                Span::styled("Website", Style::default().fg(Color::Blue)),
            ])
        } else {
            TextLine::from("")
        },
        if !app.token_info.twitter.is_empty() {
            TextLine::from(vec![
                Span::raw("𝕏 "),
                Span::styled("Twitter", Style::default().fg(Color::Blue)),
            ])
        } else {
            TextLine::from("")
        },
        if !app.token_info.telegram.is_empty() {
            TextLine::from(vec![
                Span::raw("💬 "),
                Span::styled("Telegram", Style::default().fg(Color::Blue)),
            ])
        } else {
            TextLine::from("")
        },
    ];
    f.render_widget(
        Paragraph::new(links_text).block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(crate::theme::border_style()),
        ),
        chunks[1],
    );

    // --- Bonding Curve ---
    let bonding_curve_gauge = Gauge::default()
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(crate::theme::border_style())
                .title("Bonding Curve"),
        )
        .gauge_style(crate::theme::positive_style())
        .percent(app.token_info.bonding_curve as u16)
        .label(Span::styled(
            format!("{}%", app.token_info.bonding_curve as u16),
            Style::default().fg(text).add_modifier(Modifier::BOLD),
        ));
    f.render_widget(bonding_curve_gauge, chunks[2]);

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
    let sell_pressure = format!("{}%", app.token_info.sell_pressure);
    let traders_24h = format!("{}", app.token_info.traders_24h);
    let net_buyers = if let Some(v) = app.token_info.net_buyers {
        format!("{}", v)
    } else {
        "-".to_string()
    };
    let net_buy_trend = format!("${:.4}", app.token_info.net_buy_trend_24h);

    let mcap_str = format!("${:.2}M", app.token_info.market_cap / 1_000_000.0);
    let liq_string = format!("${:.1}K", app.token_info.liquidity / 1000.0);
    let holders_string = format!("{}", app.token_info.holders);

    let stats_rows = vec![
        Row::new(vec!["5m", change_5m.as_str(), "1h", change_1h.as_str()]),
        Row::new(vec!["6h", change_6h.as_str(), "24h", change_24h.as_str()]).style(
            Style::default().fg(if app.token_info.change_24h >= 0.0 {
                crate::theme::POSITIVE
            } else {
                crate::theme::NEGATIVE
            }),
        ),
        Row::new(vec!["24h Vol", vol_24h.as_str(), "MCap", mcap_str.as_str()]),
        Row::new(vec![
            "Liq",
            liq_string.as_str(),
            "Holders",
            holders_string.as_str(),
        ]),
        Row::new(vec![
            "Sell Pr.",
            sell_pressure.as_str(),
            "Traders",
            traders_24h.as_str(),
        ]),
        Row::new(vec![
            "Net B.",
            net_buyers.as_str(),
            "Trend",
            net_buy_trend.as_str(),
        ]),
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
    f.render_widget(stats_table, chunks[3]);

    // Wrap entire sidebar in a block
    f.render_widget(
        Block::default()
            .borders(Borders::RIGHT)
            .border_style(crate::theme::border_style()),
        area,
    );
}

fn render_chart_area(f: &mut Frame, app: &mut App, area: Rect, border: Color, _text: Color) {
    // If no candles, show placeholder
    if app.candles.is_empty() {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(crate::theme::border_style())
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
            Style::default().fg(Color::White),
        )));
    }
    f.render_widget(
        Paragraph::new(price_lines)
            .alignment(ratatui::layout::Alignment::Right)
            .style(Style::default().fg(Color::White)),
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
            .style(Style::default().fg(Color::White)),
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
        let candle_color = if is_bullish { Color::Green } else { Color::Red };

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
                    .border_style(Style::default().fg(border))
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

fn render_bottom_panel(f: &mut Frame, app: &mut App, area: Rect, border: Color, text: Color) {
    // accent color based on theme
    let accent_color = match app.theme {
        Theme::Bloomberg => Color::Green,
        _ => Color::Cyan,
    };
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
        0 => render_transactions(f, app, chunks[1], border, text, accent_color),
        3 => render_holders_list(f, app, chunks[1], border, text, accent_color),
        _ => {
            let p = Paragraph::new("Coming soon...").block(Block::default().borders(Borders::NONE));
            f.render_widget(p, chunks[1]);
        }
    }
}

fn render_transactions(
    f: &mut Frame,
    app: &App,
    area: Rect,
    border: Color,
    _text: Color,
    accent: Color,
) {
    let rows: Vec<Row> = app
        .recent_trades
        .iter()
        .take(25) // Show top 25 transactions
        .map(|t| {
            let color = if t.type_.to_lowercase() == "buy" {
                Color::Green
            } else {
                Color::Red
            };
            // Combine date and age into one column"DATE (AGE)"
            let date_age = format!("{} ({})", t.date, t.age);
            Row::new(vec![
                date_age,                    // DATE (AGE) combined
                t.type_.clone(),             // TYPE
                format!("{:.7}", t.price),   // PRICE
                format!("${:.2}", t.volume), // VOLUME with $ sign
                format!("{:.4}", t.sol),     // SOL
            ])
            .style(Style::default().fg(color))
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(35), // DATE (AGE): 35%
            Constraint::Percentage(15), // TYPE: 15%
            Constraint::Percentage(20), // PRICE: 20%
            Constraint::Percentage(20), // VOLUME: 20%
            Constraint::Percentage(10), // SOL: 10%
        ],
    )
    .header(
        Row::new(vec![
            "   DATE / AGE",
            "TYPE",
            "    PRICE",
            "     VOLUME",
            "    SOL",
        ])
        .style(
            Style::default()
                .fg(accent)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        )
        .bottom_margin(1),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border))
            .title("  Transactions"),
    )
    .row_highlight_style(Style::default());

    f.render_widget(table, area);
}

fn render_holders_list(
    f: &mut Frame,
    app: &App,
    area: Rect,
    border: Color,
    text: Color,
    accent: Color,
) {
    let holders_to_show = if !app.jupiter_holders.is_empty() {
        &app.jupiter_holders
    } else {
        &app.holders
    };

    let rows: Vec<Row> = holders_to_show
        .iter()
        .take(20) // Show top 20 holders
        .enumerate()
        .map(|(_i, h)| {
            let color = if h.is_dev { Color::Green } else { text };
            let short_addr = if h.address.len() > 12 {
                h.address[..10].to_string() + ".."
            } else {
                h.address.clone()
            };
            Row::new(vec![
                short_addr,                      // ADDRESS
                format!("{:.2}%", h.percent),    // % OWNED
                format!("{:.4}", h.sol_balance), // SOL BAL
                format!("{:.0}", h.amount),      // AMOUNT
            ])
            .style(Style::default().fg(color))
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(30), // ADDRESS: 30%
            Constraint::Percentage(20), // % OWNED: 20%
            Constraint::Percentage(25), // SOL BAL: 25%
            Constraint::Percentage(25), // AMOUNT: 25%
        ],
    )
    .header(
        Row::new(vec!["  ADDRESS", "  % OWNED", "   SOL BAL", "     AMOUNT"])
            .style(
                Style::default()
                    .fg(accent)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )
            .bottom_margin(1),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border))
            .title("  Top Holders"),
    )
    .highlight_style(Style::default().add_modifier(Modifier::BOLD))
    .row_highlight_style(Style::default());

    f.render_widget(table, area);
}

fn render_right_sidebar(f: &mut Frame, app: &mut App, area: Rect, border: Color, _text: Color) {
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

fn render_search_modal(f: &mut Frame, app: &mut App, area: Rect, border: Color, text: Color) {
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
                .border_style(Style::default().fg(border))
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

fn render_wallet_connect_modal(f: &mut Frame, app: &App, area: Rect, border: Color, text: Color) {
    // Create centered popup
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(area);

    let center_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(popup_layout[1]);

    let chunk = center_layout[1];

    // Clear background
    f.render_widget(Clear, chunk);

    // Main Block
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border).add_modifier(Modifier::BOLD))
        .title("Connect Wallet");

    f.render_widget(block.clone(), chunk);

    let inner_area = block.inner(chunk);
    let inner_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(inner_area);

    // Message and Details
    let mut lines = vec![
        TextLine::from(Span::styled(
            "🔗 Wallet Status",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        TextLine::from(""),
    ];

    match &app.wallet_modal_state {
        crate::app::WalletModalState::Idle => {
            lines.push(TextLine::from(Span::styled(
                "Loading wallet details...",
                Style::default().fg(Color::Gray),
            )));
        }
        crate::app::WalletModalState::NewWalletCreated { pubkey, path } => {
            lines.push(TextLine::from(vec![
                Span::styled(
                    "SUCCESS: ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("New Wallet Created!"),
            ]));
            lines.push(TextLine::from(""));
            lines.push(TextLine::from(vec![
                Span::styled("PublicKey: ", Style::default().fg(Color::Yellow)),
                Span::raw(pubkey),
            ]));
            lines.push(TextLine::from(vec![
                Span::styled("Saved To:  ", Style::default().fg(Color::Yellow)),
                Span::styled(path, Style::default().fg(Color::DarkGray)),
            ]));

            if let Some(pk) = &app.wallet_private_key_display {
                lines.push(TextLine::from(""));
                lines.push(TextLine::from(Span::styled(
                    "⚠️  SAVE YOUR PRIVATE KEY (Import to Phantom/Solflare):",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                )));
                lines.push(TextLine::from(Span::styled(
                    pk,
                    Style::default().fg(Color::LightRed),
                )));
            }
        }
        crate::app::WalletModalState::ExistingWalletLoaded { pubkey, path } => {
            lines.push(TextLine::from(vec![
                Span::styled(
                    "SUCCESS: ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw("Existing Wallet Loaded"),
            ]));
            lines.push(TextLine::from(""));
            lines.push(TextLine::from(vec![
                Span::styled("PublicKey: ", Style::default().fg(Color::Yellow)),
                Span::raw(pubkey),
            ]));
            lines.push(TextLine::from(vec![
                Span::styled("Path:      ", Style::default().fg(Color::Yellow)),
                Span::styled(path, Style::default().fg(Color::DarkGray)),
            ]));
        }
        crate::app::WalletModalState::Error(err) => {
            lines.push(TextLine::from(vec![
                Span::styled(
                    "ERROR: ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::raw(err),
            ]));
        }
    }

    let message = Paragraph::new(lines)
        .style(Style::default().fg(text))
        .wrap(ratatui::widgets::Wrap { trim: true });

    f.render_widget(message, inner_layout[1]);

    // Instructions
    let instructions = Paragraph::new(vec![TextLine::from(Span::styled(
        "Press 'q' or 'Esc' to close",
        Style::default()
            .fg(Color::Gray)
            .add_modifier(Modifier::ITALIC),
    ))])
    .alignment(ratatui::layout::Alignment::Center)
    .style(Style::default().fg(Color::Gray));

    f.render_widget(instructions, inner_layout[2]);
}
