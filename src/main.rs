use std::collections::HashMap;
use anyhow::Result;
use config::Config;
use openssl::ssl::{SslAcceptor, SslMethod, SslFiletype};
use async_std::{
    sync::{Arc, Mutex},
    task::JoinHandle,
    net::TcpListener
};

mod args;
mod config;
mod log;
mod frontend;
//mod poll_waker;

async fn run(args: args::Args) -> Result<()> {
    let cfg = Config::load(&args.config.unwrap_or("server.cfg".to_string()))?;
    let log_handler = log::start(&cfg)?;
    info!("Starting server at {}", cfg.host);

    //let (poll_join, poll_wake) = poll_waker::start()?;

    let mut ssl = SslAcceptor::mozilla_intermediate(SslMethod::tls())?;
    ssl.set_private_key_file(&cfg.key, SslFiletype::PEM)?;
    ssl.set_certificate_chain_file(&cfg.certificate)?;
    ssl.check_private_key()?;

    let listener = TcpListener::bind(cfg.host).await?;

    let info = Arc::new(ServerInfo {
        config: cfg,
        ssl: ssl.build()
    });

    let tasks: Arc<Mutex<(usize, HashMap<usize, JoinHandle<()>>)>> = Arc::new(Mutex::new((0, HashMap::new())));

    let acceptor = frontend::acceptor::Acceptor::new(listener)?;

    while let Some(stream) = acceptor.accept().await {
        match stream {
            Ok(stream) => {
                let mut tasks_grd = tasks.lock().await;
                let mut task_id = tasks_grd.0;
                while tasks_grd.1.contains_key(&task_id) {
                    task_id = task_id.overflowing_add(1).0;
                }
                let info = info.clone();
                let tasks = tasks.clone();
                let join = async_std::task::spawn(async move {
                    frontend::handle_client(info, stream).await;
                    let mut tasks = tasks.lock().await;
                    tasks.1.remove(&task_id);
                });
                tasks_grd.1.insert(task_id, join);
                tasks_grd.0 = task_id;
            }
            Err(err) => warning!("Incomming error {err}")
        }
    }

    info!("Cancelling all tasks");
    for (_, join) in tasks.lock().await.1.drain() {
        join.cancel().await;
    }

    info!("Stopping server gracefully");
    log::stop();
    log_handler.join().unwrap();
    Ok(())
}

pub struct ServerInfo {
    pub config: Config,
    pub ssl: SslAcceptor
}

#[async_std::main]
async fn main() {
    match args::Args::from_cmd() {
        Ok(args) => if let Err(err) = run(args).await {
            eprintln!("runtime error: {err}");
            std::process::exit(1);
        },
        Err(err) => {
            eprintln!("args error: {err}");
            std::process::exit(1);
        }
    }
}