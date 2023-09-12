use anyhow::{anyhow, Ok, Result};
use clap::Parser;
use parse as parser;
use std::fs;

mod android_gen;
mod ios_gen;
mod parse;

#[derive(Parser)]
struct Args {
    platform: String,
    input_dir: String,
    output_dir: String,
    default_lang: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    run_gen_pipeline(&args.platform, &args.input_dir, &args.output_dir, &args.default_lang)
}

fn run_gen_pipeline(
    platform: &String,
    input_dir: &String,
    output_dir: &String,
    default_lang: &Option<String>,
) -> Result<()> {
    // TODO add enum for Platform parameter
    return match platform.as_str() {
        "android" => run_android_gen_pipeline(input_dir, output_dir, default_lang),
        "ios" => run_ios_gen_pipeline(input_dir, output_dir, default_lang),
        _ => panic!("Invalid platform parameter. Use android or ios")
    };
}

fn run_android_gen_pipeline(
    input_dir: &String,
    output_dir: &String,
    default_lang: &Option<String>,
) -> Result<()> {
    for src in fs::read_dir(input_dir)? {
        let src = src?;
        if src.file_type()?.is_file() {
            let parsed = parser::parse(src.path()).map_err(|err| anyhow!(err))?;
            let generated = android_gen::generate(&parsed)?;
            generated.write(
                output_dir,
                src.path()
                    .file_stem()
                    .and_then(|os_str| os_str.to_str())
                    .ok_or(anyhow!("Cannot extract file name"))?,
                default_lang,
            )?;
        }
    }
    Ok(())
}

fn run_ios_gen_pipeline(
    input_dir: &String,
    output_dir: &String,
    default_lang: &Option<String>,
) -> Result<()> {
    let parsed_files: Vec<_> = fs::read_dir(input_dir)?.filter_map( |src| {
        let src = src.ok()?;
        // TODO: https://github.com/appKODE/utas/issues/33
        if src.file_type().ok()?.is_file() && src.file_name() != ".DS_Store" {
            let parsed = parser::parse(src.path()).map_err(|err| anyhow!(err)).ok()?;
            Some(parsed)
        } else {
            None
        }
    }).collect();

    let generated = ios_gen::generate(parsed_files)?;
    generated.write(output_dir,default_lang)?;

    Ok(())
}
