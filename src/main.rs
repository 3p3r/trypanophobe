use anyhow::Result;
use clap::{Parser, Subcommand};
use salvo::conn::TcpListener;
use salvo::prelude::*;
use tracing_subscriber::EnvFilter;
use trypanophobe::{app_service, run_check_batch, version_info, CheckBatchOutcome, DetectorSlot};

#[derive(Parser)]
#[command(name = "trypanophobe", about = "Prompt injection detector")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check prompt(s) for injection (exit 0 = ok, 1 = failure)
    Check {
        /// File path, directory of `.prompt` files, and/or prompt text (batch)
        #[arg(value_name = "PATH_OR_TEXT", num_args = 1..)]
        inputs: Vec<String>,
    },
    /// Print version and model id
    Version,
    /// Start the REST API server
    Serve {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value = "9876")]
        port: u16,
        /// Start loading model in background at startup (/api/check blocks until ready)
        #[arg(long)]
        prewarm: bool,
        /// Allowed CORS origin (repeatable; default "*")
        #[arg(long = "cors", default_value = "*")]
        cors: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_target(false)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    match cli.command {
        Command::Check { inputs } => {
            if run_check_batch(&inputs)? == CheckBatchOutcome::Failed {
                std::process::exit(1);
            }
        }
        Command::Version => {
            let v = version_info();
            println!("{} {}", v.name, v.version);
            println!("model: {}", v.model);
        }
        Command::Serve {
            host,
            port,
            prewarm,
            cors,
        } => run_serve(&host, port, prewarm, cors).await?,
    }
    Ok(())
}

async fn run_serve(host: &str, port: u16, prewarm: bool, cors: Vec<String>) -> Result<()> {
    let slot = DetectorSlot::new();

    if prewarm {
        slot.start_prewarm();
    } else {
        tracing::info!("model loads on first English /api/check (use --prewarm to load at startup)");
    }

    let service = app_service(slot, cors);
    let addr: &'static str = Box::leak(format!("{host}:{port}").into_boxed_str());
    tracing::info!(%addr, "listening — open http://{addr}/ for API docs");

    let acceptor = TcpListener::new(addr).bind().await;
    Server::new(acceptor).serve(service).await;
    Ok(())
}
