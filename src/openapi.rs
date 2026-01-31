use anyhow::{Context, Result};
use serde_yaml::Value;
use std::collections::{BTreeMap, HashSet};

use crate::command_tree::{CommandTree, Operation, ParamDef, Resource};

pub fn build_command_tree(doc: &Value) -> Result<CommandTree> {
    let endpoint = doc
        .get("servers")
        .and_then(Value::as_sequence)
        .and_then(|servers| servers.first())
        .and_then(Value::as_mapping)
        .and_then(|server| server.get(&Value::String("url".into())))
        .and_then(Value::as_str)
        .unwrap_or("https://api.cloudflare.com/client/v4")
        .to_string();

    let version = doc
        .get("info")
        .and_then(Value::as_mapping)
        .and_then(|info| info.get(&Value::String("version".into())))
        .and_then(Value::as_str)
        .and_then(parse_major_version)
        .unwrap_or(4);

    let paths = doc
        .get("paths")
        .and_then(Value::as_mapping)
        .context("openapi missing paths")?;

    let mut resources: BTreeMap<String, Resource> = BTreeMap::new();
    let methods = [
        "get", "post", "put", "patch", "delete", "options", "head",
    ];

    for (path_value, path_item) in paths {
        let path = path_value
            .as_str()
            .context("path key must be string")?
            .to_string();
        let path_map = path_item
            .as_mapping()
            .context("path item must be mapping")?;

        let path_params = collect_parameters(path_map.get(&Value::String("parameters".into())));

        for method in methods {
            let op_value = match path_map.get(&Value::String(method.into())) {
                Some(value) => value,
                None => continue,
            };
            let op_map = op_value.as_mapping().context("op must be mapping")?;
            let op_id = op_map
                .get(&Value::String("operationId".into()))
                .and_then(Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| format!("{method}_{path}"));

            let summary = op_map
                .get(&Value::String("summary".into()))
                .and_then(Value::as_str)
                .map(str::to_string);
            let description = op_map
                .get(&Value::String("description".into()))
                .and_then(Value::as_str)
                .map(str::to_string);

            let op_params = collect_parameters(op_map.get(&Value::String("parameters".into())));
            let parameters = merge_parameters(path_params.clone(), op_params);

            let has_body = op_map.get(&Value::String("requestBody".into())).is_some();

            let tags = op_map
                .get(&Value::String("tags".into()))
                .and_then(Value::as_sequence)
                .cloned()
                .unwrap_or_default();

            for tag_value in tags {
                let tag = match tag_value.as_str() {
                    Some(t) => t.to_string(),
                    None => continue,
                };
                let res_name = normalize_name(&tag);
                let resource = resources.entry(res_name.clone()).or_insert_with(|| Resource {
                    name: res_name,
                    display_name: tag.clone(),
                    ops: Vec::new(),
                });

                let op_name = unique_op_name(resource, &normalize_name(&op_id), method);
                resource.ops.push(Operation {
                    name: op_name,
                    display_name: op_id.clone(),
                    method: method.to_uppercase(),
                    path: path.clone(),
                    summary: summary.clone(),
                    description: description.clone(),
                    parameters: parameters.clone(),
                    has_body,
                });
            }
        }
    }

    let resources = resources.into_values().collect();
    Ok(CommandTree {
        version,
        endpoint,
        resources,
    })
}

fn parse_major_version(input: &str) -> Option<u32> {
    input
        .split('.')
        .next()
        .and_then(|s| s.parse::<u32>().ok())
}

fn collect_parameters(value: Option<&Value>) -> Vec<ParamDef> {
    let mut out = Vec::new();
    let Some(list) = value.and_then(Value::as_sequence) else {
        return out;
    };

    for item in list {
        let Some(map) = item.as_mapping() else {
            continue;
        };
        let name = map
            .get(&Value::String("name".into()))
            .and_then(Value::as_str)
            .map(str::to_string);
        let location = map
            .get(&Value::String("in".into()))
            .and_then(Value::as_str)
            .map(str::to_string);

        let (name, location) = match (name, location) {
            (Some(n), Some(l)) => (n, l),
            _ => continue,
        };

        let required = map
            .get(&Value::String("required".into()))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let description = map
            .get(&Value::String("description".into()))
            .and_then(Value::as_str)
            .map(str::to_string);

        let schema = map.get(&Value::String("schema".into()));
        let (schema_type, list) = parse_schema(schema);

        out.push(ParamDef {
            name: name.clone(),
            flag: normalize_flag(&name),
            location,
            required,
            list,
            schema_type,
            description,
        });
    }

    out
}

fn parse_schema(value: Option<&Value>) -> (Option<String>, bool) {
    let Some(schema) = value.and_then(Value::as_mapping) else {
        return (None, false);
    };
    let schema_type = schema
        .get(&Value::String("type".into()))
        .and_then(Value::as_str)
        .map(str::to_string);
    let list = schema_type.as_deref() == Some("array");
    let schema_type = if list {
        schema
            .get(&Value::String("items".into()))
            .and_then(Value::as_mapping)
            .and_then(|items| items.get(&Value::String("type".into())))
            .and_then(Value::as_str)
            .map(str::to_string)
            .or(Some("array".to_string()))
    } else {
        schema_type
    };
    (schema_type, list)
}

fn merge_parameters(base: Vec<ParamDef>, override_params: Vec<ParamDef>) -> Vec<ParamDef> {
    let mut map: BTreeMap<(String, String), ParamDef> = BTreeMap::new();
    for param in base {
        map.insert((param.name.clone(), param.location.clone()), param);
    }
    for param in override_params {
        map.insert((param.name.clone(), param.location.clone()), param);
    }
    map.into_values().collect()
}

fn normalize_name(input: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
            dash = false;
        } else if !dash {
            out.push('-');
            dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn normalize_flag(input: &str) -> String {
    normalize_name(input).replace("--", "-")
}

fn unique_op_name(resource: &Resource, base: &str, method: &str) -> String {
    let existing: HashSet<&str> = resource.ops.iter().map(|op| op.name.as_str()).collect();
    if !existing.contains(base) {
        return base.to_string();
    }

    let candidate = format!("{base}-{method}");
    if !existing.contains(candidate.as_str()) {
        return candidate;
    }

    let mut idx = 2;
    loop {
        let next = format!("{candidate}-{idx}");
        if !existing.contains(next.as_str()) {
            return next;
        }
        idx += 1;
    }
}
