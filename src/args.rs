use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "toma")]
#[command(author, version, about = "A dynamic web server configured via YAML", long_about = None)]
pub struct Args {
    pub input_file: Option<PathBuf>,

    #[arg(short, long)]
    pub debug: bool,
}

impl Args {
    pub fn new() -> Self {
        Args::parse()
    }
}
