use serde_json::{json, Value};

fn id_schema(kind: &str) -> Value {
    json!({
        "type": "string",
        "minLength": 1,
        "maxLength": 128,
        "pattern": format!("^{kind}_[A-Za-z0-9_-]{{16,64}}$")
    })
}

fn client_schema() -> Value {
    json!({"type": "string", "minLength": 1, "maxLength": 128})
}

fn action_schema(
    action: &'static str,
    fields: Vec<(&'static str, Value)>,
    required: &[&str],
) -> Value {
    let mut properties = serde_json::Map::new();
    properties.insert(
        "action".to_string(),
        json!({"type": "string", "const": action}),
    );
    for (name, schema) in fields {
        properties.insert(name.to_string(), schema);
    }
    let mut required_fields = vec![Value::String("action".to_string())];
    required_fields.extend(
        required
            .iter()
            .map(|field| Value::String((*field).to_string())),
    );
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": properties,
        "required": required_fields
    })
}

fn gateway_schema(branches: Vec<Value>) -> Value {
    let mut properties = serde_json::Map::new();
    for branch in &branches {
        for field in branch["properties"]
            .as_object()
            .expect("Browser action properties")
            .keys()
        {
            properties.entry(field.clone()).or_insert_with(|| json!({}));
        }
    }
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": properties,
        "required": ["action"],
        "oneOf": branches
    })
}

pub fn browser_observe_input_schema() -> Value {
    gateway_schema(vec![
        action_schema("targets", vec![], &[]),
        action_schema(
            "browsers",
            vec![("client_id", client_schema())],
            &["client_id"],
        ),
        action_schema(
            "pages",
            vec![
                ("client_id", client_schema()),
                ("browser_id", id_schema("browser")),
                (
                    "limit",
                    json!({"type": "integer", "minimum": 1, "maximum": 32}),
                ),
            ],
            &["client_id", "browser_id"],
        ),
        action_schema(
            "snapshot",
            vec![
                ("client_id", client_schema()),
                ("browser_id", id_schema("browser")),
                ("page_id", id_schema("page")),
            ],
            &["client_id", "browser_id", "page_id"],
        ),
        action_schema(
            "screenshot",
            vec![
                ("client_id", client_schema()),
                ("browser_id", id_schema("browser")),
                ("page_id", id_schema("page")),
            ],
            &["client_id", "browser_id", "page_id"],
        ),
    ])
}

pub fn browser_act_input_schema() -> Value {
    let element_fields = || {
        vec![
            ("client_id", client_schema()),
            ("browser_id", id_schema("browser")),
            ("page_id", id_schema("page")),
            ("element_id", id_schema("element")),
        ]
    };
    gateway_schema(vec![
        action_schema(
            "launch",
            vec![("client_id", client_schema())],
            &["client_id"],
        ),
        action_schema(
            "new_page",
            vec![
                ("client_id", client_schema()),
                ("browser_id", id_schema("browser")),
            ],
            &["client_id", "browser_id"],
        ),
        action_schema(
            "navigate",
            vec![
                ("client_id", client_schema()),
                ("browser_id", id_schema("browser")),
                ("page_id", id_schema("page")),
                (
                    "url",
                    json!({"type": "string", "minLength": 1, "maxLength": 8192, "pattern": "^https?://"}),
                ),
            ],
            &["client_id", "browser_id", "page_id", "url"],
        ),
        action_schema(
            "click",
            element_fields(),
            &["client_id", "browser_id", "page_id", "element_id"],
        ),
        action_schema(
            "input_text",
            {
                let mut fields = element_fields();
                fields.push((
                    "text",
                    json!({"type": "string", "minLength": 1, "maxLength": 4096}),
                ));
                fields
            },
            &["client_id", "browser_id", "page_id", "element_id", "text"],
        ),
        action_schema(
            "key",
            vec![
                ("client_id", client_schema()),
                ("browser_id", id_schema("browser")),
                ("page_id", id_schema("page")),
                (
                    "key",
                    json!({"type": "string", "enum": ["enter", "tab", "escape", "backspace", "delete", "arrow_up", "arrow_down", "arrow_left", "arrow_right", "home", "end", "page_up", "page_down", "space"]}),
                ),
            ],
            &["client_id", "browser_id", "page_id", "key"],
        ),
        action_schema(
            "close_page",
            vec![
                ("client_id", client_schema()),
                ("browser_id", id_schema("browser")),
                ("page_id", id_schema("page")),
            ],
            &["client_id", "browser_id", "page_id"],
        ),
        action_schema(
            "close_browser",
            vec![
                ("client_id", client_schema()),
                ("browser_id", id_schema("browser")),
            ],
            &["client_id", "browser_id"],
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn browser_gateways_are_strict_closed_unions() {
        let observe = browser_observe_input_schema();
        let act = browser_act_input_schema();
        assert_eq!(observe["additionalProperties"], false);
        assert_eq!(act["additionalProperties"], false);
        let observe_actions = observe["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .map(|branch| branch["properties"]["action"]["const"].as_str().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            observe_actions,
            BTreeSet::from(["targets", "browsers", "pages", "snapshot", "screenshot"])
        );
        let act_actions = act["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .map(|branch| branch["properties"]["action"]["const"].as_str().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            act_actions,
            BTreeSet::from([
                "launch",
                "new_page",
                "navigate",
                "click",
                "input_text",
                "key",
                "close_page",
                "close_browser"
            ])
        );
        for schema in [observe, act] {
            for branch in schema["oneOf"].as_array().unwrap() {
                assert_eq!(branch["additionalProperties"], false);
                let properties = branch["properties"].as_object().unwrap();
                for forbidden in [
                    "method",
                    "params",
                    "cdp_method",
                    "cdp_json",
                    "javascript",
                    "evaluate",
                    "script",
                    "debugger_url",
                    "websocket_url",
                    "executable",
                    "argv",
                    "profile_path",
                ] {
                    assert!(
                        !properties.contains_key(forbidden),
                        "Browser model schema exposed forbidden raw field {forbidden}"
                    );
                }
            }
        }
    }
}
