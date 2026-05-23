use crate::app::App;
use crate::theme;
use ratatui::{
    layout::{Constraint, Rect},
    style::Style,
    widgets::{Cell, Row, Table},
    Frame,
};

#[derive(Clone)]
pub struct WatchlistPanel {
    pub items: Vec<WatchItem>,
}

#[derive(Clone)]
pub struct WatchItem {
    pub logo: String,
    pub symbol: String,
    pub chain: String,
    pub price: f64,
    pub p5: f64,
    pub p15: f64,
    pub p60: f64,
    pub p1440: f64,
    pub market_cap: String,
    pub volume: String,
    pub liquidity: String,
    pub age: String,
}

impl WatchlistPanel {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn update(&mut self, data: Vec<WatchItem>) {
        self.items = data;
    }

    pub fn render(&self, f: &mut Frame, area: Rect, _app: &App) {
        let header_style = Style::default()
            .fg(theme::BORDER)
            .add_modifier(ratatui::style::Modifier::BOLD | ratatui::style::Modifier::UNDERLINED);
        let pct_style = |val: f64| {
            if val >= 0.0 {
                Style::default().fg(theme::POSITIVE)
            } else {
                Style::default().fg(theme::NEGATIVE)
            }
        };

        let rows: Vec<Row> = self
            .items
            .iter()
            .map(|itm| {
                Row::new(vec![
                    Cell::from(itm.symbol.clone()),
                    Cell::from(itm.chain.clone()),
                    Cell::from(format!("{:.4}", itm.price)),
                    Cell::from(format!("{:.1}%", itm.p5)).style(pct_style(itm.p5)),
                    Cell::from(format!("{:.1}%", itm.p15)).style(pct_style(itm.p15)),
                    Cell::from(format!("{:.1}%", itm.p60)).style(pct_style(itm.p60)),
                    Cell::from(format!("{:.1}%", itm.p1440)).style(pct_style(itm.p1440)),
                    Cell::from(itm.market_cap.clone()),
                    Cell::from(itm.volume.clone()),
                    Cell::from(itm.liquidity.clone()),
                    Cell::from(itm.age.clone()),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Length(8),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Length(4),
        ];

        let table = Table::new(rows, widths)
            .header(
                Row::new(vec![
                    Cell::from("TICKER"),
                    Cell::from("CHAIN"),
                    Cell::from("PRICE"),
                    Cell::from("5m"),
                    Cell::from("15m"),
                    Cell::from("1h"),
                    Cell::from("24h"),
                    Cell::from("MC"),
                    Cell::from("VOL"),
                    Cell::from("LIQ"),
                    Cell::from("AGE"),
                ])
                .style(header_style),
            )
            .column_spacing(1)
            .style(theme::default_style());

        f.render_widget(table, area);
    }
}
