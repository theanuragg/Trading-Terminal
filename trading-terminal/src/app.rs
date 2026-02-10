use solana_sdk::pubkey::Pubkey;
use std::time::{Duration, Instant};

pub struct TokenInfo {
    pub name: String,
    pub symbol: String,
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
    pub sell_pressure: f64, // e.g. 53%
    pub traders_24h: u64,
    pub net_buyers: Option<i64>,
    pub net_buy_trend_24h: f64,
    pub vol_delta_percent: f64, // e.g. +16x (1600%)
    pub liquidity_delta_percent: f64,
    pub holders_delta_percent: Option<f64>,
    pub mint: String,
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
    pub bonding: f64, // 0-100%
    pub mint: String,
}

#[derive(Clone)]
pub struct Holder {
    pub address: String,
    pub balance: f64, // Percentage
    pub value: f64,   // Value in USD
    pub is_dev: bool,
}

pub struct Trade {
    pub time: String,
    pub type_: String, // "Buy" or "Sell"
    pub price: f64,
    pub volume: f64,
    pub maker: String,
}

#[derive(Clone, Copy)]
pub struct Candle {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

pub enum Theme {
    Light,
    Dark,
}

#[derive(Clone, Copy, PartialEq)]
pub enum CurrentScreen {
    Home,
    TokenDetails,
}

pub struct App {
    pub should_quit: bool,
    pub current_screen: CurrentScreen,
    pub token_list: Vec<String>,
    pub logs: Vec<String>,
    pub wallet_balance: u64,
    pub selected_tab: usize,
    pub wallet_pubkey: Option<Pubkey>,
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
}

#[derive(Clone, Copy, Debug)]
pub enum DragState {
    ColFirst,  // Dragging barrier between Col 0 and 1
    ColSecond, // Dragging barrier between Col 1 and 2
    RowCenter, // Dragging barrier between Row 0 and 1 (Center Column)
}

impl App {
    pub fn new(player_wallet: Option<Pubkey>, balance: u64) -> Self {
        let make_token = |name: &str,
                          sym: &str,
                          p: f64,
                          mc: f64,
                          ch: f64,
                          vol: f64,
                          tx: u32,
                          img: &str,
                          bond: f64,
                          mint: &str|
         -> Token {
            Token {
                name: name.to_string(),
                symbol: sym.to_string(),
                price: p,
                market_cap: mc,
                change_24h: ch,
                volume: vol,
                txns: tx,
                image_asc: img.to_string(),
                bonding: bond,
                mint: mint.to_string(),
            }
        };

        let all_tokens = vec![
            make_token(
                "Solana",
                "SOL",
                110.0,
                50_000_000_000.0,
                5.2,
                2_000_000_000.0,
                50000,
                "◎",
                100.0,
                "So11111111111111111111111111111111111111112",
            ),
            make_token(
                "Bitcoin",
                "BTC",
                43000.0,
                800_000_000_000.0,
                2.1,
                30_000_000_000.0,
                10000,
                "₿",
                100.0,
                "3NZ9...mockBTC",
            ),
            make_token(
                "Ethereum",
                "ETH",
                2300.0,
                250_000_000_000.0,
                3.5,
                15_000_000_000.0,
                25000,
                "Ξ",
                100.0,
                "7vfC...mockETH",
            ),
        ];

        let new_tokens = vec![
            make_token(
                "Baby Mobs",
                "MOBS",
                0.00000012,
                5000.0,
                120.5,
                1500.0,
                50,
                "👾",
                15.0,
                "98AF...mockMOBS",
            ),
            make_token(
                "Pepe AI",
                "PEPEAI",
                0.0000045,
                12000.0,
                -5.0,
                3200.0,
                12,
                "🐸",
                22.0,
                "2B3...mockPEPE",
            ),
            make_token(
                "Dog Wif Hat",
                "WIF",
                0.002,
                45000.0,
                35.0,
                15000.0,
                120,
                "🐕",
                45.0,
                "EKp...mockWIF",
            ),
            make_token(
                "Bonk 2.0",
                "BONK2",
                0.0000005,
                2000.0,
                10.0,
                500.0,
                5,
                "🔨",
                5.0,
                "HeL...mockBONK",
            ),
        ];

        let bonding_tokens = vec![
            make_token(
                "Rich Cat",
                "RICH",
                0.005,
                58000.0,
                15.0,
                25000.0,
                340,
                "🐱",
                92.0,
                "8su...mockRICH",
            ),
            make_token(
                "Moon Boi",
                "MOON",
                0.012,
                60500.0,
                2.0,
                12000.0,
                150,
                "🌕",
                98.5,
                "Mo0...mockMOON",
            ),
            make_token(
                "Based God",
                "BASED",
                0.008,
                55000.0,
                45.0,
                45000.0,
                500,
                "🙏",
                88.0,
                "BaS...mockBASED",
            ),
        ];

        let migrated_tokens = vec![
            make_token(
                "Smurf Cat",
                "SMURF",
                0.05,
                1_200_000.0,
                -12.0,
                500_000.0,
                1200,
                "🍄",
                100.0,
                "SmU...mockSMURF",
            ),
            make_token(
                "Punt God",
                "PUNT",
                0.12,
                5_000_000.0,
                120.0,
                2_000_000.0,
                4500,
                "🏈",
                100.0,
                "PuN...mockPUNT",
            ),
            make_token(
                "Retardio",
                "RETARDIO",
                0.02,
                800_000.0,
                5.0,
                150_000.0,
                800,
                "🤪",
                100.0,
                "ReT...mockRETARDIO",
            ),
        ];

        // Load Default Image
        Self {
            should_quit: false,
            token_list: Vec::new(),
            logs: vec!["Welcome to Trading Terminal".to_string()],
            wallet_balance: balance,
            selected_tab: 0,
            wallet_pubkey: player_wallet,
            token_info: TokenInfo {
                name: "RabbitAi".to_string(),
                symbol: "RAN".to_string(),
                market_cap: 3580.0,
                fdv: 3580.0,
                liquidity: 0.05258,
                holders: 3,
                price: 0.0041709,
                fees_paid: 0.452,
                org_score: 0.00,
                bonding_curve: 0.0,
                change_5m: None,
                change_1h: None,
                change_6h: 34.06,
                change_24h: 31.39,
                vol_24h: 2160.0,
                net_vol_24h: 126.0,
                sell_pressure: 53.0,
                traders_24h: 2,
                net_buyers: None,
                net_buy_trend_24h: -1.7705,
                vol_delta_percent: 1600.0, // 16x
                liquidity_delta_percent: -100.0,
                holders_delta_percent: None,
                mint: "RAN...mockHOLE".to_string(),
            },
            recent_trades: vec![
                Trade {
                    time: "23h".to_string(),
                    type_: "Sell".to_string(),
                    price: 0.0041709,
                    volume: 11.86,
                    maker: "HMs...AHF".to_string(),
                },
                Trade {
                    time: "23h".to_string(),
                    type_: "Buy".to_string(),
                    price: 0.0041709,
                    volume: 11.86,
                    maker: "HMs...AHF".to_string(),
                },
                Trade {
                    time: "1d".to_string(),
                    type_: "Sell".to_string(),
                    price: 0.0031463,
                    volume: 1.482,
                    maker: "GwZ...5db".to_string(),
                },
            ],
            holders: vec![
                Holder {
                    address: "8gm5...zMuk".to_string(),
                    balance: 5.59,
                    value: 0.0,
                    is_dev: true,
                },
                Holder {
                    address: "Ha2...XFR".to_string(),
                    balance: 1.2,
                    value: 0.0,
                    is_dev: false,
                },
                Holder {
                    address: "9EB...FyF".to_string(),
                    balance: 0.8,
                    value: 0.0,
                    is_dev: false,
                },
            ],
            bottom_tab_index: 0,
            swap_amount: "0.00".to_string(),
            col_constraints: [20, 60, 20],
            row_constraints: [60, 40],
            drag_state: None,
            theme: Theme::Dark,
            candles: generate_fake_candles(),
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
        }
    }

    pub fn toggle_theme(&mut self) {
        self.theme = match self.theme {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        };
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
        let trade = Trade {
            time: "Just now".to_string(),
            type_: if is_buy {
                "Buy".to_string()
            } else {
                "Sell".to_string()
            },
            price: new_price,
            volume,
            maker: "Simulated".to_string(),
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
            let new_candle = Candle {
                open: last_close,
                high: last_close,
                low: last_close,
                close: last_close,
            };
            self.candles.push(new_candle);
            self.ticks_since_candle = 0;

            // Auto-scroll to keep latest candle in view
            if self.candles.len() > 50 {
                self.chart_x_offset = (self.candles.len() as f64 - 45.0).max(0.0);
            }
        }
    }
}

fn generate_fake_candles() -> Vec<Candle> {
    let mut candles = Vec::new();
    let mut price = 0.0040;
    for _ in 0..50 {
        let change = (rand::random::<f64>() - 0.5) * 0.0002;
        let open = price;
        let close = price + change;
        let high = open.max(close) + (rand::random::<f64>() * 0.0001);
        let low = open.min(close) - (rand::random::<f64>() * 0.0001);
        candles.push(Candle {
            open,
            high,
            low,
            close,
        });
        price = close;
    }
    candles
}
