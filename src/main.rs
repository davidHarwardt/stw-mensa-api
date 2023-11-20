use axum::{Router, extract::{FromRef, Query, State}, routing::get, Json};
use chrono::{NaiveDate, Utc};
use menu::MensaMenu;
use reqwest::StatusCode;
use serde::{Serialize, Deserialize};
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};

mod config;
mod menu;

#[derive(Clone, FromRef)]
struct AppState {
    db: SqlitePool,
    req_client: reqwest::Client,
}

fn date_today() -> NaiveDate { Utc::now().date_naive() }
#[derive(Debug, Serialize, Deserialize)]
struct RetreiveQuery {
    #[serde(default = "date_today")]
    date: NaiveDate,
    #[serde(default)]
    mensa: Option<String>,
}

async fn retrieve_menu(
    Query(query): Query<RetreiveQuery>,
    State(req_client): State<reqwest::Client>,
) -> Result<Json<MensaMenu>, (StatusCode, String)> {
    tracing::info!("loading menu for {}", query.date);
    MensaMenu::load(
        &req_client,
        query.mensa.unwrap_or_else(|| format!("322")),
        query.date,
    ).await.map(Json).map_err(|err| (StatusCode::NOT_FOUND, format!("{err}")))
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let db = SqlitePoolOptions::new()
        .max_connections(5)
    .connect("sqlite::memory:").await.expect("could not connect to db");

    let req_client = reqwest::Client::new();

    let state = AppState { db, req_client };

    let app = Router::new()
        .route("/menu", get(retrieve_menu))
    .with_state(state);

    let addr = ([0, 0, 0, 0], 3000).into();
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
    .unwrap()
}

