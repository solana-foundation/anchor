use anchor_cli::Opts;
use anyhow::Result;
use clap::Parser;

fn main() -> Result<()> {
    anchor_cli::entry(Opts::parse())
}
