use anyhow::Result;
use clap::Clap;

#[derive(Debug, Clap)]
pub struct Opts {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Debug, Clap)]
pub enum Command {
    /// Outputs an interface definition file.
    Idl {
        /// Path to the program's interface definition.
        #[clap(short, long)]
        file: String,
        /// Output file for the idl (stdout if not specified).
        #[clap(short, long)]
        out: Option<String>,
    },
    /// Generates a client module.
    Gen {
        /// Path to the program's interface definition.
        #[clap(short, long, required_unless_present("idl"))]
        file: Option<String>,
        /// Output file (stdout if not specified).
        #[clap(short, long)]
        out: Option<String>,
        #[clap(short, long)]
        idl: Option<String>,
    },
}

fn main() -> Result<()> {
    let opts = Opts::parse();
    match opts.command {
        Command::Idl { file, out } => idl(file, out),
        Command::Gen { file, out, idl } => gen(file, out, idl),
    }
}

fn idl(file: String, out: Option<String>) -> Result<()> {
    let file = shellexpand::tilde(&file);
    let idl = anchor_syn::parser::file::parse(&file)?;
    let idl_json = serde_json::to_string_pretty(&idl)?;
    if let Some(out) = out {
        std::fs::write(out, idl_json)?;
        return Ok(());
    }
    println!("{}", idl_json);
    Ok(())
}

fn gen(file: Option<String>, out: Option<String>, idl: Option<String>) -> Result<()> {
    // TODO. Generate clients in any language.
    Ok(())
}
