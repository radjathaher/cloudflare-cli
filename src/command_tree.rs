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
