use clap::{Parser};

#[derive(Parser)]
#[command(name = "db_client")]
#[command(about = "A simple database client")]
pub struct CliArgs {
    // Адрес сервера
    #[arg(long, env)]
    pub server_host: String,

    // Порт сервера
    #[arg(long, env)]
    pub server_port: u16,

    // Имя пользователя базы данных
    #[arg(short = 'u', long, env)]
    pub db_user: String,

    // Пароль пользователя базы данных
    #[arg(short = 'p', long, env)]
    pub db_password: String,

    // Пароль пользователя базы данных
    #[arg(long, env)]
    pub db_host: String,

    // Пароль пользователя базы данных
    #[arg(long, env)]
    pub db_port: String,

    // Пароль пользователя базы данных
    #[arg(long, env)]
    pub db_name: String,
}
