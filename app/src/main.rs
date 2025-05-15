mod cli;
mod scripts;

use env_logger;
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use prometheus_client::{encoding::text::encode, registry::Registry};
use std::{
    future::Future,
    net::SocketAddr,
    pin::Pin,
    sync::{Arc, RwLock},
    fs::File,
    io::{self, Write},
};
use tokio::signal::unix::{signal, SignalKind};

#[tokio::main]
async fn main() {
    let env_file = cli::Args::get_params().env_file;
    dotenv::from_path(env_file).expect("Couldn't load .env file from -e parameter value.");
    env_logger::init();

    std::fs::create_dir_all("/run/pink-lady/").unwrap();

    let pid = std::process::id();
    log::debug!("Process ID: {}", pid);
    let name: String = std::env::var("PL_NAME").unwrap();
    let mut fh = File::create(format!("/run/pink-lady/{}.pid", name)).expect("Couldn't create PID file.");
    let _ = fh.write(format!("{}\n", pid).as_bytes());

    let registry = scripts::get_registry();
    let metrics_addr = cli::Args::get_params().get_bind();
    let registry = Arc::new(RwLock::new(registry));
    let reload_registry = registry.clone();

    tokio::spawn(async move { start_metrics_server(metrics_addr, registry).await });

    let mut sighup_stream = signal(SignalKind::hangup()).unwrap();
    log::debug!("Awaiting reload signal");

    loop {
        sighup_stream.recv().await;
        log::info!("Reload started");
        let new_registry = reload_settings();
        let mut reg_writer = reload_registry.write().unwrap();
        *reg_writer = new_registry;
        log::info!("Reload completed!");
    }
}

fn reload_settings() -> Registry {
    dotenv::from_path(cli::Args::get_params().env_file).unwrap();

    scripts::get_registry()
}

/// Start a HTTP server to report metrics.
pub async fn start_metrics_server(metrics_addr: SocketAddr, registry: Arc<RwLock<Registry>>) {
    let mut shutdown_stream = signal(SignalKind::terminate()).unwrap();

    log::info!("Starting metrics server on {metrics_addr}");

    Server::bind(&metrics_addr)
        .serve(make_service_fn(
            move |_conn: &hyper::server::conn::AddrStream| {
                let registry = registry.clone();
                log::info!("Serving metrics to: {}", _conn.remote_addr());
                async move {
                    let handler = make_handler(registry);
                    Ok::<_, io::Error>(service_fn(handler))
                }
            },
        ))
        .with_graceful_shutdown(async move {
            shutdown_stream.recv().await;
        })
        .await
        .unwrap();
}

/// This function returns a HTTP handler (i.e. another function)
pub fn make_handler(
    registry: Arc<RwLock<Registry>>,
) -> impl Fn(Request<Body>) -> Pin<Box<dyn Future<Output = io::Result<Response<Body>>> + Send>> {
    // This closure accepts a request and responds with the OpenMetrics encoding of our metrics.
    move |req: Request<Body>| {
        if req.uri() == "/metrics" {
            let reg = registry.clone();
            Box::pin(async move {
                let mut buf = String::new();
                encode(&mut buf, &reg.read().unwrap())
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
                    .map(|_| {
                        let body = Body::from(buf);
                        Response::builder()
                            .header(
                                hyper::header::CONTENT_TYPE,
                                "application/openmetrics-text; version=1.0.0; charset=utf-8",
                                )
                            .body(body)
                            .unwrap()
                    })
            })
        } else {
            Box::pin(async {
                Ok(
                    Response::builder()
                    .status(hyper::http::StatusCode::NOT_FOUND)
                    .body(())
                    .unwrap()
                  )
            })
        }
    }
}
