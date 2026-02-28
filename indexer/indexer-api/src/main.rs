use anyhow::Result;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use chrono::{DateTime, Utc};
use indexer_core::{
    config::IndexerConfig,
    db::{
        create_pool, get_balances_for_mint, get_portfolio_for_wallet,
        get_bonding_trades_for_mint, get_candles, get_token_transfers_for_mint, run_migrations,
    },
    models::{Balance, BondingCurveTrade, Candle, TokenTransfer},
};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use sqlx::PgPool;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tracing_subscriber::EnvFilter;

async fn health() -> &'static str {
    "ok"
}

async fn metrics_handler(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    // Fetch basic metrics from the database.
    let token_transfers_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM token_transfers")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);

    let bonding_trades_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM bonding_curve_trades")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);

    let last_processed_slot: Option<i64> = sqlx::query_scalar("SELECT slot FROM last_processed_slot WHERE id = 1")
        .fetch_optional(&state.pool)
        .await
        .unwrap_or(None);

    let total_mints: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM mints")
        .fetch_one(&state.pool)
        .await
        .unwrap_or(0);

    let metrics = serde_json::json!({
        "token_transfers_count": token_transfers_count,
        "bonding_trades_count": bonding_trades_count,
        "last_processed_slot": last_processed_slot,
        "total_mints": total_mints,
    });

    Ok(Json(metrics))
}

#[derive(Debug, Deserialize)]
struct TransfersQuery {
    limit: Option<i64>,
    before_slot: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct HoldersQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct BondingTradesQuery {
    limit: Option<i64>,
    before_slot: Option<i64>,
}

async fn token_transfers_handler(
    State(state): State<AppState>,
    Path(mint): Path<String>,
    Query(q): Query<TransfersQuery>,
) -> Result<Json<Vec<TokenTransfer>>, axum::http::StatusCode> {
    let limit = q.limit.unwrap_or(100).clamp(1, 1_000);
    let transfers = get_token_transfers_for_mint(&state.pool, &mint, limit, q.before_slot)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(transfers))
}

async fn token_holders_handler(
    State(state): State<AppState>,
    Path(mint): Path<String>,
    Query(q): Query<HoldersQuery>,
) -> Result<Json<Vec<Balance>>, axum::http::StatusCode> {
    let limit = q.limit.unwrap_or(100).clamp(1, 1_000);
    let offset = q.offset.unwrap_or(0).max(0);

    let holders = get_balances_for_mint(&state.pool, &mint, limit, offset)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(holders))
}

async fn wallet_portfolio_handler(
    State(state): State<AppState>,
    Path(owner): Path<String>,
) -> Result<Json<Vec<Balance>>, axum::http::StatusCode> {
    let portfolio = get_portfolio_for_wallet(&state.pool, &owner)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(portfolio))
}

async fn bonding_trades_handler(
    State(state): State<AppState>,
    Path(mint): Path<String>,
    Query(q): Query<BondingTradesQuery>,
) -> Result<Json<Vec<BondingCurveTrade>>, axum::http::StatusCode> {
    let limit = q.limit.unwrap_or(200).clamp(1, 5_000);

    let trades = get_bonding_trades_for_mint(&state.pool, &mint, limit, q.before_slot)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(trades))
}

#[derive(Clone)]
struct AppState {
    pool: PgPool,
    events_tx: broadcast::Sender<String>,
}

#[derive(Debug, Deserialize)]
struct CandlesQuery {
    timeframe_secs: Option<i32>,
    limit: Option<i64>,
    before: Option<String>,
}

async fn token_candles_handler(
    State(state): State<AppState>,
    Path(mint): Path<String>,
    Query(q): Query<CandlesQuery>,
) -> Result<Json<Vec<Candle>>, axum::http::StatusCode> {
    let tf = q.timeframe_secs.unwrap_or(60).clamp(1, 86_400);
    let limit = q.limit.unwrap_or(500).clamp(1, 5_000);
    let before = if let Some(s) = q.before.as_deref() {
        DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|dt| dt.with_timezone(&Utc))
    } else {
        None
    };

    let candles = get_candles(&state.pool, &mint, tf, limit, before)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(candles))
}

async fn ws_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> axum::response::Response {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: AppState) {
    // Protocol:
    // - Client may send: {"type":"subscribe","topics":["transfers","holders","candles","bonding"],"mint":"..."}
    // - Server pushes: {"topic":"...","mint_pubkey":"...","payload":{...}}
    let mut rx = state.events_tx.subscribe();

    // Default: all events.
    let mut allowed_topics: Option<Vec<String>> = None;
    let mut allowed_mint: Option<String> = None;

    loop {
        tokio::select! {
            recv = socket.recv() => {
                let Some(Ok(msg)) = recv else { break; };
                if let Message::Text(txt) = msg {
                    if let Ok(v) = serde_json::from_str::<JsonValue>(&txt) {
                        if v.get("type").and_then(|x| x.as_str()) == Some("subscribe") {
                            allowed_topics = v.get("topics")
                                .and_then(|t| t.as_array())
                                .map(|arr| arr.iter().filter_map(|x| x.as_str().map(|s| s.to_string())).collect::<Vec<_>>());
                            allowed_mint = v.get("mint").and_then(|m| m.as_str()).map(|s| s.to_string());
                            let _ = socket.send(Message::Text(r#"{"type":"subscribed"}"#.to_string())).await;
                        }
                    }
                }
            }
            evt = rx.recv() => {
                let Ok(payload) = evt else { continue; };
                // Best-effort filtering without fully parsing each payload:
                // We parse small JSON to check topic/mint keys.
                if let Ok(v) = serde_json::from_str::<JsonValue>(&payload) {
                    let topic = v.get("topic").and_then(|x| x.as_str()).unwrap_or("");
                    let mint = v.get("mint_pubkey").and_then(|x| x.as_str());

                    if let Some(ref topics) = allowed_topics {
                        if !topics.iter().any(|t| t == topic) {
                            continue;
                        }
                    }
                    if let Some(ref m) = allowed_mint {
                        if mint != Some(m.as_str()) {
                            continue;
                        }
                    }
                }

                if socket.send(Message::Text(payload)).await.is_err() {
                    break;
                }
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = IndexerConfig::from_env()?;

    let pool = create_pool(&config.db.url, config.db.max_connections).await?;
    run_migrations(&pool).await?;

    let (events_tx, _events_rx) = broadcast::channel::<String>(10_000);

    // Background: LISTEN/NOTIFY → broadcast for websocket clients.
    {
        let db_url = config.db.url.clone();
        let events_tx = events_tx.clone();
        tokio::spawn(async move {
            let mut listener = match sqlx::postgres::PgListener::connect(&db_url).await {
                Ok(l) => l,
                Err(e) => {
                    tracing::error!("PgListener connect failed: {e:?}");
                    return;
                }
            };

            if let Err(e) = listener.listen("indexer_events").await {
                tracing::error!("PgListener listen failed: {e:?}");
                return;
            }

            loop {
                match listener.recv().await {
                    Ok(n) => {
                        let _ = events_tx.send(n.payload().to_string());
                    }
                    Err(e) => {
                        tracing::error!("PgListener recv failed: {e:?}");
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    }
                }
            }
        });
    }

    let state = AppState { pool, events_tx };

    let app = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics_handler))
        .route(
            "/token/:mint/transfers",
            get(token_transfers_handler),
        )
        .route(
            "/token/:mint/holders",
            get(token_holders_handler),
        )
        .route(
            "/wallet/:owner/portfolio",
            get(wallet_portfolio_handler),
        )
        .route(
            "/token/:mint/bonding_trades",
            get(bonding_trades_handler),
        )
        .route(
            "/token/:mint/candles",
            get(token_candles_handler),
        )
        .route("/ws", get(ws_handler))
        .with_state(state);

    let addr: SocketAddr = config.api.bind_addr.parse()?;
    tracing::info!("Starting API server on {}", addr);

    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

