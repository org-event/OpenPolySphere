//! HTTP routes — port of web/routes.py.

use std::sync::Arc;

use crate::downloads;
use crate::engine_service::recreate_engine;
use crate::openrouter::{self, test_model};
use crate::settings::{
    env_deepgram_key, env_openrouter_key, local_translation_status, stt_status, Settings,
    DEEPGRAM_API_URL, USER_AGENT,
};
use crate::voices;
use crate::AppState;
use audio_core::audio;
use audio_core::protocol::Command;
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{
        sse::{Event as SseEvent, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use futures::stream;
use serde::Deserialize;
use serde_json::{json, Map, Value};

#[derive(Deserialize)]
struct CmdBody {
    cmd: String,
}

#[derive(Deserialize)]
struct TranslateBody {
    text: String,
    #[serde(default)]
    from: String,
    #[serde(default = "default_to")]
    to: String,
}

fn default_to() -> String {
    "ru".into()
}

#[derive(Deserialize)]
struct TestKeyBody {
    provider: String,
    key: String,
    #[serde(default)]
    model: Option<String>,
}

#[derive(Deserialize)]
struct VoiceDownloadBody {
    #[serde(default)]
    lang: String,
    #[serde(default)]
    voice: String,
}

#[derive(Deserialize)]
struct MonitorBody {
    #[serde(default)]
    mic: String,
    #[serde(default)]
    call_in: String,
}

#[derive(Deserialize)]
struct PreviewBody {
    #[serde(default)]
    lang: String,
    #[serde(default)]
    voice: String,
}

#[derive(Deserialize, Default)]
struct WhisperDownloadBody {
    #[serde(default)]
    variant: String,
}

#[derive(Deserialize)]
struct ModelsQuery {
    #[serde(default = "default_free")]
    free: String,
    #[serde(default = "default_sort")]
    sort: String,
}

fn default_free() -> String {
    "1".into()
}
fn default_sort() -> String {
    "latency-low-to-high".into()
}

pub fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health))
        .route("/cmd", post(cmd))
        .route("/stream", get(stream))
        .route("/api/settings", get(get_settings).post(post_settings))
        .route("/api/test-key", post(test_key))
        .route("/api/translation-models", get(translation_models))
        .route("/api/translation-status", get(translation_status))
        .route("/api/stt-status", get(stt_status_route))
        .route(
            "/api/polysphere-speech-authorize",
            post(apple_speech_authorize),
        )
        .route("/api/download-whisper-model", post(download_whisper))
        .route("/api/download-translation-models", post(download_translate))
        .route("/api/download-polish-model", post(download_polish))
        .route("/api/voices", get(api_voices))
        .route("/api/devices", get(api_devices))
        .route("/api/tts-preview", post(tts_preview))
        .route("/api/download-voice", post(download_voice))
        .route("/api/engine/restart", post(engine_restart))
        .route("/api/poll-audio", get(poll_audio))
        .route("/api/audio-levels", get(audio_levels))
        .route("/api/monitor-levels", post(monitor_levels))
        .route("/api/translate", post(api_translate))
        .route("/api/calls/new-session", post(new_session))
        .route("/api/calls/end", post(end_call))
        .route("/api/calls", get(list_calls))
        .route("/api/calls/{id}", get(get_call).delete(delete_call))
        .route("/api/calls/{id}/summary", post(call_summary))
}

async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let engine = state.engine.read().await;
    if engine.alive() {
        let pipelines = if engine.pipelines_running() {
            "running"
        } else {
            "idle"
        };
        (
            StatusCode::OK,
            Json(json!({ "engine": "ready", "pipelines": pipelines })),
        )
            .into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "engine": "loading", "pipelines": "unknown" })),
        )
            .into_response()
    }
}

async fn cmd(State(state): State<Arc<AppState>>, Json(body): Json<CmdBody>) -> Json<Value> {
    let settings = state.settings.read().await.clone();
    let engine = state.engine.read().await.clone();
    let status = engine.handle_text(&body.cmd, &settings);
    Json(json!({ "status": status }))
}

async fn stream(
    State(state): State<Arc<AppState>>,
    Query(q): Query<Map<String, Value>>,
) -> Sse<impl stream::Stream<Item = Result<SseEvent, std::convert::Infallible>>> {
    let replay = q.get("replay").and_then(|v| v.as_str()) == Some("1");
    let mut rx = state.sse_tx.subscribe();
    let stream = async_stream::stream! {
        if replay {
            // replay not stored in memory — client reconnects fresh after Start
        }
        loop {
            match rx.recv().await {
                Ok(line) => {
                    yield Ok(SseEvent::default().data(line));
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn get_settings(State(state): State<Arc<AppState>>) -> Json<Value> {
    let settings = state.settings.read().await;
    let mut out = settings.fields.clone();
    out.insert(
        "_deepgram_from_env".into(),
        json!(env_deepgram_key().is_some()),
    );
    out.insert(
        "_openrouter_from_env".into(),
        json!(env_openrouter_key().is_some()),
    );
    out.insert(
        "translation_model".into(),
        json!(settings.translation_model()),
    );
    out.insert(
        "translation_backend".into(),
        json!(settings.translation_backend()),
    );
    out.insert("stt_backend".into(), json!(settings.stt_backend()));
    out.insert("_local_translation".into(), local_translation_status());
    out.insert(
        "_default_stt_device".into(),
        json!(audio_core::stt::local::default_stt_device_name()),
    );
    out.insert("_local_stt".into(), stt_status());
    out.insert("_system_locale".into(), json!(Settings::system_ui_locale()));
    out.insert(
        "_effective_ui_locale".into(),
        json!(settings.effective_ui_locale()),
    );
    out.insert("_host_os".into(), json!(std::env::consts::OS));
    Json(Value::Object(out))
}

async fn post_settings(
    State(state): State<Arc<AppState>>,
    Json(patch): Json<Map<String, Value>>,
) -> Json<Value> {
    {
        let mut settings = state.settings.write().await;
        settings.merge(patch);
        let _ = settings.save();
    }
    Json(json!({ "status": "saved" }))
}

async fn test_key(Json(body): Json<TestKeyBody>) -> Json<Value> {
    if body.key.trim().is_empty() {
        return Json(json!({ "valid": false, "error": "Empty key" }));
    }
    if body.provider == "deepgram" {
        let client = reqwest::Client::new();
        match client
            .get(DEEPGRAM_API_URL)
            .header("Authorization", format!("Token {}", body.key))
            .header("User-Agent", USER_AGENT)
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => Json(json!({ "valid": true })),
            Ok(r) => Json(json!({ "valid": false, "error": r.status().to_string() })),
            Err(e) => Json(json!({ "valid": false, "error": e.to_string() })),
        }
    } else if body.provider == "openrouter" {
        let settings = Settings::load().unwrap_or_default();
        if !matches!(
            settings.translation_backend().as_str(),
            "openrouter" | "cloud" | "llm"
        ) {
            return Json(json!({
                "valid": false,
                "error": "Cloud backend disabled (using local Opus-MT)"
            }));
        }
        let model = body
            .model
            .clone()
            .unwrap_or_else(|| settings.translation_model());
        let result = test_model(&body.key, &model).await;
        if result.get("ok").and_then(|v| v.as_bool()) == Some(true) {
            Json(json!({
                "valid": true,
                "model": model,
                "sample": result.get("sample"),
            }))
        } else if result.get("rate_limited").and_then(|v| v.as_bool()) == Some(true) {
            Json(json!({
                "valid": false,
                "rate_limited": true,
                "key_valid": true,
                "model": model,
                "error": result.get("message"),
            }))
        } else if result.get("key_valid").and_then(|v| v.as_bool()) == Some(false) {
            Json(json!({ "valid": false, "error": "Invalid API key", "model": model }))
        } else {
            Json(json!({
                "valid": false,
                "model": model,
                "error": result.get("message"),
            }))
        }
    } else {
        Json(json!({ "valid": false, "error": "Unknown provider" }))
    }
}

async fn translation_models(Query(q): Query<ModelsQuery>) -> impl IntoResponse {
    let free_only = q.free != "0";
    let sort = q.sort;
    match openrouter::fetch_models(free_only, &sort).await {
        Ok(models) => {
            let current = Settings::load()
                .map(|s| s.translation_model())
                .unwrap_or_default();
            Json(json!({
                "models": models,
                "sort": sort,
                "free_only": free_only,
                "current": current,
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": e.to_string(), "models": [], "current": "" })),
        )
            .into_response(),
    }
}

async fn translation_status() -> Json<Value> {
    Json(local_translation_status())
}

async fn stt_status_route() -> Json<Value> {
    Json(stt_status())
}

async fn apple_speech_authorize() -> Json<Value> {
    match audio_core::stt::apple::apple_speech_request_authorization() {
        Ok(v) => Json(v),
        Err(e) => Json(json!({ "error": e.to_string() })),
    }
}

async fn download_whisper(Json(body): Json<WhisperDownloadBody>) -> impl IntoResponse {
    let variant = if body.variant.trim().is_empty() {
        Settings::load()
            .map(|s| s.whisper_model())
            .unwrap_or_else(|_| "auto".into())
    } else {
        body.variant.trim().to_lowercase()
    };
    match downloads::download_whisper_for(&variant).await {
        Ok(()) => Json(json!({ "ok": true, "status": stt_status() })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn download_translate() -> impl IntoResponse {
    match downloads::download_translation_models().await {
        Ok(()) => Json(json!({ "ok": true, "status": local_translation_status() })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn download_polish() -> impl IntoResponse {
    match downloads::download_polish_model().await {
        Ok(()) => Json(json!({ "ok": true, "status": local_translation_status() })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn api_voices() -> Json<Value> {
    Json(voices::voices_api().await)
}

async fn api_devices() -> Json<Value> {
    match audio::list_devices() {
        Ok((input, output)) => Json(json!({ "input": input, "output": output })),
        Err(e) => Json(json!({ "input": [], "output": [], "error": e.to_string() })),
    }
}

async fn tts_preview(
    State(state): State<Arc<AppState>>,
    Json(body): Json<PreviewBody>,
) -> Json<Value> {
    let mut voice = body.voice;
    if voice.is_empty() {
        let settings = state.settings.read().await;
        voice = if body.lang == "en" {
            settings.outgoing_voice()
        } else {
            settings.incoming_voice()
        };
    }
    let engine = state.engine.read().await.clone();
    engine.command(Command::TtsPreview {
        lang: body.lang,
        voice,
    });
    Json(json!({ "status": "ok:previewing" }))
}

async fn download_voice(Json(body): Json<VoiceDownloadBody>) -> Response {
    match voices::download_voice_stream(&body.lang, &body.voice).await {
        Ok(body) => ([(header::CONTENT_TYPE, "text/event-stream")], body).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("data: {}\n\n", json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn engine_restart(State(state): State<Arc<AppState>>) -> Json<Value> {
    let settings = state.settings.read().await.clone();
    let new_engine = match recreate_engine(
        &settings,
        state.db.clone(),
        state.side.clone(),
        state.sse_tx.clone(),
    ) {
        Ok(e) => e,
        Err(e) => return Json(json!({ "status": format!("error:{e}") })),
    };
    *state.engine.write().await = new_engine;
    Json(json!({ "status": "ok:restarting" }))
}

async fn poll_audio(State(state): State<Arc<AppState>>) -> Json<Value> {
    let q = state.side.tts_queue.lock().unwrap();
    Json(Value::Array(q.clone()))
}

async fn audio_levels(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(state.side.audio_levels.lock().unwrap().clone())
}

async fn monitor_levels(
    State(state): State<Arc<AppState>>,
    Json(body): Json<MonitorBody>,
) -> Json<Value> {
    let settings = state.settings.read().await.clone();
    let payload = json!({ "mic": body.mic, "call_in": body.call_in });
    let engine = state.engine.read().await.clone();
    let status = engine.handle_text(&format!("monitor_levels {}", payload), &settings);
    Json(json!({ "status": status }))
}

async fn api_translate(Json(body): Json<TranslateBody>) -> Json<Value> {
    if body.text.trim().is_empty() {
        return Json(json!({ "translation": "" }));
    }
    let from = if body.from.is_empty() {
        "en"
    } else {
        &body.from
    };
    let to = if body.to.is_empty() { "ru" } else { &body.to };
    match openrouter::translate_text(&body.text, from, to).await {
        Ok(t) => Json(json!({ "translation": t })),
        Err(e) => Json(json!({ "translation": body.text, "error": e.to_string() })),
    }
}

async fn new_session(State(state): State<Arc<AppState>>) -> Json<Value> {
    let settings = state.settings.read().await.clone();
    match state.db.new_session(&settings) {
        Ok(id) => Json(json!({ "ok": true, "call_id": id })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

async fn end_call(State(state): State<Arc<AppState>>) -> Json<Value> {
    let _ = state.db.end_call();
    Json(json!({ "ok": true }))
}

async fn list_calls(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match state.db.list_calls() {
        Ok(rows) => Json(Value::Array(rows)).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn get_call(State(state): State<Arc<AppState>>, Path(id): Path<i64>) -> impl IntoResponse {
    match state.db.get_call(id) {
        Ok(Some(v)) => Json(v).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({ "error": "not found" }))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn delete_call(State(state): State<Arc<AppState>>, Path(id): Path<i64>) -> Json<Value> {
    let _ = state.db.delete_call(id);
    Json(json!({ "ok": true }))
}

async fn call_summary(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    match openrouter::summarize_call(id, &state.db).await {
        Ok(summary) => {
            let _ = state.db.save_summary(id, &summary);
            Json(json!({ "summary": summary })).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
