use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(dead_code)]
pub struct CommandTree {
    pub version: u32,
    pub endpoint: String,
    pub resources: Vec<Resource>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(dead_code)]
pub struct Resource {
    pub name: String,
    pub display_name: String,
    pub ops: Vec<Operation>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(dead_code)]
pub struct Operation {
    pub name: String,
    pub display_name: String,
    pub method: String,
    pub path: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub parameters: Vec<ParamDef>,
    pub has_body: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[allow(dead_code)]
pub struct ParamDef {
    pub name: String,
    pub flag: String,
    pub location: String,
    pub required: bool,
    pub list: bool,
    pub schema_type: Option<String>,
    pub description: Option<String>,
}

pub fn load_command_tree() -> CommandTree {
    let raw = include_str!("../schemas/command_tree.json");
    serde_json::from_str(raw).expect("invalid command_tree.json")
}

pub fn extract_path_param_names(path: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = path;

    while let Some(start) = rest.find('{') {
        let after_start = &rest[start + 1..];
        let Some(end) = after_start.find('}') else {
            break;
        };

        let name = &after_start[..end];
        if !name.is_empty() && !names.iter().any(|existing| existing == name) {
            names.push(name.to_string());
        }
        rest = &after_start[end + 1..];
    }

    names
}

#[cfg(test)]
mod tests {
    use super::extract_path_param_names;

    #[test]
    fn extracts_unique_path_parameters_in_order() {
        assert_eq!(
            extract_path_param_names("/accounts/{account_id}/workers/{script_name}/{account_id}"),
            vec!["account_id", "script_name"]
        );
    }
}
