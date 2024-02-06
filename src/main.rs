#![recursion_limit = "256"]
use std::process;

#[tokio::main]
async fn main() {
    if let Err(e) = dunspars::cli::run().await {
        eprintln!("{e}");
        process::exit(1);
    };
}
