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

use image;
use tx_terminal::app::{App, CurrentScreen, DragState, Theme};
use tx_terminal::ui::ui;

use base64::{engine::general_purpose, Engine as _};
use solana_sdk::{
    signer::{keypair::read_keypair_file, Signer},
    transaction::VersionedTransaction,
};
use tx_terminal::network::{
    IndexerClient, JupiterCandle, JupiterHolder, JupiterTokenData, JupiterTransaction,
    NetworkClient,
};

mod time_utils;
use time_utils::{format_full_time, format_unix_time};
pub mod theme;
use tx_terminal::swap::JupiterClient;

enum AppEvent {
    Log(String),
    TokensFetched(Vec<String>),
    ApiTokensFetched(Vec<tx_terminal::app::Token>),
    SpecificTokenFetched(tx_terminal::network::ApiToken),
    JupiterTokenInfoFetched(JupiterTokenData),
    ImageFetched(String, image::DynamicImage), // mint, decoded image
    ChartDataFetched(Vec<JupiterCandle>),
    TransactionsFetched(Vec<JupiterTransaction>),
    HoldersFetched(Vec<JupiterHolder>),
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = tx_terminal::config::load_config();
    let network_client = NetworkClient::new(&config.rpc_url);
    let indexer_client = IndexerClient::new(config.indexer_api_url.clone());

    // wallet variables will be determined later; theme applied after we know them

    // Channel for async events
    let (tx, mut rx) = mpsc::channel(100);

    // Load wallet: priority is CLI --keypair-path, then saved wallet at ~/.config/tx-terminal/
    let (wallet_pubkey, balance, wallet_keypair) = if let Some(path) = &config.keypair_path {
        // CLI-provided keypair takes priority
        if let Ok(kp) = read_keypair_file(path) {
            let pubkey = kp.pubkey();
            let balance = network_client.get_balance(&pubkey).await.unwrap_or(0);
            (Some(pubkey), balance, Some(Arc::new(kp)))
        } else {
            (None, 0, None)
        }
    } else if tx_terminal::wallet::wallet_exists() {
        // Auto-load saved wallet
        match tx_terminal::wallet::load_wallet() {
            Ok(kp) => {
                let pubkey = kp.pubkey();
                let balance = network_client.get_balance(&pubkey).await.unwrap_or(0);
                (Some(pubkey), balance, Some(Arc::new(kp)))
            }
            Err(_) => (None, 0, None),
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

    // Create app after wallet info available
    let mut app = App::new(wallet_pubkey, balance);
    // apply theme
    match config.theme.as_str() {
        "light" => app.theme = Theme::Light,
        "bloomberg" => app.theme = Theme::Bloomberg,
        _ => app.theme = Theme::Dark,
    }

    if let Some(pk) = wallet_pubkey {
        app.add_log(format!("Wallet loaded: {}", pk));
    } else {
        app.add_log("No wallet loaded. Press 'c' to create/connect.".to_string());
    }

    // Generate demo data for immediate display
    app.generate_demo_transactions();
    app.generate_demo_holders();
    app.generate_demo_candles(60);

    // Fetch initial token list from API
    let tx_api_tokens = tx.clone();
    let indexer_client_clone = indexer_client.clone();
    tokio::spawn(async move {
        if let Ok(tokens) = indexer_client_clone.fetch_tokens_from_api().await {
            let _ = tx_api_tokens.send(AppEvent::ApiTokensFetched(tokens)).await;
        } else {
            let _ = tx_api_tokens
                .send(AppEvent::Log("Failed to load tokens from API.".to_string()))
                .await;
        }
    });

    // Fetch initial token list
    let tx_tokens = tx.clone();
    let indexer_client_clone2 = indexer_client.clone();
    tokio::spawn(async move {
        if let Ok(tokens) = indexer_client_clone2.fetch_tokens().await {
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
        indexer_client,
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
    mut wallet_keypair: Option<Arc<solana_sdk::signer::keypair::Keypair>>,
    indexer_client: IndexerClient,
) -> Result<()> {
    let mut redraw = true;
    loop {
        if app.should_quit {
            return Ok(());
        }

        if redraw {
            terminal.draw(|f| ui(f, app))?;
            redraw = false;
        }

        // Transaction pooling on millisecond interval
        if app.current_screen == CurrentScreen::TokenDetails
            && !app.token_info.mint.is_empty()
            && app.last_tx_poll.elapsed().as_millis() >= app.tx_pool_interval_ms as u128
        {
            app.last_tx_poll = std::time::Instant::now();
            let mint_for_poll = app.token_info.mint.clone();
            let tx_poll = tx.clone();
            let indexer_poll = indexer_client.clone();

            tokio::spawn(async move {
                if let Ok(txs) = indexer_poll
                    .fetch_transactions(&mint_for_poll, Some(50))
                    .await
                {
                    let _ = tx_poll.send(AppEvent::TransactionsFetched(txs)).await;
                }
            });
        }

        // Check for async events from network/downloader
        while let Ok(event) = rx.try_recv() {
            redraw = true;
            match event {
                AppEvent::Log(msg) => app.add_log(msg),
                AppEvent::TokensFetched(tokens) => {
                    app.token_list = tokens;
                    app.add_log(format!(
                        "Loaded {} tokens via async task.",
                        app.token_list.len()
                    ));
                }
                AppEvent::ApiTokensFetched(tokens) => {
                    app.add_log(format!("Loaded {} tokens from API.", tokens.len()));
                    // Trigger image downloads for new tokens
                    for token in &tokens {
                        if !token.image_url.is_empty() {
                            let tx_img = tx.clone();
                            let url = token.image_url.clone();
                            let mint = token.mint.clone();
                            tokio::spawn(async move {
                                let client = reqwest::Client::new();
                                if let Ok(resp) = client.get(url).send().await {
                                    if let Ok(bytes) = resp.bytes().await {
                                        if let Ok(img) = image::load_from_memory(&bytes) {
                                            // Resize the image to a manageable size for terminal rendering
                                            let resized = img.thumbnail(128, 128);
                                            let _ = tx_img
                                                .send(AppEvent::ImageFetched(mint, resized))
                                                .await;
                                        }
                                    }
                                }
                            });
                        }
                    }
                    app.update_tokens_from_api(tokens);
                }
                AppEvent::SpecificTokenFetched(api_token) => {
                    app.add_log(format!("Loaded token details: {}", api_token.symbol));
                    if let Some(url) = &api_token.imageUrl {
                        if !url.is_empty() && !app.image_cache.contains_key(&api_token.mintAddress)
                        {
                            let tx_img = tx.clone();
                            let url_clone = url.clone();
                            let mint = api_token.mintAddress.clone();
                            tokio::spawn(async move {
                                let client = reqwest::Client::new();
                                if let Ok(resp) = client.get(url_clone).send().await {
                                    if let Ok(bytes) = resp.bytes().await {
                                        if let Ok(img) = image::load_from_memory(&bytes) {
                                            // Resize the image to a manageable size for terminal rendering
                                            let resized = img.thumbnail(128, 128);
                                            let _ = tx_img
                                                .send(AppEvent::ImageFetched(mint, resized))
                                                .await;
                                        }
                                    }
                                }
                            });
                        }
                    }
                    app.update_selected_token_from_api(api_token);
                }
                AppEvent::JupiterTokenInfoFetched(jupiter_token) => {
                    app.add_log(format!("✅ Loaded Jupiter data: {}", jupiter_token.symbol));
                    app.token_info.name = jupiter_token.name.clone();
                    app.token_info.symbol = jupiter_token.symbol.clone();
                    app.token_info.mint = jupiter_token.address.clone();
                    app.token_info.market_cap = jupiter_token.mcap.unwrap_or(0.0);
                    app.token_info.vol_24h = jupiter_token.volume24h.unwrap_or(0.0);
                    app.token_info.liquidity = jupiter_token.liquidity.unwrap_or(0.0);
                    app.token_info.holders = jupiter_token.holderCount.unwrap_or(0);
                    app.token_info.change_5m =
                        jupiter_token.stats5m.as_ref().map(|s| s.priceChange);
                    app.token_info.change_1h =
                        jupiter_token.stats1h.as_ref().map(|s| s.priceChange);
                    app.token_info.change_6h = jupiter_token
                        .stats6h
                        .as_ref()
                        .map(|s| s.priceChange)
                        .unwrap_or(0.0);
                    app.token_info.change_24h = jupiter_token
                        .stats24h
                        .as_ref()
                        .map(|s| s.priceChange)
                        .unwrap_or(0.0);
                    app.token_info.image_url = jupiter_token.logoURI.unwrap_or_default();
                }
                AppEvent::ImageFetched(mint, img) => {
                    app.image_cache.insert(mint, img);
                }
                AppEvent::ChartDataFetched(candles) => {
                    if candles.is_empty() {
                        app.add_log("📊 No chart data available (API returned empty)".to_string());
                    } else {
                        app.add_log(format!(
                            "📊 Loaded {} candles from Jupiter API",
                            candles.len()
                        ));
                        app.candles = candles
                            .into_iter()
                            .map(|c| tx_terminal::app::Candle {
                                open: c.open,
                                high: c.high,
                                low: c.low,
                                close: c.close,
                                time: c.time,
                            })
                            .collect();
                    }
                }
                AppEvent::TransactionsFetched(txs) => {
                    if txs.is_empty() {
                        app.add_log(
                            "🔄 No transactions available (API returned empty)".to_string(),
                        );
                    } else {
                        app.add_log(format!(
                            "🔄 Loaded {} recent transactions from Jupiter",
                            txs.len()
                        ));
                        app.recent_trades = txs
                            .into_iter()
                            .map(|tx| {
                                let age = tx
                                    .blockTime
                                    .map(|t| format_unix_time(t))
                                    .unwrap_or_else(|| "Just now".to_string());
                                let date = tx
                                    .blockTime
                                    .map(|t| format_full_time(t))
                                    .unwrap_or_else(|| "-".to_string());
                                tx_terminal::app::Trade {
                                    timestamp: tx.blockTime,
                                    date,
                                    age,
                                    type_: tx.tradeDirection.clone(),
                                    price: tx.price,
                                    volume: tx.amount,
                                    sol: tx.vSolChange.unwrap_or(0.0),
                                    trader: tx.maker.clone(),
                                }
                            })
                            .collect();
                    }
                }
                AppEvent::HoldersFetched(holders) => {
                    if holders.is_empty() {
                        app.add_log("👥 No holder data available for this token".to_string());
                    } else {
                        app.add_log(format!("Loaded {} token holders", holders.len()));
                        app.jupiter_holders = holders
                            .into_iter()
                            .map(|h| tx_terminal::app::Holder {
                                address: h.address.clone(),
                                percent: h.percent.unwrap_or(0.0),
                                sol_balance: h.solBalanceDisplay.unwrap_or(0.0),
                                amount: h.amount.unwrap_or(0.0),
                                is_dev: false,
                            })
                            .collect();
                    }
                }
            }
        }

        if crossterm::event::poll(Duration::from_millis(50))? {
            redraw = true;
            match crossterm::event::read()? {
                Event::Key(key) => {
                    // first allow command bar / panel navigation to consume key
                    if app.handle_key(key) {
                        // consumed; skip further handling
                    } else if app.show_connect_wallet_modal {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('q') => {
                                app.close_wallet_connect_modal();
                            }
                            KeyCode::Enter => {
                                app.close_wallet_connect_modal();
                                app.add_log("Wallet connected!".to_string());
                            }
                            _ => {}
                        }
                    } else {
                        // Global Keys (only when modal not shown)
                        if key.code == KeyCode::Char('q') {
                            app.quit();
                            return Ok(());
                        }

                        // Wallet connect
                        if key.code == KeyCode::Char('c') {
                            app.open_wallet_connect_modal();
                            // Create or load real wallet
                            if let Some(kp) = app.connect_wallet() {
                                wallet_keypair = Some(Arc::new(kp));
                            }
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
                                        let max_rows = match app.home_selected_col {
                                            0 => app.new_tokens.len(),
                                            1 => app.bonding_tokens.len(),
                                            2 => app.migrated_tokens.len(),
                                            _ => 0,
                                        };
                                        if app.home_selected_row < max_rows.saturating_sub(1) {
                                            app.home_selected_row += 1;
                                        }
                                    }
                                    KeyCode::Up => {
                                        if app.home_selected_row > 0 {
                                            app.home_selected_row -= 1;
                                        }
                                    }
                                    KeyCode::Enter => {
                                        // Select token logic
                                        app.add_log(format!(
                                            "DEBUG: Enter pressed - col={}, row={}",
                                            app.home_selected_col, app.home_selected_row
                                        ));
                                        let token = match app.home_selected_col {
                                            0 => {
                                                app.add_log(format!(
                                                    "DEBUG: new_tokens has {} items",
                                                    app.new_tokens.len()
                                                ));
                                                app.new_tokens.get(app.home_selected_row)
                                            }
                                            1 => {
                                                app.add_log(format!(
                                                    "DEBUG: bonding_tokens has {} items",
                                                    app.bonding_tokens.len()
                                                ));
                                                app.bonding_tokens.get(app.home_selected_row)
                                            }
                                            2 => {
                                                app.add_log(format!(
                                                    "DEBUG: migrated_tokens has {} items",
                                                    app.migrated_tokens.len()
                                                ));
                                                app.migrated_tokens.get(app.home_selected_row)
                                            }
                                            _ => None,
                                        };

                                        if let Some(t) = token {
                                            let token_name = t.name.clone();
                                            let token_symbol = t.symbol.clone();
                                            let token_mint = t.mint.clone();
                                            app.add_log(format!(
                                                "DEBUG: Selected token: {} ({})",
                                                token_name, token_symbol
                                            ));
                                            // Fetch Jupiter Token Info (includes volume, liquidity, mcap, holders, stats)
                                            let token_query = token_symbol.clone();
                                            let tx_jupiter = tx.clone();
                                            let indexer_jupiter = indexer_client.clone();

                                            tokio::spawn(async move {
                                                match indexer_jupiter
                                                    .fetch_token_info(&token_query)
                                                    .await
                                                {
                                                    Ok(token_data) => {
                                                        let _ = tx_jupiter
                                                            .send(
                                                                AppEvent::JupiterTokenInfoFetched(
                                                                    token_data,
                                                                ),
                                                            )
                                                            .await;
                                                    }
                                                    Err(e) => {
                                                        let _ = tx_jupiter
                                                            .send(AppEvent::Log(format!(
                                                                "⚠️ Jupiter API: {}",
                                                                e
                                                            )))
                                                            .await;
                                                    }
                                                }
                                            });

                                            // Fetch full token details from API (fallback)
                                            let mint_address = token_mint.clone();
                                            let tx_token = tx.clone();
                                            let indexer_clone = indexer_client.clone();

                                            tokio::spawn(async move {
                                                match indexer_clone
                                                    .fetch_token_by_mint(&mint_address)
                                                    .await
                                                {
                                                    Ok(fetched_token) => {
                                                        let _ = tx_token
                                                            .send(AppEvent::SpecificTokenFetched(
                                                                fetched_token,
                                                            ))
                                                            .await;
                                                    }
                                                    Err(e) => {
                                                        let _ = tx_token
                                                            .send(AppEvent::Log(format!(
                                                                "Failed to fetch token: {}",
                                                                e
                                                            )))
                                                            .await;
                                                    }
                                                }
                                            });

                                            // Fetch Jupiter Chart Data
                                            let mint_for_chart = token_mint.clone();
                                            let tx_chart = tx.clone();
                                            let indexer_for_chart = indexer_client.clone();
                                            tokio::spawn(async move {
                                                match indexer_for_chart
                                                    .fetch_token_chart_data(&mint_for_chart, "1h")
                                                    .await
                                                {
                                                    Ok(candles) => {
                                                        let _ = tx_chart
                                                            .send(AppEvent::ChartDataFetched(
                                                                candles,
                                                            ))
                                                            .await;
                                                    }
                                                    Err(e) => {
                                                        let _ = tx_chart
                                                            .send(AppEvent::Log(format!(
                                                                "Chart error: {}",
                                                                e
                                                            )))
                                                            .await;
                                                    }
                                                }
                                            });

                                            // Fetch Jupiter Transactions
                                            let mint_for_tx = token_mint.clone();
                                            let tx_txs = tx.clone();
                                            let indexer_for_tx = indexer_client.clone();
                                            tokio::spawn(async move {
                                                match indexer_for_tx
                                                    .fetch_transactions(&mint_for_tx, Some(50))
                                                    .await
                                                {
                                                    Ok(txs) => {
                                                        let _ = tx_txs
                                                            .send(AppEvent::TransactionsFetched(
                                                                txs,
                                                            ))
                                                            .await;
                                                    }
                                                    Err(e) => {
                                                        let _ = tx_txs
                                                            .send(AppEvent::Log(format!(
                                                                "Failed to fetch transactions: {}",
                                                                e
                                                            )))
                                                            .await;
                                                    }
                                                }
                                            });

                                            // Fetch Jupiter Holders
                                            let mint_for_holders = token_mint.clone();
                                            let tx_holders = tx.clone();
                                            let indexer_for_holders = indexer_client.clone();
                                            tokio::spawn(async move {
                                                match indexer_for_holders
                                                    .fetch_holders(&mint_for_holders, Some(100))
                                                    .await
                                                {
                                                    Ok(holders) => {
                                                        let _ = tx_holders
                                                            .send(AppEvent::HoldersFetched(holders))
                                                            .await;
                                                    }
                                                    Err(e) => {
                                                        let _ = tx_holders
                                                            .send(AppEvent::Log(format!(
                                                                "Failed to fetch holders: {}",
                                                                e
                                                            )))
                                                            .await;
                                                    }
                                                }
                                            });

                                            // Switch to TokenDetails screen
                                            app.current_screen = CurrentScreen::TokenDetails;
                                        } else {
                                            app.add_log(
                                                "DEBUG: No token found at selected position!"
                                                    .to_string(),
                                            );
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
                                        KeyCode::Char('y') => {
                                            // Copy Mint
                                            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                                if clipboard
                                                    .set_text(app.token_info.mint.clone())
                                                    .is_ok()
                                                {
                                                    app.add_log(
                                                        "Mint address copied to clipboard!"
                                                            .to_string(),
                                                    );
                                                }
                                            }
                                        }
                                        KeyCode::Char('v') => {
                                            // Open creator on Solscan
                                            let url = format!(
                                                "https://solscan.io/account/{}",
                                                app.token_info.creator_address
                                            );
                                            let _ =
                                                std::process::Command::new("open").arg(url).spawn();
                                            app.add_log(
                                                "Opening creator on Solscan...".to_string(),
                                            );
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
                                                                        .send(AppEvent::Log(
                                                                            format!(
                                                                        "Swap API failed: {}",
                                                                        e
                                                                    ),
                                                                        ))
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
                                        KeyCode::Char('[') | KeyCode::Char('i') => {
                                            // Previous interval
                                            use tx_terminal::app::ChartInterval;
                                            app.chart_interval = match app.chart_interval {
                                                ChartInterval::OneMin => ChartInterval::OneMin,
                                                ChartInterval::FiveMin => ChartInterval::OneMin,
                                                ChartInterval::FifteenMin => ChartInterval::FiveMin,
                                                ChartInterval::OneHour => ChartInterval::FifteenMin,
                                                ChartInterval::FourHour => ChartInterval::OneHour,
                                                ChartInterval::OneDay => ChartInterval::FourHour,
                                            };
                                            // Fetch chart data for new interval
                                            let mint_for_chart = app.token_info.mint.clone();
                                            let tx_chart = tx.clone();
                                            let indexer_for_chart = indexer_client.clone();
                                            let interval_str = app.chart_interval.as_string();
                                            let candle_count = app.chart_interval.candle_count();
                                            tokio::spawn(async move {
                                                if !mint_for_chart.is_empty() {
                                                    match indexer_for_chart
                                                        .fetch_chart_data(
                                                            &mint_for_chart,
                                                            &interval_str,
                                                            candle_count,
                                                        )
                                                        .await
                                                    {
                                                        Ok(candles) => {
                                                            let _ = tx_chart
                                                                .send(AppEvent::ChartDataFetched(
                                                                    candles,
                                                                ))
                                                                .await;
                                                        }
                                                        Err(e) => {
                                                            let _ = tx_chart
                                                                .send(AppEvent::Log(format!(
                                                                    "Failed to fetch chart: {}",
                                                                    e
                                                                )))
                                                                .await;
                                                        }
                                                    }
                                                }
                                            });
                                        }
                                        KeyCode::Char(']') | KeyCode::Char('o') => {
                                            // Next interval
                                            use tx_terminal::app::ChartInterval;
                                            app.chart_interval = match app.chart_interval {
                                                ChartInterval::OneMin => ChartInterval::FiveMin,
                                                ChartInterval::FiveMin => ChartInterval::FifteenMin,
                                                ChartInterval::FifteenMin => ChartInterval::OneHour,
                                                ChartInterval::OneHour => ChartInterval::FourHour,
                                                ChartInterval::FourHour => ChartInterval::OneDay,
                                                ChartInterval::OneDay => ChartInterval::OneDay,
                                            };
                                            // Fetch chart data for new interval
                                            let mint_for_chart = app.token_info.mint.clone();
                                            let tx_chart = tx.clone();
                                            let indexer_for_chart = indexer_client.clone();
                                            let interval_str = app.chart_interval.as_string();
                                            let candle_count = app.chart_interval.candle_count();
                                            tokio::spawn(async move {
                                                if !mint_for_chart.is_empty() {
                                                    match indexer_for_chart
                                                        .fetch_chart_data(
                                                            &mint_for_chart,
                                                            &interval_str,
                                                            candle_count,
                                                        )
                                                        .await
                                                    {
                                                        Ok(candles) => {
                                                            let _ = tx_chart
                                                                .send(AppEvent::ChartDataFetched(
                                                                    candles,
                                                                ))
                                                                .await;
                                                        }
                                                        Err(e) => {
                                                            let _ = tx_chart
                                                                .send(AppEvent::Log(format!(
                                                                    "Failed to fetch chart: {}",
                                                                    e
                                                                )))
                                                                .await;
                                                        }
                                                    }
                                                }
                                            });
                                        }
                                        _ => {}
                                    }
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
                            } else if app.current_screen == CurrentScreen::TokenDetails
                                && x >= main_content_chunks[0].left()
                                && x < main_content_chunks[0].right()
                                && y >= main_content_chunks[0].top()
                                && y < main_content_chunks[0].bottom()
                            {
                                let rel_y = y - main_content_chunks[0].top();
                                match rel_y {
                                    2 => {
                                        // Mint Row
                                        if let Ok(mut clipboard) = arboard::Clipboard::new() {
                                            let _ = clipboard.set_text(app.token_info.mint.clone());
                                            app.add_log("Mint address copied!".to_string());
                                        }
                                    }
                                    3 => {
                                        // Creator Row
                                        let url = format!(
                                            "https://solscan.io/account/{}",
                                            app.token_info.creator_address
                                        );
                                        let _ = std::process::Command::new("open").arg(url).spawn();
                                        app.add_log("Opening Solscan...".to_string());
                                    }
                                    7 => {
                                        // Website
                                        if !app.token_info.website.is_empty() {
                                            let _ = std::process::Command::new("open")
                                                .arg(&app.token_info.website)
                                                .spawn();
                                        }
                                    }
                                    8 => {
                                        // Twitter
                                        if !app.token_info.twitter.is_empty() {
                                            let _ = std::process::Command::new("open")
                                                .arg(&app.token_info.twitter)
                                                .spawn();
                                        }
                                    }
                                    9 => {
                                        // Telegram
                                        if !app.token_info.telegram.is_empty() {
                                            let _ = std::process::Command::new("open")
                                                .arg(&app.token_info.telegram)
                                                .spawn();
                                        }
                                    }
                                    _ => {}
                                }
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
