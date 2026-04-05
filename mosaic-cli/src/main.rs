mod cli;
mod commands;
mod instructions;
mod models;
mod rpc;
mod util;
mod views;

fn main() {
    if let Err(error) = commands::run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}
