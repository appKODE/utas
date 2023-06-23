use android_gen as generator;
use anyhow::{anyhow, Ok, Result};
use clap::Parser;
use parse as parser;
use std::fs;

mod android_gen;
mod parse;

#[derive(Parser)]
struct Args {
    input_dir: String,
    output_dir: String,
    default_lang: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    run_android_gen_pipeline(&args.input_dir, &args.output_dir, &args.default_lang)
}

fn run_android_gen_pipeline(input_dir: &String, output_dir: &String, default_lang: &Option<String>) -> Result<()> {
    for src in fs::read_dir(input_dir)? {
        let src = src?;
        if src.file_type()?.is_file() {
            let parsed = parser::parse(src.path()).map_err(|err| anyhow!(err))?;
            let generated = generator::generate(&parsed)?;
            generated.write(
                output_dir, 
                src.path().file_stem().and_then(|os_str| os_str.to_str()).ok_or(anyhow!("Cannot extract file name"))?,
                default_lang,
            )?;
        }
    }
    Ok(())
}
