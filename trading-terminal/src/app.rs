use crossterm::event::KeyCode;
use image::DynamicImage;
use ratatui::layout::Rect;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
    pub description: String,
    pub market_cap: f64,
    pub fdv: f64,
    pub liquidity: f64,
    pub holders: u64,
    pub price: f64,
    pub fees_paid: f64,
    pub org_score: f64,
    pub bonding_curve: f64,
    pub change_5m: Option<f64>,
    pub change_1h: Option<f64>,
    pub change_6h: f64,
    pub change_24h: f64,
    pub vol_24h: f64,
    pub net_vol_24h: f64,
    pub sell_pressure: f64,
    pub traders_24h: u64,
    pub net_buyers: Option<i64>,
    pub net_buy_trend_24h: f64,
    pub vol_delta_percent: f64,
    pub liquidity_delta_percent: f64,
    pub holders_delta_percent: Option<f64>,
    pub mint: String,
    pub website: String,
    pub twitter: String,
    pub telegram: String,
    pub image_url: String,
    pub creator_address: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub name: String,
    pub symbol: String,
    pub price: f64,
    pub market_cap: f64,
    pub change_24h: f64,
    pub volume: f64,
    pub txns: u32,
    pub image_asc: String,
    pub image_url: String,
    pub bonding: f64, // 0-100%
    pub mint: String,
}

#[derive(Clone)]
pub struct Holder {
    pub address: String,
    /// Percentage owned (0-100)
    pub percent: f64,
    /// SOL balance of this holder
    pub sol_balance: f64,
    /// Token amount held
    pub amount: f64,
    pub is_dev: bool,
}

pub struct Trade {
    /// Unix timestamp from the source
    pub timestamp: Option<u64>,
    /// Date string shown in the table (YYYY-MM-DD HH:MM)
    pub date: String,
    /// Human‑readable age (e.g. "5m ago")
    pub age: String,

    pub type_: String, // "Buy" or "Sell"
    pub price: f64,
    pub volume: f64,
    /// SOL equivalent amount, when available
    pub sol: f64,
    /// Counterparty / trader identifier
    pub trader: String,
}

#[derive(Clone, Copy)]
pub struct Candle {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub time: u64, // Unix timestamp
}

#[derive(Clone, Copy, PartialEq)]
pub enum ChartInterval {
    OneMin,
    FiveMin,
    FifteenMin,
    OneHour,
    FourHour,
    OneDay,
}

impl ChartInterval {
    pub fn as_string(&self) -> String {
        match self {
            ChartInterval::OneMin => "1m".to_string(),
            ChartInterval::FiveMin => "5m".to_string(),
            ChartInterval::FifteenMin => "15m".to_string(),
            ChartInterval::OneHour => "1h".to_string(),
            ChartInterval::FourHour => "4h".to_string(),
            ChartInterval::OneDay => "1d".to_string(),
        }
    }

    pub fn candle_count(&self) -> u32 {
        match self {
            ChartInterval::OneMin => 60,
            ChartInterval::FiveMin => 96,
            ChartInterval::FifteenMin => 96,
            ChartInterval::OneHour => 168,
            ChartInterval::FourHour => 168,
            ChartInterval::OneDay => 365,
        }
    }
}

pub enum Theme {
    Light,
    Dark,
    Bloomberg,
}

#[derive(Clone, Copy, PartialEq)]
pub enum CurrentScreen {
    Home,
    TokenDetails,
}

// ─── PANEL SYSTEM ─────────────────────────────────────────────────────────────

use crate::panels::watchlist::WatchlistPanel;

#[derive(Clone)]
pub struct Panel {
    pub id: usize,
    pub title: String,
    pub func: String, // e.g. "WATCHLIST", "TOKEN"
    pub rect: Rect,
    pub focused: bool,
    pub content: PanelContent,
}

#[derive(Clone)]
pub enum PanelContent {
    Home,
    TokenDetails,
    Watchlist(WatchlistPanel),
    // other panel types may be added later
}

impl PanelContent {
    pub fn title(&self) -> &'static str {
        match self {
            PanelContent::Home => "HOME",
            PanelContent::TokenDetails => "TOKEN",
            PanelContent::Watchlist(_) => "WATCHLIST",
        }
    }
}

impl Panel {
    pub fn new(id: usize, content: PanelContent) -> Self {
        let func = content.title().to_string();
        Panel {
            id,
            title: func.clone(),
            func,
            rect: Rect::new(0, 0, 40, 20),
            focused: false,
            content,
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum WalletModalState {
    Idle,
    NewWalletCreated { pubkey: String, path: String },
    ExistingWalletLoaded { pubkey: String, path: String },
    Error(String),
}

pub struct App {
    pub should_quit: bool,
    pub current_screen: CurrentScreen,
    pub token_list: Vec<String>,
    pub logs: Vec<String>,
    pub wallet_balance: u64,
    pub selected_tab: usize,
    pub wallet_pubkey: Option<Pubkey>,

    // panel / command system
    pub panels: Vec<Panel>,
    pub next_panel_id: usize,
    pub command_input: String,
    pub command_history: Vec<String>,

    // New UI State
    pub token_info: TokenInfo,
    pub recent_trades: Vec<Trade>,
    pub holders: Vec<Holder>,
    pub bottom_tab_index: usize, // 0 = Trades, 1 = Holders, 2 = Orders (maybe later)
    pub swap_amount: String,
    // Layout State
    pub col_constraints: [u16; 3], // Left, Center, Right in %
    pub row_constraints: [u16; 2], // Chart, Trades in %
    pub drag_state: Option<DragState>,
    // Polish
    pub theme: Theme,
    pub candles: Vec<Candle>,
    pub search_input: String,
    // Chart State
    pub chart_x_offset: f64,
    pub chart_y_offset: f64,
    pub last_tick: Instant,
    pub show_search_modal: bool,
    pub search_select_index: usize,
    pub filtered_tokens: Vec<Token>,
    pub all_tokens: Vec<Token>,
    pub ticks_since_candle: usize,
    // Home View Lists
    pub new_tokens: Vec<Token>,
    pub bonding_tokens: Vec<Token>,
    pub migrated_tokens: Vec<Token>,
    pub home_selected_col: usize, // 0=New, 1=Bonding, 2=Migrated
    pub home_selected_row: usize,
    // Wallet Connection State
    pub show_connect_wallet_modal: bool,
    pub wallet_connection_message: String,
    pub wallet_modal_state: WalletModalState,
    pub wallet_private_key_display: Option<String>, // Temporarily shown in modal
    pub image_cache: HashMap<String, DynamicImage>,
    pub image_draw_requests: Vec<ImageDrawRequest>,
    // Jupiter Chart & Holders
    pub chart_interval: ChartInterval,
    pub jupiter_holders: Vec<Holder>,
    pub tx_pool_interval_ms: u64, // Millisecond pooling interval
    pub last_tx_poll: Instant,    // For transaction polling
}

pub struct ImageDrawRequest {
    pub mint: String,
    pub area: Rect,
}

#[derive(Clone, Copy, Debug)]
pub enum DragState {
    ColFirst,  // Dragging barrier between Col 0 and 1
    ColSecond, // Dragging barrier between Col 1 and 2
    RowCenter, // Dragging barrier between Row 0 and 1 (Center Column)
}

impl App {
    pub fn new(player_wallet: Option<Pubkey>, balance: u64) -> Self {
        let all_tokens = vec![];
        let new_tokens = vec![];
        let bonding_tokens = vec![];
        let migrated_tokens = vec![];

        // Load Default Image
        let mut app = Self {
            should_quit: false,
            token_list: Vec::new(),
            logs: vec!["Welcome to Trading Terminal".to_string()],
            wallet_balance: balance,
            selected_tab: 0,
            wallet_pubkey: player_wallet,

            panels: Vec::new(),
            next_panel_id: 1,
            command_input: String::new(),
            command_history: Vec::new(),

            token_info: TokenInfo {
                name: String::new(),
                symbol: String::new(),
                description: String::new(),
                market_cap: 0.0,
                fdv: 0.0,
                liquidity: 0.0,
                holders: 0,
                price: 0.0,
                fees_paid: 0.0,
                org_score: 0.0,
                bonding_curve: 0.0,
                change_5m: None,
                change_1h: None,
                change_6h: 0.0,
                change_24h: 0.0,
                vol_24h: 0.0,
                net_vol_24h: 0.0,
                sell_pressure: 0.0,
                traders_24h: 0,
                net_buyers: None,
                net_buy_trend_24h: 0.0,
                vol_delta_percent: 0.0,
                liquidity_delta_percent: 0.0,
                holders_delta_percent: None,
                mint: String::new(),
                website: String::new(),
                twitter: String::new(),
                telegram: String::new(),
                image_url: String::new(),
                creator_address: String::new(),
                created_at: String::new(),
                updated_at: String::new(),
            },
            recent_trades: vec![],
            holders: vec![],
            bottom_tab_index: 0,
            swap_amount: "0.00".to_string(),
            col_constraints: [25, 50, 25],
            row_constraints: [65, 35],
            drag_state: None,
            theme: Theme::Dark, // default; could change via CLI
            candles: vec![],
            search_input: String::new(),
            chart_x_offset: 0.0,
            chart_y_offset: 0.0,
            last_tick: Instant::now(),
            show_search_modal: false,
            search_select_index: 0,
            filtered_tokens: all_tokens.clone(),
            new_tokens,
            bonding_tokens,
            migrated_tokens,
            all_tokens,
            ticks_since_candle: 0,
            current_screen: CurrentScreen::Home,
            home_selected_col: 0,
            home_selected_row: 0,
            show_connect_wallet_modal: false,
            wallet_connection_message: String::new(),
            wallet_modal_state: WalletModalState::Idle,
            wallet_private_key_display: None,
            image_cache: HashMap::new(),
            image_draw_requests: Vec::new(),
            chart_interval: ChartInterval::OneHour,
            jupiter_holders: vec![],
            tx_pool_interval_ms: 500, // Poll transactions every 500ms
            last_tx_poll: Instant::now(),
        };
        // open default home panel
        app.add_panel("HOME", PanelContent::Home);
        app
    }

    pub fn toggle_theme(&mut self) {
        self.theme = match self.theme {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Bloomberg,
            Theme::Bloomberg => Theme::Light,
        };
    }

    // Panel helpers -------------------------------------------------------
    pub fn add_panel(&mut self, func: &str, content: PanelContent) {
        let mut panel = Panel::new(self.next_panel_id, content);
        panel.title = func.to_string();
        panel.func = func.to_string();
        panel.focused = self.panels.is_empty();
        self.panels.push(panel);
        self.next_panel_id += 1;
    }

    pub fn focus_next_panel(&mut self) {
        if self.panels.is_empty() {
            return;
        }
        let idx = self.panels.iter().position(|p| p.focused).unwrap_or(0);
        self.panels[idx].focused = false;
        let next = (idx + 1) % self.panels.len();
        self.panels[next].focused = true;
    }

    // very simple command execution; actual parser lives in utils/commands.rs
    pub fn execute_command(&mut self, cmd: &str) {
        crate::utils::commands::parse_and_dispatch(self, cmd);
    }

    // handle keyboard events centrally; integrates command bar input
    pub fn handle_key(&mut self, _key: crossterm::event::KeyEvent) -> bool {
        // Disabled command bar interception to fix input bugs.
        // Keys will pass through directly to main.rs.
        false
    }

    pub fn tick(&self) {}

    pub fn update_search_results(&mut self) {
        if self.search_input.is_empty() {
            self.filtered_tokens = self.all_tokens.clone();
        } else {
            let query = self.search_input.to_lowercase();
            self.filtered_tokens = self
                .all_tokens
                .iter()
                .filter(|t| {
                    t.name.to_lowercase().contains(&query)
                        || t.symbol.to_lowercase().contains(&query)
                })
                .cloned()
                .collect();
        }
        if self.search_select_index >= self.filtered_tokens.len() {
            self.search_select_index = 0;
        }
    }

    pub fn select_current_token(&mut self) {
        if let Some(token) = self.filtered_tokens.get(self.search_select_index) {
            self.token_info.name = token.name.clone();
            self.token_info.symbol = token.symbol.clone();
            self.token_info.price = token.price;
            // Update other derived fields roughly
            self.token_info.market_cap = token.price * 1_000_000_000.0;
            self.token_info.mint = token.mint.clone();
            self.show_search_modal = false;
            self.search_input.clear();
            self.update_search_results();
        }
    }

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn add_log(&mut self, message: String) {
        self.logs.push(message);
        if self.logs.len() > 100 {
            self.logs.remove(0);
        }
    }

    pub fn generate_demo_candles(&mut self, count: usize) {
        let mut candles = vec![];
        let base_price = 0.001;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        for i in 0..count {
            let volatility = 0.03;
            let trend = (i as f64) * 0.0001; // Slight uptrend
            let random_factor = (rand::random::<f64>() - 0.5) * volatility;

            let open = base_price + trend + random_factor;
            let close = base_price + trend + (rand::random::<f64>() - 0.5) * volatility;
            let high = open.max(close) * (1.0 + rand::random::<f64>() * 0.01);
            let low = open.min(close) * (1.0 - rand::random::<f64>() * 0.01);

            candles.push(Candle {
                open,
                high,
                low,
                close,
                time: now - ((count - i) as u64 * 3600), // 1 hour apart
            });
        }

        self.candles = candles;
    }

    pub fn generate_demo_transactions(&mut self) {
        let mut trades = vec![];
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        for i in 0..20 {
            let is_buy = i % 2 == 0;
            let price = 0.001 + (rand::random::<f64>() * 0.0005);
            let volume = (rand::random::<f64>() * 10000.0) + 1000.0;
            let now_unix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            // Spread trades over the past 2 hours (120 mins = 7200 seconds)
            let trade_timestamp = now_unix.saturating_sub(i as u64 * 360); // 6 min apart

            trades.push(Trade {
                timestamp: Some(trade_timestamp),
                date: crate::time_utils::format_full_time(trade_timestamp),
                age: crate::time_utils::format_unix_time(trade_timestamp),
                type_: if is_buy {
                    "buy".to_string()
                } else {
                    "sell".to_string()
                },
                price,
                volume,
                sol: volume * 0.00001,
                trader: format!(
                    "{}...{}",
                    &self.token_info.symbol[0..2.min(self.token_info.symbol.len())],
                    i
                ),
            });
        }

        self.recent_trades = trades;
    }

    pub fn generate_demo_holders(&mut self) {
        let mut holders = vec![];
        let total_supply = 1_000_000_000.0;

        // Top holder
        holders.push(Holder {
            address: "DEV_WALLET_1234...".to_string(),
            percent: 15.5,
            sol_balance: 100.0,
            amount: 155000.0,
            is_dev: true,
        });

        // Other holders
        for i in 0..19 {
            let pct = (10.0 - (i as f64 * 0.4)).max(0.1);
            let tokens = (total_supply * pct) / 100.0;

            holders.push(Holder {
                address: format!("Holder_{}...", i + 1),
                percent: pct,
                sol_balance: pct * 10.0,
                amount: tokens,
                is_dev: i < 2,
            });
        }

        self.jupiter_holders = holders;
    }

    pub fn simulate_market_activity(&mut self) {
        // Update much faster for smoother animation (e.g. 50ms)
        if self.last_tick.elapsed() < Duration::from_millis(50) {
            return;
        }
        self.last_tick = Instant::now();

        // 1. Simulate new trade
        let is_buy = rand::random::<bool>(); // 50/50 chance
        let current_price = self.token_info.price;
        // Make it more volatile for visual effect
        let volatility = 0.02;

        let change_percent = (rand::random::<f64>() - 0.5) * volatility;
        let new_price = (current_price * (1.0 + change_percent)).max(0.0000001);

        // Update token info
        self.token_info.price = new_price;
        self.token_info.market_cap = new_price * 1_000_000_000.0 * 0.5; // Rough estimate
        self.token_info.bonding_curve =
            (self.token_info.bonding_curve + (if is_buy { 0.1 } else { -0.05 })).clamp(0.0, 100.0);

        // Add to trade history
        let volume = (rand::random::<f64>() * 10.0 + 0.1).round();
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let trade = Trade {
            timestamp: Some(now_unix),
            date: crate::time_utils::format_full_time(now_unix),
            age: "Just now".to_string(),
            type_: if is_buy {
                "Buy".to_string()
            } else {
                "Sell".to_string()
            },
            price: new_price,
            volume,
            sol: volume * 0.00001,
            trader: "Simulated".to_string(),
        };
        self.recent_trades.insert(0, trade);
        if self.recent_trades.len() > 50 {
            self.recent_trades.pop();
        }

        // Update Charts (Candles)
        // For simplicity, just update the last candle's close price
        // Update candle
        if let Some(last_candle) = self.candles.last_mut() {
            last_candle.close = new_price;
            if new_price > last_candle.high {
                last_candle.high = new_price;
            }
            if new_price < last_candle.low {
                last_candle.low = new_price;
            }
        }

        // Advance to new candle every 20 ticks (~1s)
        self.ticks_since_candle += 1;
        if self.ticks_since_candle > 20 {
            let last_close = self.candles.last().map(|c| c.close).unwrap_or(new_price);
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let new_candle = Candle {
                open: last_close,
                high: last_close,
                low: last_close,
                close: last_close,
                time: now,
            };
            self.candles.push(new_candle);
            self.ticks_since_candle = 0;

            // Auto-scroll to keep latest candle in view
            if self.candles.len() > 50 {
                self.chart_x_offset = (self.candles.len() as f64 - 45.0).max(0.0);
            }
        }
    }

    pub fn open_wallet_connect_modal(&mut self) {
        self.show_connect_wallet_modal = true;
        self.wallet_modal_state = WalletModalState::Idle;
        self.wallet_private_key_display = None;
        self.wallet_connection_message = "Loading wallet...".to_string();
        self.add_log("Wallet connection modal opened".to_string());
    }

    pub fn close_wallet_connect_modal(&mut self) {
        self.show_connect_wallet_modal = false;
        self.wallet_modal_state = WalletModalState::Idle;
        self.wallet_private_key_display = None; // Clear sensitive data
    }

    /// Load existing wallet or create a new one.
    /// Returns the keypair on success so the caller can store it for signing.
    pub fn connect_wallet(&mut self) -> Option<solana_sdk::signer::keypair::Keypair> {
        use solana_sdk::signer::Signer;

        match crate::wallet::load_or_create_wallet() {
            Ok((keypair, is_new)) => {
                let pubkey = keypair.pubkey();
                let path = crate::wallet::wallet_path().display().to_string();
                let pk_display = crate::wallet::export_private_key_bs58(&keypair);

                self.wallet_pubkey = Some(pubkey);

                if is_new {
                    self.wallet_connection_message = format!(
                        "New wallet created!\nPublic Key: {}\nSaved to: {}",
                        pubkey, path
                    );
                    self.wallet_modal_state = WalletModalState::NewWalletCreated {
                        pubkey: pubkey.to_string(),
                        path: path.clone(),
                    };
                    self.wallet_private_key_display = Some(pk_display);
                    self.add_log(format!("New wallet created: {}", pubkey));
                    self.add_log(format!("Wallet saved to: {}", path));
                } else {
                    self.wallet_connection_message =
                        format!("Wallet loaded!\nPublic Key: {}\nFrom: {}", pubkey, path);
                    self.wallet_modal_state = WalletModalState::ExistingWalletLoaded {
                        pubkey: pubkey.to_string(),
                        path: path.clone(),
                    };
                    self.add_log(format!("Wallet loaded: {}", pubkey));
                }

                Some(keypair)
            }
            Err(e) => {
                let msg = format!("Wallet error: {}", e);
                self.wallet_connection_message = msg.clone();
                self.wallet_modal_state = WalletModalState::Error(msg.clone());
                self.add_log(msg);
                None
            }
        }
    }

    pub fn update_tokens_from_api(&mut self, tokens: Vec<Token>) {
        self.all_tokens = tokens.clone();
        self.filtered_tokens = tokens.clone();

        // Distribute tokens into categories based on bonding progress
        self.new_tokens = tokens
            .iter()
            .filter(|t| t.bonding < 30.0)
            .cloned()
            .collect();
        self.bonding_tokens = tokens
            .iter()
            .filter(|t| t.bonding >= 30.0 && t.bonding < 95.0)
            .cloned()
            .collect();
        self.migrated_tokens = tokens
            .iter()
            .filter(|t| t.bonding >= 95.0)
            .cloned()
            .collect();
    }

    pub fn update_selected_token_from_api(&mut self, api_token: crate::network::ApiToken) {
        // Update basic info
        self.token_info.name = api_token.name.clone();
        self.token_info.symbol = api_token.symbol.clone();
        self.token_info.description = api_token.description.unwrap_or_default();
        self.token_info.mint = api_token.mintAddress.clone();
        self.token_info.website = api_token.website.unwrap_or_default();
        self.token_info.twitter = api_token.twitter.unwrap_or_default();
        self.token_info.telegram = api_token.telegram.unwrap_or_default();
        self.token_info.image_url = api_token.imageUrl.unwrap_or_default();
        self.token_info.creator_address = api_token.creatorAddress.clone();
        self.token_info.created_at = api_token.createdAt.clone();
        self.token_info.updated_at = api_token.updatedAt.clone();

        // Update market data
        self.token_info.market_cap = api_token.marketCap.unwrap_or(0.0);
        self.token_info.liquidity = api_token.liquidity.unwrap_or(0.0);
        self.token_info.holders = api_token.holderCount;
        self.token_info.bonding_curve = (api_token.bondingCurveProgress * 100.0).min(100.0);
        self.token_info.vol_24h = api_token.volume.unwrap_or(0.0);

        // Update stats from stats24h
        if let Some(stats24h) = &api_token.stats24h {
            self.token_info.change_24h = stats24h.priceChange;
            self.token_info.vol_24h = stats24h.buyVolume + stats24h.sellVolume;
            self.token_info.traders_24h = stats24h.numTraders;
            self.token_info.net_buyers = Some(stats24h.numNetBuyers);

            // Calculate sell pressure
            let total_volume = stats24h.buyVolume + stats24h.sellVolume;
            if total_volume > 0.0 {
                self.token_info.sell_pressure = (stats24h.sellVolume / total_volume) * 100.0;
            }
        }

        // Update stats from stats6h
        if let Some(stats6h) = &api_token.stats6h {
            self.token_info.change_6h = stats6h.priceChange;
        }

        // Update stats from stats1h
        if let Some(stats1h) = &api_token.stats1h {
            self.token_info.change_1h = Some(stats1h.priceChange);
        }

        // Update stats from stats5m
        if let Some(stats5m) = &api_token.stats5m {
            self.token_info.change_5m = Some(stats5m.priceChange);
        }
    }
}
