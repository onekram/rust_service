use axum::{
    extract::{Json, Path, State},
    response::IntoResponse,
    routing::{get, post},
    http::StatusCode,
    Router,
};
use clap::Parser;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_postgres::{NoTls, Client};

use dotenv::dotenv;

use log::{info, error};

use serde_json::json;

use std::collections::HashMap;

mod model;
use model::Order;

mod db;

mod cli;
use cli::CliArgs;
struct ClientAndCache {
    pub client: Client,
    pub orders: HashMap<String, Order>,
}
type ClientAndCacheLock = Arc<RwLock<ClientAndCache>>;

#[tokio::main]
async fn main() {
    dotenv().ok();
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();


    let (server_address, database_url) = CliArgs::parse_urls();

    info!("Starting server...");

    let (client, connection) = tokio_postgres::connect(&database_url, NoTls)
        .await
        .expect("Failed to connect to the database");
    
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e);
        }
    });

    let app = Router::new()
    .route("/add_order", post(create_order))
    .route("/get_order/:uid", get(get_order))
    .with_state(Arc::new(RwLock::new(
        ClientAndCache {
            client,
            orders: HashMap::new(),
        }
    )));

    

    let addr = server_address.parse().expect("Unable to parse address");
    info!("Listening on {}", addr);

    axum_server::bind(addr)
    .serve(app.into_make_service())
    .await
    .unwrap();
}

async fn create_order(
    State(state): State<ClientAndCacheLock>,
    Json(order): Json<Order> 
) -> impl IntoResponse {
    let mut state = state.write().await;

    let result = db::add_order(&order, &state.client).await;

    match result {
        Ok(_) => {
            info!("Order added successfully: {:?}", order);
            state.orders.insert(order.order_uid.clone(), order.clone());
            let pretty_json_order = serde_json::to_string_pretty(&order).unwrap();
            (StatusCode::OK, pretty_json_order)
        }
        Err(e) => {
            let error_response = json!({
                "success": false,
                "message": e.to_string(),
            });
            error!("Failed to add order: {:?}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, error_response.to_string())
        }
    }
}

async fn get_order(
    Path(id): Path<String>,
    State(state): State<ClientAndCacheLock>,
) -> impl IntoResponse {
    let mut state = state.write().await;
    

    match state.orders.get(&id) {
        Some(order) => {
            info!("Order {:?} finded in cache", id);
            let pretty_json_orders = serde_json::to_string_pretty(
                &order
            ).unwrap();
            (StatusCode::OK, pretty_json_orders)
        },
        None => {
            let result = db::get_order_by_uid(id, &state.client).await;
            match result {
                Ok(order) => {
                    info!("Order get from db successfully: {:?}", order);
                    state.orders.insert(order.order_uid.clone(), order.clone());
                    let pretty_json_order = serde_json::to_string_pretty(&order).unwrap();
                    (StatusCode::OK, pretty_json_order)
                }
                Err(e) => {
                    let error_response = json!({
                        "success": false,
                        "message": e.to_string(),
                    });
                    error!("Failed to get order from db: {:?}", e);
                    (StatusCode::INTERNAL_SERVER_ERROR, error_response.to_string())
                }
            }
        },
    }
}
