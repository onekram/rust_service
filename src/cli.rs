use clap::{Parser};

#[derive(Parser)]
#[command(name = "db_client")]
#[command(about = "A simple database client")]
pub struct CliArgs {
    #[arg(long, env)]
    pub server_host: String,

    #[arg(long, env)]
    pub server_port: u16,

    #[arg(short = 'u', long, env)]
    pub db_user: String,

    #[arg(short = 'p', long, env)]
    pub db_password: String,

    #[arg(long, env)]
    pub db_host: String,

    #[arg(long, env)]
    pub db_port: String,

    #[arg(long, env)]
    pub db_name: String,
}
