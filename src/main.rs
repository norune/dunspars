#![recursion_limit = "256"]
use std::process;

#[tokio::main]
async fn main() {
    let run = dunspars::cli::run().await;
    if let Ok(code) = run {
        process::exit(code)
    } else if let Err(e) = run {
        eprintln!("{e}");
        process::exit(1);
    }
}
