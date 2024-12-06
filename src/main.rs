use clap::Parser;
use cli::Cli;

mod app;
mod cli;
mod ui;

fn main() {
    Cli::parse().run();
}
