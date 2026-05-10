use {anchor_cli::Opts, anyhow::Result, clap::Parser};

fn main() -> Result<()> {
    anchor_cli::entry(Opts::parse())
}
