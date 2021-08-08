use anchor_cli::Opts;
use anyhow::Result;
use clap::Clap;

fn main() -> Result<()> {
    anchor_cli::entry(Opts::parse())
}
