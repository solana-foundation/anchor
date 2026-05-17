use {anchor_cli::Opts, anyhow::Result, clap::Parser};

fn main() -> Result<()> {
    if is_version_request() {
        print!("{}", anchor_cli::support_version_report());
        return Ok(());
    }

    anchor_cli::entry(Opts::parse())
}

fn is_version_request() -> bool {
    let mut args = std::env::args_os().skip(1);
    let Some(arg) = args.next() else {
        return false;
    };

    (arg == "--version" || arg == "-V") && args.next().is_none()
}
