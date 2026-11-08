use rune::TextEdit;
use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, Deserialize)]
pub struct Schema {
    pub schema_version: u32,
    pub voxtype_version: String,
    pub keys: Vec<Spec>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Spec {
    pub key: String,
    pub section: String,
    pub label: String,
    pub description: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub value: Value,
    pub file_value: Value,
    pub compiled: bool,
    pub restart_required: bool,
    pub engine: Option<String>,
    #[serde(default)]
    pub choices: Vec<String>,
    #[serde(default)]
    pub open: bool,
    pub source: Option<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

pub struct Field {
    pub spec: Spec,
    pub edit: TextEdit,
    pub original: String,
    pub clear: bool,
}

/// File-backed model keys get a Browse action. These are paths, unlike the
/// catalog names used by directory-based engines.
pub fn is_model_file_key(key: &str) -> bool {
    key.ends_with(".model") || key.ends_with(".secondary_model") || key.ends_with(".model_path")
}

/// A friendly model name for display. The configuration keeps the full path;
/// only the label loses the directory and whisper.cpp's `ggml-` prefix.
pub fn model_label(value: &str) -> String {
    if !(value.contains('/') || value.contains('\\')) {
        return value.to_owned();
    }
    let stem = std::path::Path::new(value)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(value);
    stem.strip_prefix("ggml-").unwrap_or(stem).to_owned()
}

pub fn display(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Array(a) => a.iter().map(display).collect::<Vec<_>>().join(","),
        other => other.to_string(),
    }
}

fn editable_text(kind: &str, value: &Value) -> String {
    match (kind, value) {
        ("float", Value::Number(n)) => {
            let mut text = format!("{:.4}", n.as_f64().unwrap_or_default());
            while text.ends_with('0') && text.len() - text.find('.').unwrap_or(0) > 2 {
                text.pop();
            }
            text
        }
        _ => display(value),
    }
}

impl Field {
    pub fn new(spec: Spec) -> Self {
        let original = editable_text(&spec.kind, &spec.value);
        let mut edit = TextEdit::new(&original);
        edit.set_secure(spec.key.contains("api_key"));
        Self {
            spec,
            edit,
            original,
            clear: false,
        }
    }
    pub fn dirty(&self) -> bool {
        if self.clear {
            !self.spec.file_value.is_null()
        } else {
            self.edit.text != self.original
        }
    }
    pub fn set(&mut self, value: &str) {
        self.edit = TextEdit::new(value);
        self.edit.set_secure(self.spec.key.contains("api_key"));
        self.clear = false;
    }
    pub fn reset_override(&mut self) {
        self.clear = true;
        self.edit = TextEdit::new(&self.original);
        self.edit.set_secure(self.spec.key.contains("api_key"));
    }
    pub fn value(&self) -> Result<Option<Value>, String> {
        if self.clear {
            return Ok(None);
        }
        let input = self.edit.text.as_str();
        // Emptying an optional value removes the override rather than
        // producing a type error; booleans always have an explicit state.
        if input.is_empty() && self.spec.kind != "bool" {
            return Ok(None);
        }
        let numeric = |n: f64| -> Result<(), String> {
            if !n.is_finite() {
                return Err("Enter a finite number.".into());
            }
            if self.spec.min.is_some_and(|m| n < m) || self.spec.max.is_some_and(|m| n > m) {
                return Err(format!(
                    "Use a value between {} and {}.",
                    self.spec.min.unwrap_or(f64::MIN),
                    self.spec.max.unwrap_or(f64::MAX)
                ));
            }
            Ok(())
        };
        let value = match self.spec.kind.as_str() {
            "bool" => Value::Bool(
                input
                    .parse()
                    .map_err(|_| "Expected true or false.".to_owned())?,
            ),
            "int" => {
                let n: i64 = input.parse().map_err(|_| {
                    "Enter a whole number, or use Default to remove the override.".to_owned()
                })?;
                numeric(n as f64)?;
                Value::from(n)
            }
            "float" => {
                let n: f64 = input.parse().map_err(|_| {
                    "Enter a number, or use Default to remove the override.".to_owned()
                })?;
                numeric(n)?;
                Value::from(n)
            }
            "enum" => {
                if !self.spec.open && !self.spec.choices.iter().any(|s| s == input) {
                    return Err(
                        "Choose a supported value, or use Default to remove the override.".into(),
                    );
                }
                if input.trim().is_empty() {
                    return Err("Use Default to remove an optional value.".into());
                }
                if self.spec.key.ends_with(".language") && input.contains(',') {
                    let languages: Vec<_> = input.split(',').map(str::trim).collect();
                    if languages.iter().any(|s| s.is_empty()) {
                        return Err("Language codes must not be empty.".into());
                    }
                    Value::Array(
                        languages
                            .into_iter()
                            .map(|s| Value::String(s.into()))
                            .collect(),
                    )
                } else {
                    Value::String(input.into())
                }
            }
            "string" | "dynamic_enum" => Value::String(input.into()),
            _ => return Err("This schema type is not supported. Use the full TOML editor.".into()),
        };
        Ok(Some(value))
    }
    pub fn error(&self) -> Option<String> {
        self.dirty().then(|| self.value().err()).flatten()
    }
}

impl Schema {
    pub fn parse(value: Value) -> Result<Self, String> {
        let schema: Self =
            serde_json::from_value(value).map_err(|e| format!("Invalid Voxtype schema: {e}"))?;
        if schema.schema_version != 1 {
            return Err(format!(
                "Unsupported Voxtype schema version {}.",
                schema.schema_version
            ));
        }
        if schema.keys.is_empty() {
            return Err("Voxtype returned no settings.".into());
        }
        Ok(schema)
    }
    pub fn demo() -> Self {
        Self::parse(serde_json::from_str(include_str!("../fixtures/schema-1.1.1.json")).unwrap())
            .unwrap()
    }
}
