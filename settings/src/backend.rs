use crate::schema::Schema;
use serde_json::Value as Json;
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use tempfile::NamedTempFile;
use toml_edit::{DocumentMut, Item, Table, Value};

#[derive(Clone)]
pub struct Backend {
    pub path: PathBuf,
    pub voxtype: PathBuf,
    pub systemctl: PathBuf,
    pub restart_allowed: bool,
}

pub struct Loaded {
    pub schema: Schema,
    pub source: String,
    pub original: Option<Vec<u8>>,
    pub inventory: Inventory,
}

#[derive(Clone, Default)]
pub struct Inventory {
    pub devices: Vec<String>,
    pub models: BTreeMap<String, Vec<String>>,
    pub engines: Vec<String>,
    pub styles: Vec<String>,
    pub variants: String,
    pub service: String,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
pub struct Saved {
    pub schema: Schema,
    pub source: String,
    pub backup: Option<PathBuf>,
}

pub fn command(program: &Path, args: &[&OsStr]) -> Result<String, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not run {}: {e}", program.display()))?;
    // Drain both pipes while waiting: schema output can exceed the pipe capacity.
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let collect = |stream: Box<dyn Read + Send>| {
        thread::spawn(move || {
            let mut text = String::new();
            stream
                .take(8 * 1024 * 1024)
                .read_to_string(&mut text)
                .map(|_| text)
        })
    };
    let out = collect(Box::new(stdout));
    let err = collect(Box::new(stderr));
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if start.elapsed() < Duration::from_secs(10) => {
                thread::sleep(Duration::from_millis(10))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "{} did not finish within 10 seconds.",
                    program.display()
                ));
            }
        }
    };
    let stdout = out
        .join()
        .map_err(|_| "Output reader failed")?
        .map_err(|e| e.to_string())?;
    let stderr = err
        .join()
        .map_err(|_| "Error reader failed")?
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!("{} failed: {}", program.display(), stderr.trim()));
    }
    Ok(stdout)
}

fn args<'a>(values: &'a [&'a str]) -> Vec<&'a OsStr> {
    values.iter().map(OsStr::new).collect()
}

impl Backend {
    pub fn default_path() -> Result<PathBuf, String> {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|p| p.join("voxtype/config.toml"))
            .ok_or("Cannot resolve the configuration path; use --config.".into())
    }
    pub fn new(path: PathBuf) -> Self {
        let restart_allowed = Self::default_path().is_ok_and(|p| p == path);
        Self {
            path,
            voxtype: "voxtype".into(),
            systemctl: "systemctl".into(),
            restart_allowed,
        }
    }
    pub fn schema_at(&self, path: &Path) -> Result<Schema, String> {
        let output = command(
            &self.voxtype,
            &[
                OsStr::new("-c"),
                path.as_os_str(),
                OsStr::new("config"),
                OsStr::new("schema"),
                OsStr::new("--json"),
            ],
        )?;
        Schema::parse(
            serde_json::from_str(&output).map_err(|e| format!("Invalid schema JSON: {e}"))?,
        )
    }
    fn info(&self, kind: &str) -> Result<Json, String> {
        serde_json::from_str(&command(&self.voxtype, &args(&["info", kind, "--json"]))?)
            .map_err(|e| e.to_string())
    }
    pub fn read_source(&self) -> Result<(String, Option<Vec<u8>>), String> {
        let original = match fs::read(&self.path) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(format!("Cannot read {}: {e}", self.path.display())),
        };
        let source = String::from_utf8(original.clone().unwrap_or_default())
            .map_err(|_| "Configuration is not UTF-8".to_string())?;
        source
            .parse::<DocumentMut>()
            .map_err(|e| format!("Invalid configuration: {e}"))?;
        Ok((source, original))
    }
    pub fn load(&self) -> Result<Loaded, String> {
        let original = match fs::read(&self.path) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(format!("Cannot read {}: {e}", self.path.display())),
        };
        let source = String::from_utf8(original.clone().unwrap_or_default())
            .map_err(|_| "Configuration is not UTF-8")?;
        source
            .parse::<DocumentMut>()
            .map_err(|e| format!("Invalid configuration: {e}"))?;
        let mut temporary = NamedTempFile::new().map_err(|e| e.to_string())?;
        temporary
            .write_all(source.as_bytes())
            .map_err(|e| e.to_string())?;
        let schema = self.schema_at(temporary.path())?;
        let inventory = self.discover();
        Ok(Loaded {
            schema,
            source,
            original,
            inventory,
        })
    }
    pub fn discover(&self) -> Inventory {
        let mut inventory = Inventory::default();
        for kind in ["devices", "models", "engines", "styles", "variants"] {
            match self.info(kind) {
                Ok(data) => match kind {
                    "devices" => {
                        inventory.devices = data
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(|v| v["name"].as_str().map(str::to_owned))
                            .collect()
                    }
                    "models" => {
                        if let Some(engines) = data["engines"].as_object() {
                            for (engine, data) in engines {
                                inventory.models.insert(
                                    engine.clone(),
                                    data["models"]
                                        .as_array()
                                        .into_iter()
                                        .flatten()
                                        .filter(|v| v["installed"] == true)
                                        .filter_map(|v| v["name"].as_str().map(str::to_owned))
                                        .collect(),
                                );
                            }
                        }
                    }
                    "engines" => {
                        inventory.engines = data
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter(|v| v["compiled"] == true)
                            .filter_map(|v| v["name"].as_str().map(str::to_owned))
                            .collect()
                    }
                    "styles" => collect_names(&data, &mut inventory.styles),
                    "variants" => {
                        inventory.variants = format!(
                            "Active: {}\nRecommended: {}\nInstall: {}",
                            data["active_variant"].as_str().unwrap_or("unknown"),
                            data["recommendation"]["primary"]
                                .as_str()
                                .unwrap_or("unknown"),
                            data["install_kind"].as_str().unwrap_or("unknown")
                        )
                    }
                    _ => (),
                },
                Err(e) => inventory.warnings.push(format!("{kind}: {e}")),
            }
        }
        if let Some(home) = std::env::var_os("HOME") {
            let data = std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(home).join(".local/share"));
            if let Ok(entries) = fs::read_dir(data.join("voxtype/models")) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "bin") && path.is_file() {
                        inventory
                            .models
                            .entry("whisper".into())
                            .or_default()
                            .push(path.to_string_lossy().into_owned());
                    }
                }
            }
        }
        inventory.service = command(
            &self.systemctl,
            &args(&[
                "--user",
                "show",
                "voxtype.service",
                "--property=ActiveState",
                "--property=MainPID",
            ]),
        )
        .unwrap_or_else(|e| {
            inventory.warnings.push(e);
            "Unavailable".into()
        });
        inventory
    }
    pub fn save(&self, expected: Option<Vec<u8>>, source: String) -> Result<Saved, String> {
        self.save_validated(expected, source, |p| self.schema_at(p))
    }
    pub fn save_validated(
        &self,
        expected: Option<Vec<u8>>,
        source: String,
        validate: impl FnOnce(&Path) -> Result<Schema, String>,
    ) -> Result<Saved, String> {
        source
            .parse::<DocumentMut>()
            .map_err(|e| format!("Invalid TOML: {e}"))?;
        let parent = self
            .path
            .parent()
            .ok_or("Configuration has no parent directory")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let lock_path = parent.join(".voxtype-settings.lock");
        let _lock = Lock::acquire(lock_path)?;
        self.check_unchanged(expected.as_deref())?;
        let mut stage = tempfile::Builder::new()
            .prefix(".voxtype-settings-")
            .suffix(".toml")
            .tempfile_in(parent)
            .map_err(|e| e.to_string())?;
        stage
            .write_all(source.as_bytes())
            .map_err(|e| e.to_string())?;
        stage.as_file().sync_all().map_err(|e| e.to_string())?;
        // This invokes only the daemon's parser/schema command, never the daemon.
        let schema = validate(stage.path())?;
        self.check_unchanged(expected.as_deref())?;
        // Match the original file's mode so a 0644 config does not become
        // private (or a deliberately private one become readable) on save.
        #[cfg(unix)]
        if let Ok(metadata) = fs::metadata(&self.path) {
            use std::os::unix::fs::PermissionsExt;
            stage
                .as_file()
                .set_permissions(fs::Permissions::from_mode(metadata.permissions().mode()))
                .map_err(|e| e.to_string())?;
        }
        let backup = if let Some(bytes) = &expected {
            let mut backup = tempfile::Builder::new()
                .prefix("config.toml.backup-")
                .tempfile_in(parent)
                .map_err(|e| e.to_string())?;
            backup.write_all(bytes).map_err(|e| e.to_string())?;
            backup.as_file().sync_all().map_err(|e| e.to_string())?;
            Some(backup.keep().map_err(|e| e.to_string())?.1)
        } else {
            None
        };
        self.check_unchanged(expected.as_deref())?;
        stage
            .persist(&self.path)
            .map_err(|e| format!("Atomic save failed: {}", e.error))?;
        fs::File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|e| format!("Saved, but directory sync failed: {e}"))?;
        Ok(Saved {
            schema,
            source,
            backup,
        })
    }
    fn check_unchanged(&self, expected: Option<&[u8]>) -> Result<(), String> {
        match fs::symlink_metadata(&self.path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(
                    "Refusing to replace a symlink. Open its real target with --config.".into(),
                );
            }
            Ok(metadata) if !metadata.is_file() => {
                return Err("Configuration is not a regular file.".into());
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.to_string()),
            _ => (),
        }
        let current = match fs::read(&self.path) {
            Ok(b) => Some(b),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(e.to_string()),
        };
        if current.as_deref() != expected {
            return Err(
                "Configuration changed on disk. Reload before saving; your draft has been kept."
                    .into(),
            );
        }
        Ok(())
    }
    pub fn restart(&self) -> Result<String, String> {
        if !self.restart_allowed {
            return Err("Restart is disabled for an alternate configuration path.".into());
        }
        let status = command(&self.voxtype, &args(&["status", "--format", "json"]))?;
        let value: Json = serde_json::from_str(&status)
            .map_err(|_| "Cannot verify that Voxtype is idle; restart refused.")?;
        let state = value["state"]
            .as_str()
            .or_else(|| value["class"].as_str())
            .unwrap_or("");
        if state != "idle" {
            return Err("Voxtype is recording, processing, stopped, or its state is unknown. Restart refused.".into());
        }
        command(
            &self.systemctl,
            &args(&["--user", "restart", "voxtype.service"]),
        )?;
        Ok("Voxtype service restarted.".into())
    }
}

fn collect_names(data: &Json, names: &mut Vec<String>) {
    if let Some(list) = data.as_array() {
        for entry in list {
            if let Some(n) = entry["name"].as_str() {
                names.push(n.into());
            }
        }
    }
    if let Some(styles) = data.get("styles") {
        collect_names(styles, names);
    }
}

struct Lock(fs::File);
impl Lock {
    fn acquire(path: PathBuf) -> Result<Self, String> {
        use fs2::FileExt;
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|e| format!("Cannot open editor lock {}: {e}", path.display()))?;
        file.try_lock_exclusive().map_err(|e| {
            format!(
                "Cannot acquire editor lock {}: {e}. Another editor may be saving.",
                path.display()
            )
        })?;
        Ok(Self(file))
    }
}
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs2::FileExt::unlock(&self.0);
    }
}

pub fn patch_document(source: &str, changes: &[(String, Option<Json>)]) -> Result<String, String> {
    let mut document: DocumentMut = source.parse().map_err(|e| format!("Invalid TOML: {e}"))?;
    for (path, value) in changes {
        let segments: Vec<_> = path.split('.').collect();
        edit_table(document.as_table_mut(), &segments, value)?;
    }
    Ok(document.to_string())
}

fn edit_table(
    table: &mut dyn toml_edit::TableLike,
    path: &[&str],
    value: &Option<Json>,
) -> Result<(), String> {
    let (key, tail) = path.split_first().ok_or("Empty configuration key")?;
    if tail.is_empty() {
        if let Some(value) = value {
            let mut value = json_toml(value)?;
            if let Some(old) = table.get(key).and_then(Item::as_value) {
                *value.decor_mut() = old.decor().clone();
            }
            table.insert(key, Item::Value(value));
        } else {
            table.remove(key);
        }
        return Ok(());
    }
    if table.get(key).is_none() {
        if value.is_none() {
            return Ok(());
        }
        let mut nested = Table::new();
        nested.set_implicit(true);
        table.insert(key, Item::Table(nested));
    }
    let child = table
        .get_mut(key)
        .and_then(Item::as_table_like_mut)
        .ok_or_else(|| format!("{key} is not a table; refusing to discard it."))?;
    edit_table(child, tail, value)
}

fn json_toml(value: &Json) -> Result<Value, String> {
    Ok(match value {
        Json::String(s) => Value::from(s.as_str()),
        Json::Bool(b) => Value::from(*b),
        Json::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::from(i)
            } else {
                Value::from(n.as_f64().ok_or("Invalid number")?)
            }
        }
        Json::Array(values) => {
            let mut a = toml_edit::Array::new();
            for v in values {
                a.push(json_toml(v)?);
            }
            Value::Array(a)
        }
        _ => return Err("This value must be edited in the full TOML editor.".into()),
    })
}
