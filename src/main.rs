mod file;
mod parse;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
struct Args {
    input_dir: String,
    output_dir: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    run_android_gen_pipeline(&args.input_dir, &args.output_dir)
}

fn run_android_gen_pipeline(input_dir: &String, output_dir: &String) -> Result<()> {
    file::copy_recursively(input_dir, output_dir)
}
