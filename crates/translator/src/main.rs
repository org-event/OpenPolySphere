mod db;
mod downloads;
mod engine_service;
mod events;
mod openrouter;
mod paths;
mod port;
mod routes;
mod settings;
mod setup;
mod voices;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::Router;
use clap::{Parser, Subcommand};
use log::info;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};

use crate::db::Db;
use crate::engine_service::{recreate_engine, EngineService};
use crate::events::SideState;
use crate::paths::{base_dir, models_dir, web_static_dir};
use crate::settings::{apply_env, Settings};

#[derive(Parser)]
#[command(name = "translator", about = "Call Translator — all-Rust server")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Download models and check environment (replaces setup.sh)
    Setup,
    /// Run HTTP server on :5050 (default)
    Serve,
}

pub struct AppState {
    pub engine: Arc<RwLock<Arc<EngineService>>>,
    pub settings: Arc<RwLock<Settings>>,
    pub db: Arc<Db>,
    pub side: Arc<SideState>,
    pub sse_tx: broadcast::Sender<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::optional().ok();

    let cli = Cli::parse();
    match cli.command.unwrap_or(Commands::Serve) {
        Commands::Setup => setup::run().await,
        Commands::Serve => serve().await,
    }
}

async fn serve() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    audio_core::init_ort();

    let db = Arc::new(Db::open()?);
    let settings = Settings::load()?;

    let stt = settings.stt_backend();
    if matches!(stt.as_str(), "apple" | "system" | "macos") {
        apply_env(&settings, &models_dir());
        info!(
            "PolySphere Speech STT configured — requesting speech recognition permission if needed"
        );
        if let Err(e) = audio_core::stt::apple::apple_speech_ensure_authorized() {
            log::warn!("PolySphere Speech authorization: {e:#}");
        }
    }

    let side = Arc::new(SideState::default());
    let (sse_tx, _) = broadcast::channel(512);
    let engine = recreate_engine(&settings, db.clone(), side.clone(), sse_tx.clone())?;

    let state = Arc::new(AppState {
        engine: Arc::new(RwLock::new(engine)),
        settings: Arc::new(RwLock::new(settings)),
        db,
        side,
        sse_tx,
    });

    let static_dir = web_static_dir();
    let index = static_dir.join("index.html");
    let history = static_dir.join("history.html");

    let app = Router::new()
        .merge(routes::api_routes())
        .nest_service("/static", ServeDir::new(static_dir.clone()))
        .route_service("/", ServeFile::new(index))
        .route_service("/history", ServeFile::new(history))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 5050));
    port::reclaim(5050);
    info!(
        "Call Translator listening on http://{addr}  (root: {})",
        base_dir().display()
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

// Minimal .env loader without extra dep — read .env from base dir if present
mod dotenvy {
    use crate::paths::user_data_dir;
    use std::fs;

    pub fn optional() -> Result<(), ()> {
        let path = user_data_dir().join(".env");
        let Ok(raw) = fs::read_to_string(&path) else {
            return Err(());
        };
        let mut groq_value: Option<String> = None;
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let key = k.trim();
                let value = v.trim().trim_matches('"').trim_matches('\'');
                if key == "GROQ_API_KEY" && !value.is_empty() {
                    groq_value = Some(value.to_string());
                }
                if std::env::var(key).is_err() {
                    std::env::set_var(key, value);
                }
            }
        }
        if let Some(groq) = groq_value {
            if std::env::var("OPENROUTER_API_KEY")
                .map(|s| s.trim().is_empty())
                .unwrap_or(true)
            {
                std::env::set_var("OPENROUTER_API_KEY", &groq);
            }
        }
        Ok(())
    }
}
