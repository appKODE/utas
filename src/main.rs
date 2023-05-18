use anyhow::{Ok, Result};
use clap::Parser;

#[derive(Parser)]
struct Args {
    input_dir: String,
    output_dir: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("{} {}", args.input_dir, args.output_dir);
    Ok(())
}
