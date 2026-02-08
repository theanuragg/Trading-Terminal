use solana_sdk::pubkey::Pubkey;

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

pub struct App {
    pub should_quit: bool,
    pub token_list: Vec<String>,
    pub logs: Vec<String>,
    pub wallet_balance: u64,
    pub selected_tab: usize,
    pub wallet_pubkey: Option<Pubkey>,
    // New UI State
    pub token_info: TokenInfo,
    pub recent_trades: Vec<Trade>,
    pub swap_amount: String,
    // Polish
    pub theme: Theme,
    pub candles: Vec<Candle>,
    pub search_input: String,
}

impl App {
    pub fn new(player_wallet: Option<Pubkey>, balance: u64) -> Self {
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
            swap_amount: "0.00".to_string(),
            theme: Theme::Dark,
            candles: generate_fake_candles(),
            search_input: String::new(),
        }
    }

    pub fn toggle_theme(&mut self) {
        self.theme = match self.theme {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        };
    }

    pub fn tick(&self) {}

    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    pub fn add_log(&mut self, message: String) {
        self.logs.push(message);
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
