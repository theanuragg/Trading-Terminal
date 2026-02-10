use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    Terminal,
};
use std::{io, sync::Arc, time::Duration};

use tokio::sync::mpsc;

use tx_terminal::app::{App, CurrentScreen, DragState};
use tx_terminal::ui::ui;

use base64::{engine::general_purpose, Engine as _};
use solana_sdk::{
    signer::{keypair::read_keypair_file, Signer},
    transaction::VersionedTransaction,
};
use tx_terminal::network::{IndexerClient, NetworkClient};
use tx_terminal::swap::JupiterClient;

enum AppEvent {
    Log(String),
    TokensFetched(Vec<String>),
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = tx_terminal::config::load_config();
    let network_client = NetworkClient::new(&config.rpc_url);
    let indexer_client = IndexerClient::new();

    // Channel for async events
    let (tx, mut rx) = mpsc::channel(100);

    // Load wallet if provided
    let (wallet_pubkey, balance, wallet_keypair) = if let Some(path) = &config.keypair_path {
        if let Ok(kp) = read_keypair_file(path) {
            let pubkey = kp.pubkey();
            let balance = network_client.get_balance(&pubkey).await.unwrap_or(0);
            (Some(pubkey), balance, Some(Arc::new(kp)))
        } else {
            (None, 0, None)
        }
    } else {
        (None, 0, None)
    };

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new(wallet_pubkey, balance);

    if let Some(pk) = wallet_pubkey {
        app.add_log(format!("Wallet loaded: {}", pk));
    } else {
        app.add_log("No wallet loaded. Use --keypair-path to connect.".to_string());
    }

    // Fetch initial token list
    let tx_tokens = tx.clone();
    tokio::spawn(async move {
        if let Ok(tokens) = indexer_client.fetch_tokens().await {
            let _ = tx_tokens.send(AppEvent::TokensFetched(tokens)).await;
        } else {
            let _ = tx_tokens
                .send(AppEvent::Log("Failed to load tokens.".to_string()))
                .await;
        }
    });

    // Run app
    let res = run_app(
        &mut terminal,
        &mut app,
        tx,
        &mut rx,
        network_client,
        wallet_keypair,
    )
    .await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

async fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    tx: mpsc::Sender<AppEvent>,
    rx: &mut mpsc::Receiver<AppEvent>,
    network_client: NetworkClient,
    wallet_keypair: Option<Arc<solana_sdk::signer::keypair::Keypair>>,
) -> Result<()> {
    loop {
        app.simulate_market_activity();
        terminal.draw(|f| ui(f, app))?;

        // Check for async events
        if let Ok(event) = rx.try_recv() {
            match event {
                AppEvent::Log(msg) => app.add_log(msg),
                AppEvent::TokensFetched(tokens) => {
                    app.token_list = tokens;
                    app.add_log(format!(
                        "Loaded {} tokens via async task.",
                        app.token_list.len()
                    ));
                }
            }
        }

        if crossterm::event::poll(Duration::from_millis(10))? {
            match crossterm::event::read()? {
                Event::Key(key) => {
                    // Global Keys
                    if key.code == KeyCode::Char('q') {
                        app.quit();
                        return Ok(());
                    }

                    match app.current_screen {
                        CurrentScreen::Home => {
                            match key.code {
                                KeyCode::Right => {
                                    if app.home_selected_col < 2 {
                                        app.home_selected_col += 1;
                                        app.home_selected_row = 0; // Reset row when switching col
                                    }
                                }
                                KeyCode::Left => {
                                    if app.home_selected_col > 0 {
                                        app.home_selected_col -= 1;
                                        app.home_selected_row = 0;
                                    }
                                }
                                KeyCode::Down => {
                                    // simplistic check, ideally check vec len
                                    app.home_selected_row += 1;
                                }
                                KeyCode::Up => {
                                    if app.home_selected_row > 0 {
                                        app.home_selected_row -= 1;
                                    }
                                }
                                KeyCode::Enter => {
                                    // Select token logic
                                    let token = match app.home_selected_col {
                                        0 => app.new_tokens.get(app.home_selected_row),
                                        1 => app.bonding_tokens.get(app.home_selected_row),
                                        2 => app.migrated_tokens.get(app.home_selected_row),
                                        _ => None,
                                    };

                                    if let Some(t) = token {
                                        app.token_info.name = t.name.clone();
                                        app.token_info.symbol = t.symbol.clone();
                                        app.token_info.price = t.price;
                                        app.current_screen = CurrentScreen::TokenDetails;
                                    }
                                }
                                _ => {}
                            }
                        }
                        CurrentScreen::TokenDetails => {
                            if app.show_search_modal {
                                match key.code {
                                    KeyCode::Esc => {
                                        app.show_search_modal = false;
                                    }
                                    KeyCode::Enter => {
                                        app.select_current_token();
                                        app.show_search_modal = false;
                                        app.search_input.clear();
                                        app.update_search_results(); // Reset results
                                    }
                                    KeyCode::Up => {
                                        if app.search_select_index > 0 {
                                            app.search_select_index -= 1;
                                        }
                                    }
                                    KeyCode::Down => {
                                        if app.search_select_index
                                            < app.filtered_tokens.len().saturating_sub(1)
                                        {
                                            app.search_select_index += 1;
                                        }
                                    }
                                    KeyCode::Backspace => {
                                        app.search_input.pop();
                                        app.update_search_results();
                                    }
                                    KeyCode::Char(c) => {
                                        app.search_input.push(c);
                                        app.update_search_results();
                                    }
                                    _ => {}
                                }
                            } else {
                                match key.code {
                                    KeyCode::Esc => {
                                        app.current_screen = CurrentScreen::Home;
                                    }
                                    KeyCode::Backspace => {
                                        app.swap_amount.pop();
                                    }
                                    KeyCode::Char(c) if c.is_digit(10) || c == '.' => {
                                        app.swap_amount.push(c);
                                    }
                                    KeyCode::Char('s') => {
                                        if let Some(kp) = &wallet_keypair {
                                            app.add_log(format!(
                                                "Initiating swap: {} SOL -> {}",
                                                app.swap_amount, app.token_info.symbol
                                            ));

                                            // Capture data before spawn
                                            let input_mint =
                                                "So11111111111111111111111111111111111111112"
                                                    .to_string();
                                            let output_mint = app.token_info.mint.clone();
                                            let amount_sol =
                                                app.swap_amount.parse::<f64>().unwrap_or(0.0);
                                            let amount = (amount_sol * 1_000_000_000.0) as u64;

                                            let tx_swap = tx.clone();
                                            let nc = network_client.clone();
                                            let kp_arc = kp.clone();

                                            tokio::spawn(async move {
                                                let jupiter = JupiterClient::new();
                                                // SOL -> Selected Token
                                                // input_mint, output_mint, amount already captured

                                                let quote_res = jupiter
                                                    .get_quote(
                                                        &input_mint,
                                                        &output_mint,
                                                        amount,
                                                        50,
                                                    )
                                                    .await;

                                                match quote_res {
                                                    Ok(quote) => {
                                                        let _ = tx_swap
                                                            .send(AppEvent::Log(format!(
                                                                "Quote: Out {}",
                                                                quote.out_amount
                                                            )))
                                                            .await;

                                                        // Get Swap Transaction
                                                        let user_pubkey =
                                                            kp_arc.pubkey().to_string();
                                                        match jupiter
                                                            .get_swap_transaction(
                                                                &user_pubkey,
                                                                quote,
                                                            )
                                                            .await
                                                        {
                                                            Ok(swap_base64) => {
                                                                // Descerealize
                                                                if let Ok(swap_bytes) =
                                                                    general_purpose::STANDARD
                                                                        .decode(swap_base64)
                                                                {
                                                                    if let Ok(versioned_tx) =
                                                                        bincode::deserialize::<
                                                                            VersionedTransaction,
                                                                        >(
                                                                            &swap_bytes
                                                                        )
                                                                    {
                                                                        // Sign
                                                                        // VersionedTransaction signing is different, usually needs latest blockhash?
                                                                        // Jupiter provides blockhash in the tx.
                                                                        // We just need to sign.
                                                                        let signed_tx =
                                                                            VersionedTransaction::try_new(
                                                                                versioned_tx.message,
                                                                                &[kp_arc.as_ref()],
                                                                            );

                                                                        match signed_tx {
                                                                            Ok(tx_signed) => {
                                                                                // Send
                                                                                match nc
                                                                                    .rpc_client
                                                                                    .send_transaction(
                                                                                        &tx_signed,
                                                                                    )
                                                                                    .await
                                                                                {
                                                                                    Ok(sig) => {
                                                                                        let _ = tx_swap
                                                                                            .send(
                                                                                                AppEvent::Log(
                                                                                                    format!(
                                                                                                        "Swap sent: {}",
                                                                                                        sig
                                                                                                    ),
                                                                                                ),
                                                                                            )
                                                                                            .await;
                                                                                    }
                                                                                    Err(e) => {
                                                                                        let _ = tx_swap.send(AppEvent::Log(format!("Send failed: {}", e))).await;
                                                                                    }
                                                                                }
                                                                            }
                                                                            Err(e) => {
                                                                                let _ = tx_swap
                                                                                    .send(AppEvent::Log(
                                                                                        format!(
                                                                                            "Signing failed: {}",
                                                                                            e
                                                                                        ),
                                                                                    ))
                                                                                    .await;
                                                                            }
                                                                        }
                                                                    } else {
                                                                        let _ = tx_swap
                                                                            .send(AppEvent::Log(
                                                                                "Failed to deserialize tx"
                                                                                    .to_string(),
                                                                            ))
                                                                            .await;
                                                                    }
                                                                } else {
                                                                    let _ = tx_swap
                                                                        .send(AppEvent::Log(
                                                                            "Failed to decode base64"
                                                                                .to_string(),
                                                                        ))
                                                                        .await;
                                                                }
                                                            }
                                                            Err(e) => {
                                                                let _ = tx_swap
                                                                    .send(AppEvent::Log(format!(
                                                                        "Swap API failed: {}",
                                                                        e
                                                                    )))
                                                                    .await;
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        let _ = tx_swap
                                                            .send(AppEvent::Log(format!(
                                                                "Quote failed: {}",
                                                                e
                                                            )))
                                                            .await;
                                                    }
                                                }
                                            });
                                        } else {
                                            app.add_log(
                                                "Cannot swap: No wallet loaded.".to_string(),
                                            );
                                        }
                                    }
                                    KeyCode::Char('/') => {
                                        app.show_search_modal = true;
                                    }
                                    KeyCode::Char('t') => {
                                        app.toggle_theme();
                                    }
                                    // Chart Navigation
                                    KeyCode::Right => {
                                        app.chart_x_offset += 1.0;
                                    }
                                    KeyCode::Left => {
                                        app.chart_x_offset -= 1.0;
                                    }
                                    KeyCode::Up => {
                                        app.chart_y_offset += 0.0001;
                                    }
                                    KeyCode::Down => {
                                        app.chart_y_offset -= 0.0001;
                                    }
                                    KeyCode::Tab => {
                                        app.bottom_tab_index = (app.bottom_tab_index + 1) % 6;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                Event::Mouse(mouse) => {
                    let size = terminal.size()?;
                    let size = Rect::new(0, 0, size.width, size.height);

                    // Calculate Layout Rects (matching ui.rs)
                    let vertical_layout = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(3), // Navbar
                            Constraint::Min(0),    // Main
                        ])
                        .split(size);

                    let navbar_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(20),
                            Constraint::Percentage(60),
                            Constraint::Percentage(20),
                        ])
                        .split(vertical_layout[0]);

                    let main_content_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(app.col_constraints[0]),
                            Constraint::Percentage(app.col_constraints[1]),
                            Constraint::Percentage(app.col_constraints[2]),
                        ])
                        .split(vertical_layout[1]);

                    let center_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Percentage(app.row_constraints[0]),
                            Constraint::Percentage(app.row_constraints[1]),
                        ])
                        .split(main_content_chunks[1]);

                    match mouse.kind {
                        MouseEventKind::Down(_) => {
                            let x = mouse.column;
                            let y = mouse.row;

                            // 1. Check Navbar Search Click
                            let is_search_click = x >= navbar_chunks[1].left()
                                && x < navbar_chunks[1].right()
                                && y >= navbar_chunks[1].top()
                                && y < navbar_chunks[1].bottom();

                            if is_search_click {
                                app.show_search_modal = true;
                            } else if app.show_search_modal {
                                // If modal is open, ignore clicks on underlying UI
                            } else {
                                // Check Vertical Separators
                                let col1_right = main_content_chunks[0].right();
                                let col2_right = main_content_chunks[1].right();

                                if x >= col1_right.saturating_sub(1) && x <= col1_right + 1 {
                                    app.drag_state = Some(DragState::ColFirst);
                                } else if x >= col2_right.saturating_sub(1) && x <= col2_right + 1 {
                                    app.drag_state = Some(DragState::ColSecond);
                                } else {
                                    // Check Horizontal Separator (only in center column)
                                    if x >= main_content_chunks[1].left()
                                        && x < main_content_chunks[1].right()
                                    {
                                        let row1_bottom = center_chunks[0].bottom();
                                        if y >= row1_bottom.saturating_sub(1)
                                            && y <= row1_bottom + 1
                                        {
                                            app.drag_state = Some(DragState::RowCenter);
                                        } else {
                                            // Check for Tab Clicks in Bottom Panel
                                            let bottom_panel_top = center_chunks[1].top();
                                            if y >= bottom_panel_top && y < bottom_panel_top + 3 {
                                                // Tab click logic
                                                let panel_width = main_content_chunks[1].width;
                                                if panel_width > 0 {
                                                    let tab_width = panel_width / 6;
                                                    let rel_x = x.saturating_sub(
                                                        main_content_chunks[1].left(),
                                                    );
                                                    let clicked_tab = (rel_x / tab_width) as usize;
                                                    if clicked_tab < 6 {
                                                        app.bottom_tab_index = clicked_tab;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        MouseEventKind::Drag(_) => {
                            if let Some(state) = app.drag_state {
                                let total_width = size.width as f64;
                                let total_height = main_content_chunks[1].height as f64;
                                let mouse_x = mouse.column as f64;
                                let mouse_y = mouse.row;

                                match state {
                                    DragState::ColFirst => {
                                        let new_p0 = ((mouse_x / total_width) * 100.0)
                                            .clamp(5.0, 50.0)
                                            as u16;
                                        let p2 = app.col_constraints[2];
                                        if new_p0 + p2 < 100 {
                                            app.col_constraints[0] = new_p0;
                                            app.col_constraints[1] = 100 - new_p0 - p2;
                                        }
                                    }
                                    DragState::ColSecond => {
                                        let combined_p0_p1 =
                                            ((mouse_x / total_width) * 100.0).clamp(10.0, 95.0);
                                        let p0 = app.col_constraints[0];
                                        if combined_p0_p1 > p0 as f64 {
                                            let new_p1 = (combined_p0_p1 - p0 as f64) as u16;
                                            if p0 + new_p1 < 100 {
                                                app.col_constraints[1] = new_p1;
                                                app.col_constraints[2] = 100 - p0 - new_p1;
                                            }
                                        }
                                    }
                                    DragState::RowCenter => {
                                        let center_top = main_content_chunks[1].top();
                                        if mouse_y >= center_top {
                                            let rel_y = (mouse_y - center_top) as f64;
                                            let new_row0 = ((rel_y / total_height) * 100.0)
                                                .clamp(10.0, 90.0)
                                                as u16;
                                            app.row_constraints[0] = new_row0;
                                            app.row_constraints[1] = 100 - new_row0;
                                        }
                                    }
                                }
                            }
                        }
                        MouseEventKind::Up(_) => {
                            app.drag_state = None;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}
