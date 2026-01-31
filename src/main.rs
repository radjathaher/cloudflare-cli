use anyhow::{Context, Result, anyhow};
use clap::{Arg, ArgAction, Command};
use cloudflare_cli::command_tree::{CommandTree, Operation, ParamDef};
use cloudflare_cli::http::HttpClient;
use serde_json::{Value, json};
use std::{env, fs, io::Write};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let tree = cloudflare_cli::command_tree::load_command_tree();
    let cli = build_cli(&tree);
    let matches = cli.get_matches();

    if let Some(matches) = matches.subcommand_matches("list") {
        return handle_list(&tree, matches);
    }
    if let Some(matches) = matches.subcommand_matches("describe") {
        return handle_describe(&tree, matches);
    }
    if let Some(matches) = matches.subcommand_matches("tree") {
        return handle_tree(&tree, matches);
    }
    if let Some(matches) = matches.subcommand_matches("api") {
        return handle_api(&tree, matches);
    }

    let token = env::var("CLOUDFLARE_API_TOKEN").context("CLOUDFLARE_API_TOKEN missing")?;
    let endpoint = env::var("CLOUDFLARE_API_URL").unwrap_or_else(|_| tree.endpoint.clone());

    let pretty = matches.get_flag("pretty");
    let raw = matches.get_flag("raw");
    let headers = parse_headers(matches.get_many::<String>("header"));

    let (res_name, res_matches) = matches
        .subcommand()
        .ok_or_else(|| anyhow!("resource required"))?;
    let (op_name, op_matches) = res_matches
        .subcommand()
        .ok_or_else(|| anyhow!("operation required"))?;

    let op = find_op(&tree, res_name, op_name)
        .ok_or_else(|| anyhow!("unknown command {res_name} {op_name}"))?;

    let (path, query, body, extra_headers) = build_request(&op, op_matches)?;
    let mut headers = headers;
    headers.extend(extra_headers);

    let method = op.method.parse().context("invalid http method")?;
    let client = HttpClient::new(endpoint, token)?;
    let response = client.execute(method, &path, &query, &headers, body)?;

    let output = format_output(&response.body, raw)?;
    write_json_output(output, pretty)?;

    if response.status >= 400 {
        return Err(anyhow!("http {}", response.status));
    }

    Ok(())
}

fn build_cli(tree: &CommandTree) -> Command {
    let mut cmd = Command::new("cloudflare")
        .about("Cloudflare CLI (OpenAPI-powered)")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .arg(
            Arg::new("pretty")
                .long("pretty")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Pretty-print JSON output"),
        )
        .arg(
            Arg::new("raw")
                .long("raw")
                .global(true)
                .action(ArgAction::SetTrue)
                .help("Return full API response"),
        )
        .arg(
            Arg::new("header")
                .long("header")
                .global(true)
                .action(ArgAction::Append)
                .value_name("NAME:VALUE")
                .help("Add header (repeatable)"),
        );

    cmd = cmd.subcommand(
        Command::new("list")
            .about("List resources and operations")
            .arg(
                Arg::new("json")
                    .long("json")
                    .action(ArgAction::SetTrue)
                    .help("Emit machine-readable JSON"),
            ),
    );

    cmd = cmd.subcommand(
        Command::new("describe")
            .about("Describe a specific operation")
            .arg(Arg::new("resource").required(true))
            .arg(Arg::new("op").required(true))
            .arg(
                Arg::new("json")
                    .long("json")
                    .action(ArgAction::SetTrue)
                    .help("Emit machine-readable JSON"),
            ),
    );

    cmd = cmd.subcommand(
        Command::new("tree").about("Show full command tree").arg(
            Arg::new("json")
                .long("json")
                .action(ArgAction::SetTrue)
                .help("Emit machine-readable JSON"),
        ),
    );

    cmd = cmd.subcommand(
        Command::new("api")
            .about("Call any API endpoint")
            .arg(Arg::new("method").required(true))
            .arg(Arg::new("path").required(true))
            .arg(
                Arg::new("query")
                    .long("query")
                    .action(ArgAction::Append)
                    .value_name("KEY=VALUE")
                    .help("Query param (repeatable)"),
            )
            .arg(
                Arg::new("body")
                    .long("body")
                    .value_name("JSON")
                    .conflicts_with("body-file")
                    .help("JSON request body"),
            )
            .arg(
                Arg::new("body-file")
                    .long("body-file")
                    .value_name("PATH")
                    .conflicts_with("body")
                    .help("Read JSON request body from file"),
            ),
    );

    for resource in &tree.resources {
        let mut res_cmd = Command::new(resource.name.clone())
            .about(resource.display_name.clone())
            .subcommand_required(true)
            .arg_required_else_help(true);
        for op in &resource.ops {
            let mut op_cmd = Command::new(op.name.clone()).about(op.display_name.clone());
            for param in &op.parameters {
                op_cmd = op_cmd.arg(build_param_arg(param));
            }
            if op.has_body {
                op_cmd = op_cmd.arg(
                    Arg::new("body")
                        .long("body")
                        .value_name("JSON")
                        .conflicts_with("body-file")
                        .help("JSON request body"),
                );
                op_cmd = op_cmd.arg(
                    Arg::new("body-file")
                        .long("body-file")
                        .value_name("PATH")
                        .conflicts_with("body")
                        .help("Read JSON request body from file"),
                );
            }
            res_cmd = res_cmd.subcommand(op_cmd);
        }
        cmd = cmd.subcommand(res_cmd);
    }

    cmd
}

fn build_param_arg(param: &ParamDef) -> Arg {
    let mut arg = Arg::new(param.flag.clone())
        .long(param.flag.clone())
        .value_name(param.name.clone())
        .help(param.description.clone().unwrap_or_else(|| param.location.clone()));

    if param.list {
        arg = arg.action(ArgAction::Append);
    }

    arg
}

fn handle_list(tree: &CommandTree, matches: &clap::ArgMatches) -> Result<()> {
    if matches.get_flag("json") {
        let mut out = Vec::new();
        for res in &tree.resources {
            let ops: Vec<String> = res.ops.iter().map(|op| op.name.clone()).collect();
            out.push(json!({"resource": res.name, "display": res.display_name, "ops": ops}));
        }
        write_stdout_line(&serde_json::to_string_pretty(&out)?)?;
        return Ok(());
    }

    for res in &tree.resources {
        write_stdout_line(&format!("{} ({})", res.name, res.display_name))?;
        for op in &res.ops {
            write_stdout_line(&format!("  {} ({})", op.name, op.display_name))?;
        }
    }
    Ok(())
}

fn handle_describe(tree: &CommandTree, matches: &clap::ArgMatches) -> Result<()> {
    let resource = matches
        .get_one::<String>("resource")
        .ok_or_else(|| anyhow!("resource required"))?;
    let op_name = matches
        .get_one::<String>("op")
        .ok_or_else(|| anyhow!("operation required"))?;

    let op = find_op(tree, resource, op_name)
        .ok_or_else(|| anyhow!("unknown command {resource} {op_name}"))?;

    if matches.get_flag("json") {
        write_stdout_line(&serde_json::to_string_pretty(op)?)?;
        return Ok(());
    }

    write_stdout_line(&format!("{} {}", op.method, op.path))?;
    write_stdout_line(&format!("name: {}", op.display_name))?;
    if let Some(summary) = &op.summary {
        write_stdout_line(&format!("summary: {summary}"))?;
    }
    if let Some(description) = &op.description {
        write_stdout_line(&format!("description: {description}"))?;
    }
    if !op.parameters.is_empty() {
        write_stdout_line("params:")?;
        for param in &op.parameters {
            write_stdout_line(&format!(
                "  --{} ({}, required: {})",
                param.flag, param.location, param.required
            ))?;
        }
    }
    Ok(())
}

fn handle_tree(tree: &CommandTree, matches: &clap::ArgMatches) -> Result<()> {
    if matches.get_flag("json") {
        write_stdout_line(&serde_json::to_string_pretty(tree)?)?;
        return Ok(());
    }

    for res in &tree.resources {
        write_stdout_line(&format!("{} ({})", res.name, res.display_name))?;
        for op in &res.ops {
            write_stdout_line(&format!("  {} ({})", op.name, op.display_name))?;
        }
    }
    Ok(())
}

fn handle_api(tree: &CommandTree, matches: &clap::ArgMatches) -> Result<()> {
    let token = env::var("CLOUDFLARE_API_TOKEN").context("CLOUDFLARE_API_TOKEN missing")?;
    let endpoint = env::var("CLOUDFLARE_API_URL").unwrap_or_else(|_| tree.endpoint.clone());

    let pretty = matches.get_flag("pretty");
    let raw = matches.get_flag("raw");
    let headers = parse_headers(matches.get_many::<String>("header"));

    let method = matches
        .get_one::<String>("method")
        .ok_or_else(|| anyhow!("method required"))?;
    let path = matches
        .get_one::<String>("path")
        .ok_or_else(|| anyhow!("path required"))?;

    let query = parse_key_values(matches.get_many::<String>("query"))?;
    let body = load_body(matches.get_one::<String>("body"), matches.get_one::<String>("body-file"))?;

    let client = HttpClient::new(endpoint, token)?;
    let response = client.execute(method.parse()?, path, &query, &headers, body)?;

    let output = format_output(&response.body, raw)?;
    write_json_output(output, pretty)?;

    if response.status >= 400 {
        return Err(anyhow!("http {}", response.status));
    }

    Ok(())
}

fn build_request(op: &Operation, matches: &clap::ArgMatches) -> Result<(String, Vec<(String, String)>, Option<Value>, Vec<(String, String)>)> {
    let mut path = op.path.clone();
    let mut query = Vec::new();
    let mut headers = Vec::new();

    for param in &op.parameters {
        match param.location.as_str() {
            "path" => {
                let value = resolve_param_value(param, matches)?
                    .ok_or_else(|| anyhow!("missing path param {}", param.name))?;
                path = path.replace(&format!("{{{}}}", param.name), &urlencoding::encode(&value));
            }
            "query" => {
                let values = resolve_param_values(param, matches)?;
                if param.required && values.is_empty() {
                    return Err(anyhow!("missing query param {}", param.name));
                }
                for value in values {
                    query.push((param.name.clone(), value));
                }
            }
            "header" => {
                let values = resolve_param_values(param, matches)?;
                if param.required && values.is_empty() {
                    return Err(anyhow!("missing header param {}", param.name));
                }
                for value in values {
                    headers.push((param.name.clone(), value));
                }
            }
            _ => {}
        }
    }

    let body = load_body(matches.get_one::<String>("body"), matches.get_one::<String>("body-file"))?;
    Ok((path, query, body, headers))
}

fn resolve_param_value(param: &ParamDef, matches: &clap::ArgMatches) -> Result<Option<String>> {
    if let Some(value) = matches.get_one::<String>(&param.flag) {
        return Ok(Some(value.to_string()));
    }
    if let Some(value) = default_env_for_param(&param.name) {
        return Ok(Some(value));
    }
    Ok(None)
}

fn resolve_param_values(param: &ParamDef, matches: &clap::ArgMatches) -> Result<Vec<String>> {
    if param.list {
        let mut values = Vec::new();
        if let Some(items) = matches.get_many::<String>(&param.flag) {
            for item in items {
                values.extend(split_list(item));
            }
        }
        return Ok(values);
    }

    if let Some(value) = matches.get_one::<String>(&param.flag) {
        return Ok(vec![value.to_string()]);
    }
    Ok(Vec::new())
}

fn default_env_for_param(name: &str) -> Option<String> {
    match name {
        "account_id" | "account_identifier" | "accountId" => env::var("CLOUDFLARE_ACCOUNT_ID").ok(),
        "zone_id" | "zone_identifier" | "zoneId" => env::var("CLOUDFLARE_ZONE_ID").ok(),
        _ => None,
    }
}

fn split_list(value: &str) -> Vec<String> {
    if value.contains(',') {
        value.split(',').map(|v| v.trim().to_string()).filter(|v| !v.is_empty()).collect()
    } else {
        vec![value.to_string()]
    }
}

fn load_body(body: Option<&String>, body_file: Option<&String>) -> Result<Option<Value>> {
    if let Some(raw) = body {
        let value = serde_json::from_str(raw).context("invalid JSON body")?;
        return Ok(Some(value));
    }
    if let Some(path) = body_file {
        let raw = fs::read_to_string(path).with_context(|| format!("read body file {}", path))?;
        let value = serde_json::from_str(&raw).context("invalid JSON body file")?;
        return Ok(Some(value));
    }
    Ok(None)
}

fn parse_headers(values: Option<clap::parser::ValuesRef<String>>) -> Vec<(String, String)> {
    let mut headers = Vec::new();
    if let Some(values) = values {
        for value in values {
            if let Some((k, v)) = split_key_value(value) {
                headers.push((k.to_string(), v.to_string()));
            }
        }
    }
    headers
}

fn parse_key_values(values: Option<clap::parser::ValuesRef<String>>) -> Result<Vec<(String, String)>> {
    let mut out = Vec::new();
    if let Some(values) = values {
        for value in values {
            if let Some((k, v)) = split_key_value(value) {
                out.push((k.to_string(), v.to_string()));
            }
        }
    }
    Ok(out)
}

fn split_key_value(value: &str) -> Option<(&str, &str)> {
    if let Some((k, v)) = value.split_once('=') {
        return Some((k, v));
    }
    if let Some((k, v)) = value.split_once(':') {
        return Some((k, v));
    }
    None
}

fn format_output(body: &Value, raw: bool) -> Result<Value> {
    if raw {
        return Ok(body.clone());
    }
    if let Some(result) = body.get("result") {
        return Ok(result.clone());
    }
    Ok(body.clone())
}

fn write_json_output(value: Value, pretty: bool) -> Result<()> {
    if pretty {
        write_stdout_line(&serde_json::to_string_pretty(&value)?)?;
    } else {
        write_stdout_line(&serde_json::to_string(&value)?)?;
    }
    Ok(())
}

fn find_op<'a>(tree: &'a CommandTree, res_name: &str, op_name: &str) -> Option<&'a Operation> {
    tree.resources
        .iter()
        .find(|res| res.name == res_name)
        .and_then(|res| res.ops.iter().find(|op| op.name == op_name))
}

fn write_stdout_line(line: &str) -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    stdout.write_all(line.as_bytes())?;
    stdout.write_all(b"\n")?;
    Ok(())
}
