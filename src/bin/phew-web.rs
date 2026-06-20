use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::Context;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use clap::Parser;
use phew::config::{Config, IndentStyle};
use phew::formatter::Formatter;
use serde::{Deserialize, Serialize};
use tower_http::services::{ServeDir, ServeFile};

const DEFAULT_ADDR: &str = "127.0.0.1:3010";
const DEFAULT_WEB_DIR: &str = "web";
const INDEX_FILE: &str = "index.html";
const HERO_ROUTE: &str = "/hero.png";
const HERO_FILE: &str = "docs/hero.png";
const FORMAT_ROUTE: &str = "/api/format";
const MAX_SOURCE_BYTES: usize = 2_000_000;
const MIN_INDENT_SIZE: usize = 1;
const MAX_INDENT_SIZE: usize = 12;
const MIN_LINE_LENGTH: usize = 40;
const MAX_LINE_LENGTH: usize = 240;
const MILLIS_PER_SECOND: f64 = 1_000.0;

#[derive(Debug, Parser)]
#[command(name = "phew-web")]
#[command(about = "Веб-интерфейс phew")]
struct Cli {
    #[arg(long, default_value = DEFAULT_ADDR, help = "Адрес HTTP-сервера")]
    addr: SocketAddr,

    #[arg(long, value_name = "DIR", default_value = DEFAULT_WEB_DIR, help = "Каталог веб-интерфейса")]
    web_dir: PathBuf,

    #[arg(long, value_name = "PATH", help = "Путь к .phew.toml")]
    config: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct AppState {
    base_config: Config,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FormatRequest {
    source: String,
    #[serde(default)]
    options: FormatOptions,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct FormatOptions {
    indent_style: Option<IndentStyle>,
    indent_size: Option<usize>,
    max_line_length: Option<usize>,
}

#[derive(Debug, Serialize)]
struct FormatResponse {
    formatted: String,
    changed: bool,
    duration_ms: f64,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: String,
}

#[derive(Debug)]
struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(ErrorBody { error: self.message })).into_response()
    }
}

fn resolve_config(cli: &Cli) -> anyhow::Result<Config> {
    match &cli.config {
        Some(path) => Config::load(path).map_err(Into::into),
        None => {
            let cwd = std::env::current_dir()?;
            match Config::discover(&cwd) {
                Some(path) => Config::load(&path).map_err(Into::into),
                None => Ok(Config::default()),
            }
        }
    }
}

fn validate_web_dir(path: &Path) -> anyhow::Result<()> {
    let index_path = path.join(INDEX_FILE);
    if index_path.is_file() {
        return Ok(());
    }
    anyhow::bail!("не найден {}", index_path.display())
}

fn apply_options(base: &Config, options: FormatOptions) -> Result<Config, ApiError> {
    let mut config = base.clone();
    if let Some(indent_style) = options.indent_style {
        config.indent_style = indent_style;
    }
    if let Some(indent_size) = options.indent_size {
        validate_range(indent_size, MIN_INDENT_SIZE, MAX_INDENT_SIZE, "indent_size")?;
        config.indent_size = indent_size;
    }
    if let Some(max_line_length) = options.max_line_length {
        validate_range(max_line_length, MIN_LINE_LENGTH, MAX_LINE_LENGTH, "max_line_length")?;
        config.max_line_length = max_line_length;
    }
    Ok(config)
}

fn validate_range(value: usize, min: usize, max: usize, name: &str) -> Result<(), ApiError> {
    if (min..=max).contains(&value) {
        return Ok(());
    }
    Err(ApiError::bad_request(format!(
        "{name} должен быть в диапазоне {min}..={max}"
    )))
}

async fn format_handler(
    State(state): State<AppState>,
    Json(payload): Json<FormatRequest>,
) -> Result<Json<FormatResponse>, ApiError> {
    if payload.source.len() > MAX_SOURCE_BYTES {
        return Err(ApiError::bad_request("исходник слишком большой"));
    }

    let config = apply_options(&state.base_config, payload.options)?;
    let formatter = Formatter::new(&config);
    let started = Instant::now();
    let formatted = formatter.format_source(&payload.source);
    let duration_ms = started.elapsed().as_secs_f64() * MILLIS_PER_SECOND;
    let changed = formatted != payload.source;

    Ok(Json(FormatResponse {
        formatted,
        changed,
        duration_ms,
    }))
}

fn app(cli: &Cli, state: AppState) -> Router {
    let index_path = cli.web_dir.join(INDEX_FILE);
    let static_files = ServeDir::new(&cli.web_dir).not_found_service(ServeFile::new(index_path));

    Router::new()
        .route(FORMAT_ROUTE, post(format_handler))
        .route_service(HERO_ROUTE, ServeFile::new(HERO_FILE))
        .fallback_service(static_files)
        .with_state(state)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    validate_web_dir(&cli.web_dir)?;
    let base_config = resolve_config(&cli)?;
    let listener = tokio::net::TcpListener::bind(cli.addr)
        .await
        .with_context(|| format!("не удалось запустить сервер на {}", cli.addr))?;
    let addr = listener.local_addr()?;

    println!("Веб-интерфейс phew: http://{addr}");

    axum::serve(listener, app(&cli, AppState { base_config })).await?;
    Ok(())
}
