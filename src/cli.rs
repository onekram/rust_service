// Импортируем библиотеку clap для парсинга аргументов командной строки
use clap::Parser;

// Определяем структуру для аргументов командной строки
#[derive(Parser)]
#[command(name = "db_client")]
#[command(about = "A simple database client")]
pub struct CliArgs {
    #[arg(long, env)] // Хост сервера
    pub server_host: String,

    #[arg(long, env)] // Порт сервера
    pub server_port: u16,

    #[arg(short = 'u', long, env, help = "Database username")] // Имя пользователя базы данных
    pub db_user: String,

    #[arg(short = 'p', long, env, help = "Database user password")] // Пароль пользователя базы данных
    pub db_password: String,

    #[arg(long, env)] // Хост базы данных
    pub db_host: String,

    #[arg(long, env)] // Порт базы данных
    pub db_port: String,

    #[arg(long, env)] // Имя базы данных
    pub db_name: String,

    #[arg(short = 'l', long, help = "Enable logging")] // Включение логирования
    pub log: bool,

    #[arg(short = 'c', long, default_value_t = 100, help = "LRU cache size")] // Размер кэша LRU
    pub cache_size: usize,
}

// Функция для формирования адреса сервера и URL базы данных
pub fn parse_urls(args: &CliArgs) -> (String, String) {
    let server_address = format!("{}:{}", args.server_host, args.server_port); // Адрес сервера
    let database_url = format!( // URL для подключения к базе данных
        "postgres://{}:{}@{}:{}/{}",
        args.db_user, args.db_password, args.db_host, args.db_port, args.db_name
    );
    (server_address, database_url) // Возвращаем кортеж
}
