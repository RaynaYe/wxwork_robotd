use std::collections::HashMap;
use std::sync::Arc;

use regex::Regex;
use serde_json;

#[derive(Debug, Clone)]
pub struct WXWorkCommandHelp {
    pub prefix: String,
    pub suffix: String,
}

#[derive(Debug, Clone)]
pub struct WXWorkCommandEcho {
    pub echo: String,
}

#[derive(Debug, Clone, Copy)]
pub enum WXWorkCommandSpawnOutputType {
    Markdown,
    Text,
    Image,
}

#[derive(Debug, Clone)]
pub struct WXWorkCommandSpawn {
    pub exec: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub output_type: WXWorkCommandSpawnOutputType,
}

#[derive(Debug, Clone, Copy)]
pub enum WXWorkCommandHttpMethod {
    Auto,
    Get,
    Post,
    Delete,
    Head,
    Put,
}

#[derive(Debug, Clone)]
pub struct WXWorkCommandHttp {
    pub url: String,
    pub echo: String,
    pub post: String,
    pub method: WXWorkCommandHttpMethod,
    pub content_type: String,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub enum WXWorkCommandData {
    ECHO(Arc<WXWorkCommandEcho>),
    SPAWN(Arc<WXWorkCommandSpawn>),
    HTTP(Arc<WXWorkCommandHttp>),
    HELP(Arc<WXWorkCommandHelp>),
}

#[derive(Debug, Clone)]
pub struct WXWorkCommand {
    pub data: WXWorkCommandData,
    name: Arc<String>,
    pub envs: serde_json::Value,
    rule: Regex,
    config: serde_json::Value,
    pub hidden: bool,
    pub description: Arc<String>,
}

#[derive(Debug, Clone)]
pub struct WXWorkCommandMatch(serde_json::Value);

pub type WXWorkCommandPtr = Arc<WXWorkCommand>;
pub type WXWorkCommandList = Vec<WXWorkCommandPtr>;

pub fn read_string_from_json_object(json: &serde_json::Value, name: &str) -> Option<String> {
    if let Some(ref x) = json.as_object() {
        if let Some(ref v) = x.get(name) {
            if let Some(r) = v.as_str() {
                return Some(String::from(r));
            }
        }
    }

    None
}

pub fn read_object_from_json_object<'a>(
    json: &'a serde_json::Value,
    name: &str,
) -> Option<&'a serde_json::map::Map<String, serde_json::Value>> {
    if let Some(ref x) = json.as_object() {
        if let Some(ref v) = x.get(name) {
            if let Some(r) = v.as_object() {
                return Some(r);
            }
        }
    }

    None
}

pub fn read_bool_from_json_object<'a>(json: &'a serde_json::Value, name: &str) -> Option<bool> {
    if let Some(ref x) = json.as_object() {
        if let Some(ref v) = x.get(name) {
            if let Some(r) = v.as_bool() {
                return Some(r);
            }
        }
    }

    None
}

pub fn read_array_from_json_object<'a>(
    json: &'a serde_json::Value,
    name: &str,
) -> Option<&'a Vec<serde_json::Value>> {
    if let Some(ref x) = json.as_object() {
        if let Some(ref v) = x.get(name) {
            if let Some(r) = v.as_array() {
                return Some(r);
            }
        }
    }

    None
}

pub fn merge_envs(mut l: serde_json::Value, r: &serde_json::Value) -> serde_json::Value {
    if !l.is_object() {
        return l;
    }

    if !r.is_object() {
        return l;
    }

    if let Some(kvs) = r.as_object() {
        for (k, v) in kvs {
            match v {
                serde_json::Value::Null => {
                    l[k] = v.clone();
                }
                serde_json::Value::Bool(_) => {
                    l[k] = v.clone();
                }
                serde_json::Value::Number(_) => {
                    l[k] = v.clone();
                }
                serde_json::Value::String(_) => {
                    l[k] = v.clone();
                }
                _ => {}
            }
        }
    }

    l
}

impl WXWorkCommand {
    pub fn parse(json: &serde_json::Value) -> WXWorkCommandList {
        let mut ret: WXWorkCommandList = Vec::new();

        if let Some(kvs) = json.as_object() {
            for (k, v) in kvs {
                let cmd_res = WXWorkCommand::new(k, v);
                if let Some(cmd) = cmd_res {
                    ret.push(Arc::new(cmd));
                }
            }
        }

        ret
    }

    pub fn new(cmd_name: &str, json: &serde_json::Value) -> Option<WXWorkCommand> {
        let cmd_data: WXWorkCommandData;
        let mut envs_obj = json!({});
        let rule_obj = match Regex::new(cmd_name) {
            Ok(x) => x,
            Err(e) => {
                error!("command {} regex invalid: {}\n{}", cmd_name, json, e);
                return None;
            }
        };

        {
            if !json.is_object() {
                error!(
                    "command {} configure must be a json object, but real is {}",
                    cmd_name, json
                );
                return None;
            };

            let type_name = if let Some(x) = read_string_from_json_object(json, "type") {
                x
            } else {
                error!("command {} configure require type: {}", cmd_name, json);
                return None;
            };

            cmd_data = match type_name.as_str() {
                "echo" => WXWorkCommandData::ECHO(Arc::new(WXWorkCommandEcho {
                    echo: if let Some(x) = read_string_from_json_object(json, "echo") {
                        x
                    } else {
                        String::from("Ok")
                    },
                })),
                "spawn" => {
                    let exec_field = if let Some(x) = read_string_from_json_object(json, "exec") {
                        x
                    } else {
                        error!("spawn command {} requires exec: {}", cmd_name, json);
                        return None;
                    };

                    let mut args_field: Vec<String> = Vec::new();
                    if let Some(arr) = read_array_from_json_object(json, "args") {
                        for v in arr {
                            args_field.push(match v {
                                serde_json::Value::Null => String::default(),
                                serde_json::Value::Bool(x) => {
                                    if *x {
                                        String::from("true")
                                    } else {
                                        String::from("false")
                                    }
                                }
                                serde_json::Value::Number(x) => x.to_string(),
                                serde_json::Value::String(x) => x.clone(),
                                x => x.to_string(),
                            });
                        }
                    }

                    let cwd_field = if let Some(x) = read_string_from_json_object(json, "cwd") {
                        x
                    } else {
                        String::default()
                    };

                    WXWorkCommandData::SPAWN(Arc::new(WXWorkCommandSpawn {
                        exec: exec_field,
                        args: args_field,
                        cwd: cwd_field,
                        output_type: match read_string_from_json_object(json, "output_type") {
                            Some(x) => match x.to_lowercase().as_str() {
                                "text" => WXWorkCommandSpawnOutputType::Text,
                                "image" => WXWorkCommandSpawnOutputType::Image,
                                _ => WXWorkCommandSpawnOutputType::Markdown,
                            },
                            None => WXWorkCommandSpawnOutputType::Markdown,
                        },
                    }))
                }
                "http" => {
                    let url_field = if let Some(x) = read_string_from_json_object(json, "url") {
                        x
                    } else {
                        error!("http command {} requires url: {}", cmd_name, json);
                        return None;
                    };
                    let echo_field = if let Some(x) = read_string_from_json_object(json, "echo") {
                        x
                    } else {
                        String::from("Ok")
                    };
                    let post_field = if let Some(x) = read_string_from_json_object(json, "post") {
                        x
                    } else {
                        String::from("Ok")
                    };

                    WXWorkCommandData::HTTP(Arc::new(WXWorkCommandHttp {
                        url: url_field,
                        echo: echo_field,
                        post: post_field,
                        method: match read_string_from_json_object(json, "method") {
                            Some(x) => match x.to_lowercase().as_str() {
                                "get" => WXWorkCommandHttpMethod::Get,
                                "post" => WXWorkCommandHttpMethod::Post,
                                "delete" => WXWorkCommandHttpMethod::Delete,
                                "put" => WXWorkCommandHttpMethod::Put,
                                "head" => WXWorkCommandHttpMethod::Head,
                                _ => WXWorkCommandHttpMethod::Auto,
                            },
                            None => WXWorkCommandHttpMethod::Auto,
                        },
                        content_type: if let Some(x) =
                            read_string_from_json_object(json, "content_type")
                        {
                            x
                        } else {
                            String::default()
                        },
                        headers: if let Some(m) = read_object_from_json_object(json, "headers") {
                            let mut res = HashMap::new();
                            for (k, v) in m {
                                res.insert(
                                    k.clone(),
                                    match v {
                                        serde_json::Value::Null => String::default(),
                                        serde_json::Value::Bool(x) => {
                                            if *x {
                                                String::from("true")
                                            } else {
                                                String::from("false")
                                            }
                                        }
                                        serde_json::Value::Number(x) => x.to_string(),
                                        serde_json::Value::String(x) => x.clone(),
                                        x => x.to_string(),
                                    },
                                );
                            }

                            res
                        } else {
                            HashMap::new()
                        },
                    }))
                }
                "help" => WXWorkCommandData::HELP(Arc::new(WXWorkCommandHelp {
                    prefix: if let Some(x) = read_string_from_json_object(json, "prefix") {
                        x
                    } else {
                        String::default()
                    },
                    suffix: if let Some(x) = read_string_from_json_object(json, "suffix") {
                        x
                    } else {
                        String::default()
                    },
                })),

                _ => {
                    error!("command {} configure type invalid: {}", cmd_name, json);
                    return None;
                }
            };

            if let Some(envs_kvs) = read_object_from_json_object(json, "env") {
                for (k, v) in envs_kvs {
                    envs_obj[format!("WXWORK_ROBOT_CMD_{}", k).as_str().to_uppercase()] =
                        if v.is_string() {
                            v.clone()
                        } else {
                            serde_json::Value::String(v.to_string())
                        };
                }
            }
        }

        Some(WXWorkCommand {
            data: cmd_data,
            name: Arc::new(String::from(cmd_name)),
            rule: rule_obj,
            envs: envs_obj,
            config: json.clone(),
            hidden: if let Some(x) = read_bool_from_json_object(json, "hidden") {
                x
            } else {
                false
            },
            description: if let Some(x) = read_string_from_json_object(json, "description") {
                Arc::new(x)
            } else {
                Arc::new(String::default())
            },
        })
    }

    pub fn name(&self) -> Arc<String> {
        self.name.clone()
    }

    pub fn try_capture(&self, message: &str) -> WXWorkCommandMatch {
        let caps = if let Some(x) = self.rule.captures(message) {
            x
        } else {
            return WXWorkCommandMatch(serde_json::Value::Null);
        };

        let mut json = self.envs.clone();
        json["WXWORK_ROBOT_CMD"] =
            serde_json::Value::String(String::from(caps.get(0).unwrap().as_str()));

        for cap_name in self.rule.capture_names() {
            if let Some(key) = cap_name {
                if let Some(m) = caps.name(key) {
                    json[format!("WXWORK_ROBOT_CMD_{}", key).as_str().to_uppercase()] =
                        serde_json::Value::String(String::from(m.as_str()));
                }
            }
        }

        WXWorkCommandMatch(json)
    }

    pub fn description(&self) -> Arc<String> {
        self.description.clone()
    }

    pub fn is_hidden(&self) -> bool {
        self.hidden
    }
}

impl WXWorkCommandMatch {
    pub fn has_result(&self) -> bool {
        self.0.is_object()
    }

    pub fn ref_json(&self) -> &serde_json::Value {
        &self.0
    }

    pub fn mut_json(&mut self) -> &mut serde_json::Value {
        &mut self.0
    }
}

pub fn get_command_description(cmd: &WXWorkCommandPtr) -> Option<Arc<String>> {
    if cmd.is_hidden() {
        None
    } else {
        let desc = cmd.description();
        if desc.len() > 0 {
            Some(desc)
        } else {
            Some(cmd.name())
        }
    }
}
