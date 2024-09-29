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

use lru::LruCache;
use std::num::NonZeroUsize;

mod model; // Модуль, содержащий определения моделей данных
use model::Order;

mod db; // Модуль для работы с базой данных

mod cli; // Модуль для обработки командной строки
use cli::CliArgs;

// Структура для хранения клиента базы данных и кэша заказов
struct ClientAndCache {
    pub client: Client, // Клиент для подключения к базе данных
    pub orders: LruCache<String, Order>, // Кэш для хранения заказов
}

// Тип для блокировки доступа к ClientAndCache
type ClientAndCacheLock = Arc<RwLock<ClientAndCache>>;

#[tokio::main]
async fn main() {
    dotenv().ok(); // Загружаем переменные окружения из .env файла
    let args = CliArgs::parse(); // Парсим аргументы командной строки

    // Инициализация логирования, если указано в аргументах
    if args.log {
        log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    }

    // Парсим адрес сервера и URL базы данных из аргументов
    let (server_address, database_url) = cli::parse_urls(&args);
    // Запускаем соединение с базой данных и сервер
    start_connection(server_address, database_url, args.cache_size).await;
}

// Функция для создания маршрутизатора с заданным состоянием
fn create_router(state: ClientAndCacheLock) -> Router {
    Router::new()
    .route("/add_order", post(create_order)) // Обработка POST-запроса для добавления заказа
    .route("/get_order/:uid", get(get_order)) // Обработка GET-запроса для получения заказа по UID
    .with_state(state) // Устанавливаем состояние для маршрутизатора
}

// Асинхронная функция для запуска соединения с базой данных и сервера
async fn start_connection(server_address: String, database_url: String, cache_size: usize) {
    info!("Starting server..."); // Логируем запуск сервера

    // Подключаемся к базе данных
    let (client, connection) = tokio_postgres::connect(&database_url, NoTls)
    .await
    .expect("Failed to connect to the database");
    
    // Запускаем задачу для обработки соединения с базой данных
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("Connection error: {}", e); // Логируем ошибку соединения
        }
    });

    // Создаем маршрутизатор с клиентом и кэшем
    let app = create_router(Arc::new(RwLock::new(
        ClientAndCache {
            client,
            orders: LruCache::new(NonZeroUsize::new(cache_size).expect("Incorrect cache size passed")),
        }
    )));

    // Парсим адрес для сервера
    let addr = server_address.parse().expect("Unable to parse address");
    info!("Listening on {}", addr); // Логируем адрес, на котором слушает сервер

    // Запускаем сервер
    axum_server::bind(addr)
    .serve(app.into_make_service())
    .await
    .unwrap();
}


// Асинхронная функция для создания нового заказа
async fn create_order(
    State(state): State<ClientAndCacheLock>, // Извлекаем состояние, которое содержит клиент и кэш заказов
    Json(order): Json<Order> // Извлекаем данные заказа из JSON
) -> impl IntoResponse { // Функция возвращает ответ, который может быть преобразован в HTTP-ответ
    let mut state = state.write().await; // Получаем доступ к состоянию для записи (блокируем для других потоков)

    // Добавляем заказ в базу данных
    let result = db::add_order(&order, &state.client).await;

    match result {
        Ok(_) => { // Если добавление прошло успешно
            // Сохраняем заказ в кэше
            state.orders.put(order.order_uid.clone(), order.clone());
            // Форматируем заказ в красивый JSON
            let pretty_json_order = serde_json::to_string_pretty(&order).unwrap();
            // Возвращаем статус 200 и данные заказа
            (StatusCode::OK, pretty_json_order)
        }
        Err(e) => { // В случае ошибки при добавлении заказа
            // Формируем ответ с ошибкой
            let error_response = json!({
                "success": false,
                "message": e.to_string(), // Сообщение об ошибке
            });
            error!("Failed to add order: {:?}", e); // Логируем ошибку
            // Возвращаем статус 500 и сообщение об ошибке
            (StatusCode::INTERNAL_SERVER_ERROR, error_response.to_string())
        }
    }
}

// Асинхронная функция для получения заказа по его UID
async fn get_order(
    Path(id): Path<String>, // Извлекаем UID заказа из пути запроса
    State(state): State<ClientAndCacheLock>, // Извлекаем состояние, которое содержит клиент и кэш заказов
) -> impl IntoResponse { // Функция возвращает ответ, который может быть преобразован в HTTP-ответ
    let mut state = state.write().await; // Получаем доступ к состоянию для записи (блокируем для других потоков)

    // Проверяем, есть ли заказ в кэше
    match state.orders.get(&id) {
        Some(order) => { // Если заказ найден в кэше
            info!("Order {:?} found in cache", id); // Логируем, что заказ найден в кэше
            // Форматируем заказ в красивый JSON
            let pretty_json_orders = serde_json::to_string_pretty(&order).unwrap();
            // Возвращаем статус 200 и данные заказа
            (StatusCode::OK, pretty_json_orders)
        },
        None => { // Если заказ не найден в кэше
            // Пытаемся получить заказ из базы данных
            let result = db::get_order_by_uid(&id, &state.client).await;
            match result {
                Ok(order) => { // Если заказ успешно получен из базы данных
                    // Сохраняем заказ в кэше
                    state.orders.put(order.order_uid.clone(), order.clone());
                    // Форматируем заказ в красивый JSON
                    let pretty_json_order = serde_json::to_string_pretty(&order).unwrap();
                    // Возвращаем статус 200 и данные заказа
                    (StatusCode::OK, pretty_json_order)
                }
                Err(e) => { // В случае ошибки при получении заказа из базы данных
                    // Формируем ответ с ошибкой
                    let error_response = json!({
                        "success": false,
                        "message": e.to_string(), // Сообщение об ошибке
                    });
                    error!("Failed to get order from db: {:?}", e); // Логируем ошибку
                    // Возвращаем статус 500 и сообщение об ошибке
                    (StatusCode::INTERNAL_SERVER_ERROR, error_response.to_string())
                }
            }
        },
    }
}
