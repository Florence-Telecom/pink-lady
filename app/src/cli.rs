use clap::Parser;

use std::net::SocketAddr;

#[derive(Parser, Debug)]
#[command(name = "Pink Lady")]
#[command(author = "Antoine Charbonneau <antoine@florencetelecom.com>")]
#[command(about = "Pink Lady: Un système de monitoring Prometheus pour les commandes simples")]
#[command(version, long_about = None)]
pub struct Args {
    /// Path to .env config file.
    #[arg(short, default_value_t = String::from("./.env"))]
    pub env_file: String,

    /// Socket address to bind the application to.
    #[arg(short, long, default_value_t = String::from("0.0.0.0:9101"))]
    pub bind: String,
}

impl Args {
    pub fn get_params() -> Args {
        return Args::parse();
    }

    pub fn get_bind(&self) -> SocketAddr {
        return self
            .bind
            .parse()
            .expect("Unable to parse the bind socket address");
    }
}
