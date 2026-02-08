use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{io, sync::Arc, time::Duration};

use tokio::sync::mpsc;

use trading_terminal::app::App;
use trading_terminal::ui::ui;

use base64::{Engine as _, engine::general_purpose};
use solana_sdk::{
    signer::{Signer, keypair::read_keypair_file},
    transaction::VersionedTransaction,
};
use trading_terminal::network::{IndexerClient, NetworkClient};
use trading_terminal::swap::JupiterClient;

enum AppEvent {
    Log(String),
    TokensFetched(Vec<String>),
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = trading_terminal::config::load_config();
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

        if crossterm::event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => {
                        app.quit();
                        return Ok(());
                    }
                    KeyCode::Char('s') => {
                        if let Some(kp) = &wallet_keypair {
                            app.add_log("Initiating swap...".to_string());
                            let tx_swap = tx.clone();
                            let nc = network_client.clone();
                            let kp_arc = kp.clone();

                            tokio::spawn(async move {
                                let jupiter = JupiterClient::new();
                                // USDC -> SOL (Swap back for testing if needed, or stick to SOL->USDC)
                                // Let's do SOL -> USDC
                                let input_mint = "So11111111111111111111111111111111111111112";
                                let output_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
                                let amount = 100_000; // 0.0001 SOL

                                let quote_res =
                                    jupiter.get_quote(input_mint, output_mint, amount, 50).await;

                                match quote_res {
                                    Ok(quote) => {
                                        let _ = tx_swap
                                            .send(AppEvent::Log(format!(
                                                "Quote: Out {}",
                                                quote.out_amount
                                            )))
                                            .await;

                                        // Get Swap Transaction
                                        let user_pubkey = kp_arc.pubkey().to_string();
                                        match jupiter
                                            .get_swap_transaction(&user_pubkey, quote)
                                            .await
                                        {
                                            Ok(swap_base64) => {
                                                // Descerealize
                                                if let Ok(swap_bytes) =
                                                    general_purpose::STANDARD.decode(swap_base64)
                                                {
                                                    if let Ok(versioned_tx) = bincode::deserialize::<
                                                        VersionedTransaction,
                                                    >(
                                                        &swap_bytes
                                                    ) {
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
                                                                    .send_transaction(&tx_signed)
                                                                    .await
                                                                {
                                                                    Ok(sig) => {
                                                                        let _ = tx_swap
                                                                            .send(AppEvent::Log(
                                                                                format!(
                                                                                    "Swap sent: {}",
                                                                                    sig
                                                                                ),
                                                                            ))
                                                                            .await;
                                                                    }
                                                                    Err(e) => {
                                                                        let _ = tx_swap.send(AppEvent::Log(format!("Send failed: {}", e))).await;
                                                                    }
                                                                }
                                                            }
                                                            Err(e) => {
                                                                let _ = tx_swap
                                                                    .send(AppEvent::Log(format!(
                                                                        "Signing failed: {}",
                                                                        e
                                                                    )))
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
                                                            "Failed to decode base64".to_string(),
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
                                            .send(AppEvent::Log(format!("Quote failed: {}", e)))
                                            .await;
                                    }
                                }
                            });
                        } else {
                            app.add_log("Cannot swap: No wallet loaded.".to_string());
                        }
                    }
                    KeyCode::Char('t') => {
                        app.toggle_theme();
                    }
                    _ => {}
                }
            }
        }
    }
}
