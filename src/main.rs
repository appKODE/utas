use anyhow::{Ok, Result};
use clap::Parser;
use configparser::ini::Ini;

#[derive(Parser)]
struct Args {
    input_dir: String,
    output_dir: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("{} {}", args.input_dir, args.output_dir);
    println!();
    ini_test();
    Ok(())
}

fn ini_test() {
    let mut config = Ini::new();
    let map = config.load("/tmp/strings.txt").unwrap();
    println!("{:?}", map);
}
