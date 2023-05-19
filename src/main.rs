use anyhow::{Ok, Result};
use clap::Parser;

mod parse;

#[derive(Parser)]
struct Args {
    input_dir: String,
    output_dir: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("{} {}", args.input_dir, args.output_dir);
    println!();
    parse::parse("/tmp/strings.txt");
    Ok(())
}
