use anyhow::{Context, Result};
use clap::{Arg, Command};
use std::fs;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let matches = Command::new("gen_command_tree")
        .arg(Arg::new("openapi").long("openapi").required(true))
        .arg(Arg::new("out").long("out").required(true))
        .get_matches();

    let openapi_path = matches
        .get_one::<String>("openapi")
        .context("openapi path missing")?;
    let out_path = matches
        .get_one::<String>("out")
        .context("out path missing")?;

    let raw = fs::read_to_string(openapi_path)
        .with_context(|| format!("read openapi {}", openapi_path))?;
    let doc: serde_yaml::Value = serde_yaml::from_str(&raw).context("parse openapi yaml")?;
    let tree = cloudflare_cli::openapi::build_command_tree(&doc)?;
    let json = serde_json::to_string_pretty(&tree)?;
    fs::write(out_path, json).with_context(|| format!("write {}", out_path))?;
    Ok(())
}
