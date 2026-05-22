mod assets;
mod detector;
mod http;
mod inputs;
mod model_slot;
mod types;

use anyhow::Result;
use clap::{Parser, Subcommand};
use detector::{is_english, Detector};
use http::{build_router, build_service, mount_openapi};
use inputs::{collect_check_items, CheckItem};
use model_slot::DetectorSlot;
use salvo::conn::TcpListener;
use salvo::prelude::*;
use tracing_subscriber::EnvFilter;
use types::{version_info, CheckResult};

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
        Command::Check { inputs } => run_check_batch(&inputs),
        Command::Version => {
            let v = version_info();
            println!("{} {}", v.name, v.version);
            println!("model: {}", v.model);
            Ok(())
        }
        Command::Serve {
            host,
            port,
            prewarm,
            cors,
        } => run_serve(&host, port, prewarm, cors).await,
    }
}

fn run_check_batch(args: &[String]) -> Result<()> {
    let items = collect_check_items(args)?;
    let mut detector: Option<Detector> = None;
    let mut failed = false;

    for item in &items {
        let result = check_one_item(item, &mut detector)?;
        let status = if result.rejected {
            "rejected"
        } else if result.is_injection {
            "injection"
        } else {
            "ok"
        };
        tracing::info!(target = %item.name, %status, "check result");

        if result.rejected || result.is_injection {
            failed = true;
        }
    }

    if failed {
        std::process::exit(1);
    }
    Ok(())
}

fn check_one_item(item: &CheckItem, detector: &mut Option<Detector>) -> Result<CheckResult> {
    if !is_english(&item.text) {
        return Ok(CheckResult::rejected_non_english());
    }
    if detector.is_none() {
        *detector = Some(Detector::new()?);
    }
    detector.as_mut().unwrap().check(&item.text)
}

async fn run_serve(host: &str, port: u16, prewarm: bool, cors: Vec<String>) -> Result<()> {
    let slot = DetectorSlot::new();

    if prewarm {
        slot.start_prewarm();
    } else {
        tracing::info!("model loads on first English /api/check (use --prewarm to load at startup)");
    }

    let router = mount_openapi(build_router(slot));
    let service = build_service(router, cors);

    let addr: &'static str = Box::leak(format!("{host}:{port}").into_boxed_str());
    tracing::info!(%addr, "listening — open http://{addr}/ for API docs");

    let acceptor = TcpListener::new(addr).bind().await;
    Server::new(acceptor).serve(service).await;
    Ok(())
}
