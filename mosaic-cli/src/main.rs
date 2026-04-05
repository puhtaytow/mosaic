use anyhow::Result;

mod cli;
mod commands;
mod instructions;
mod models;
mod rpc;
mod util;
mod views;

fn main() -> Result<()> {
    commands::run()
}
