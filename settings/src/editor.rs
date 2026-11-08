use crate::{
    backend::{Backend, Inventory, Loaded, Saved, patch_document},
    schema::{self, Field, Schema},
    theme::Theme,
};
use rune::{Key, KeyEvent, TextEdit, Tx};
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{
        OnceLock,
        mpsc::{self, Receiver},
    },
    thread,
};

pub const SECTIONS: [&str; 13] = [
    "Overview",
    "Engine",
    "Hotkey",
    "Audio",
    "Output",
    "Text",
    "VAD",
    "Meeting",
    "Notifications",
    "OSD",
    "Status",
    "Advanced",
    "Config file",
];
pub static LAUNCH: OnceLock<(bool, Option<PathBuf>)> = OnceLock::new();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modal {
    Save,
    Reset,
    Reload,
    Restart,
    Quit,
    Help,
}

enum Completion {
    Load(Result<Loaded, String>),
    Save(Result<Saved, String>, bool),
    Restart(Result<String, String>),
}

pub struct Editor {
    pub demo: bool,
    pub backend: Option<Backend>,
    pub fields: Vec<Field>,
    pub inventory: Inventory,
    pub original: Option<Vec<u8>>,
    pub source: String,
    pub document: TextEdit,
    pub version: String,
    pub page: usize,
    pub search: TextEdit,
    pub all_engines: bool,
    pub open_select: Option<usize>,
    pub custom: Option<usize>,
    pub modal: Option<Modal>,
    pub notice: String,
    pub error: Option<String>,
    pub restart_needed: bool,
    pub theme: Theme,
    pub next_theme_check: f64,
    pub generation: u64,
    task: Option<Receiver<Completion>>,
}

impl Editor {
    pub fn init() -> Self {
        let (demo, path) = LAUNCH.get().cloned().unwrap_or_default();
        if demo || (rune::is_headless() && path.is_none()) {
            return Self::demo();
        }
        match path.map(Ok).unwrap_or_else(Backend::default_path) {
            Ok(path) => Self::live(Backend::new(path)),
            Err(error) => {
                let mut s = Self::empty(false);
                s.error = Some(error);
                s
            }
        }
    }
    fn empty(demo: bool) -> Self {
        Self {
            demo,
            backend: None,
            fields: Vec::new(),
            inventory: Inventory::default(),
            original: None,
            source: String::new(),
            document: TextEdit::new(""),
            version: String::new(),
            page: 0,
            search: TextEdit::new(""),
            all_engines: false,
            open_select: None,
            custom: None,
            modal: None,
            notice: "Loading configuration and installed capabilities…".into(),
            error: None,
            restart_needed: false,
            theme: Theme::for_app(),
            next_theme_check: 0.0,
            generation: 0,
            task: None,
        }
    }
    pub fn demo() -> Self {
        let mut s = Self::empty(true);
        s.theme = Theme::for_app();
        s.accept(
            Schema::demo(),
            include_str!("../fixtures/example.toml").into(),
        );
        s.inventory = Inventory {
            devices: vec!["default".into(), "Example USB microphone".into()],
            engines: vec!["whisper".into()],
            models: [("whisper".into(), vec!["base.en".into(), "small".into()])].into(),
            styles: vec!["default".into()],
            variants: "Active: whisper-vulkan (example)".into(),
            service: "Demo — no service access".into(),
            warnings: Vec::new(),
        };
        s.notice =
            "Demo data. Apply only changes this session; no files or services are touched.".into();
        s
    }
    pub fn live(backend: Backend) -> Self {
        let mut s = Self::empty(false);
        s.backend = Some(backend);
        s.load();
        s
    }
    pub fn accept(&mut self, schema: Schema, source: String) {
        self.version = schema.voxtype_version;
        self.fields = schema.keys.into_iter().map(Field::new).collect();
        self.original = Some(source.as_bytes().to_vec());
        self.document = TextEdit::new(&source);
        self.source = source;
        self.open_select = None;
        self.custom = None;
        self.generation += 1;
    }
    pub fn busy(&self) -> bool {
        self.task.is_some()
    }
    pub fn raw_dirty(&self) -> bool {
        self.document.text != self.source
    }
    pub fn dirty(&self) -> bool {
        self.raw_dirty() || self.fields.iter().any(Field::dirty)
    }
    pub fn changed_count(&self) -> usize {
        self.fields.iter().filter(|f| f.dirty()).count()
    }
    pub fn field_index(&self, key: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.spec.key == key)
    }
    pub fn engine(&self) -> String {
        self.field_index("engine")
            .map(|i| self.fields[i].edit.text.clone())
            .unwrap_or_else(|| "whisper".into())
    }
    pub fn navigate(&mut self, page: usize) {
        self.page = page.min(SECTIONS.len() - 1);
        self.open_select = None;
        self.search = TextEdit::new("");
    }
    pub fn visible(&self, field: &Field) -> bool {
        let query = self.search.text.to_lowercase();
        if !query.is_empty() {
            return format!(
                "{} {} {} {}",
                field.spec.label, field.spec.description, field.spec.key, field.spec.section
            )
            .to_lowercase()
            .contains(&query);
        }
        field.spec.section == SECTIONS[self.page]
            && (self.all_engines
                || field
                    .spec
                    .engine
                    .as_ref()
                    .is_none_or(|e| e == &self.engine()))
    }
    pub fn editable(&self, index: usize) -> bool {
        !self.busy() && !self.raw_dirty() && self.fields[index].spec.compiled
    }
    /// Labels and underlying values for a picker. Model paths get a friendly
    /// file-stem label; two files with the same stem get a numeric suffix.
    pub fn choice_pairs(&self, index: usize) -> Vec<(String, String)> {
        let f = &self.fields[index];
        let mut values = if f.spec.key == "engine" {
            self.inventory.engines.clone()
        } else {
            match f.spec.source.as_deref() {
                Some("devices") => self.inventory.devices.clone(),
                Some("models") => self
                    .inventory
                    .models
                    .get(f.spec.engine.as_deref().unwrap_or("whisper"))
                    .cloned()
                    .unwrap_or_default(),
                Some("styles") => self.inventory.styles.clone(),
                _ => f.spec.choices.clone(),
            }
        };
        if !f.edit.text.is_empty() && !values.contains(&f.edit.text) {
            values.insert(0, f.edit.text.clone());
        }
        let mut pairs: Vec<(String, String)> = Vec::new();
        for value in values {
            if pairs.iter().any(|(_, existing)| existing == &value) {
                continue;
            }
            let base = if schema::is_model_file_key(&f.spec.key) {
                schema::model_label(&value)
            } else {
                value.clone()
            };
            let mut label = base.clone();
            let mut n = 2;
            while pairs.iter().any(|(shown, _)| shown == &label) {
                label = format!("{base} ({n})");
                n += 1;
            }
            pairs.push((label, value));
        }
        pairs
    }
    pub fn choices(&self, index: usize) -> Vec<String> {
        self.choice_pairs(index)
            .into_iter()
            .map(|(label, _)| label)
            .collect()
    }
    pub fn choice_display(&self, index: usize, value: &str) -> String {
        self.choice_pairs(index)
            .into_iter()
            .find(|(_, candidate)| candidate == value)
            .map(|(label, _)| label)
            .unwrap_or_else(|| {
                if schema::is_model_file_key(&self.fields[index].spec.key) {
                    schema::model_label(value)
                } else {
                    value.to_owned()
                }
            })
    }
    pub fn choice_value(&self, index: usize, label: &str) -> String {
        self.choice_pairs(index)
            .into_iter()
            .find(|(shown, _)| shown == label)
            .map(|(_, value)| value)
            .unwrap_or_else(|| label.to_owned())
    }
    pub fn model_warning(&self) -> Option<String> {
        let engine = self.engine();
        let index = self.field_index(&format!("{engine}.model"))?;
        let model = &self.fields[index].edit.text;
        if model.is_empty() {
            return Some("No model is selected for this engine.".into());
        }
        if PathBuf::from(model).is_absolute() {
            return (!PathBuf::from(model).is_file())
                .then(|| format!("Model file not found: {model}"));
        }
        let installed = self
            .inventory
            .models
            .get(&engine)
            .is_some_and(|m| m.iter().any(|name| name == model));
        (!installed).then(|| format!("{engine} model \"{model}\" is not installed. Voxtype will not be able to transcribe until it is downloaded with `voxtype setup model`."))
    }
    pub fn device_warning(&self) -> Option<String> {
        let index = self.field_index("audio.device")?;
        let device = &self.fields[index].edit.text;
        (device != "default" && !self.inventory.devices.iter().any(|d| d == device))
            .then(|| format!("Audio device \"{device}\" is not in the current device list."))
    }
    pub fn validation(&self) -> Option<String> {
        if self.raw_dirty() {
            if self.fields.iter().any(Field::dirty) {
                return Some("Apply or reset field edits before editing the full document.".into());
            }
            return self
                .document
                .text
                .parse::<toml_edit::DocumentMut>()
                .err()
                .map(|e| format!("Invalid TOML: {e}"));
        }
        self.fields
            .iter()
            .find_map(|f| f.error().map(|e| format!("{}: {e}", f.spec.label)))
    }
    pub fn candidate(&self) -> Result<String, String> {
        if let Some(error) = self.validation() {
            return Err(error);
        }
        if self.raw_dirty() {
            return Ok(self.document.text.clone());
        }
        let changes: Result<Vec<_>, String> = self
            .fields
            .iter()
            .filter(|f| f.dirty())
            .map(|f| f.value().map(|v| (f.spec.key.clone(), v)))
            .collect();
        patch_document(&self.source, &changes?)
    }
    pub fn review(&self) -> String {
        if self.raw_dirty() {
            return "Full TOML document edited. Comments and all fields will be saved as shown in Config file. This can include commands, paths and credentials. Review carefully.".into();
        }
        self.fields
            .iter()
            .filter(|f| f.dirty())
            .map(|f| {
                if f.spec.key.contains("api_key") {
                    format!("{}: [secret changed]", f.spec.key)
                } else if f.clear {
                    format!("{}: remove override (use built-in default)", f.spec.key)
                } else {
                    format!(
                        "{}: {} → {}",
                        f.spec.key,
                        if f.original.is_empty() {
                            "(unset)"
                        } else {
                            &f.original
                        },
                        f.edit.text
                    )
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    pub fn reset(&mut self) {
        for field in &mut self.fields {
            field.set(&field.original.clone());
            field.clear = false;
        }
        self.document = TextEdit::new(&self.source);
        self.modal = None;
        self.open_select = None;
        self.error = None;
    }
    pub fn open_raw(&mut self) {
        let Some(backend) = &self.backend else {
            return;
        };
        match backend.read_source() {
            Ok((source, original)) => {
                self.accept(Schema::demo(), source);
                self.fields.clear();
                self.original = original;
                self.page = 12;
                self.error = None;
                self.notice = "Editing the full file. Fix the error, then review and apply. Voxtype validates the result before saving.".into();
            }
            Err(error) => self.error = Some(error),
        }
    }
    pub fn load(&mut self) {
        if self.busy() {
            return;
        }
        let Some(backend) = self.backend.clone() else {
            return;
        };
        let (sender, receiver) = mpsc::channel();
        self.task = Some(receiver);
        self.modal = None;
        self.error = None;
        self.notice = "Reading configuration and discovering installed models/devices…".into();
        thread::spawn(move || {
            let _ = sender.send(Completion::Load(backend.load()));
        });
    }
    pub fn save(&mut self) {
        if self.busy() || !self.dirty() {
            return;
        }
        let source = match self.candidate() {
            Ok(s) => s,
            Err(e) => {
                self.error = Some(e);
                return;
            }
        };
        let restart = self.raw_dirty()
            || self
                .fields
                .iter()
                .any(|f| f.dirty() && f.spec.restart_required);
        self.modal = None;
        if self.demo {
            let mut schema = Schema::demo();
            for (spec, field) in schema.keys.iter_mut().zip(&self.fields) {
                if field.dirty() {
                    let value = field.value().unwrap_or(None);
                    spec.file_value = value.clone().unwrap_or(Value::Null);
                    spec.value = value.unwrap_or(Value::Null);
                }
            }
            self.accept(schema, source);
            self.notice =
                "Demo applied in memory. No configuration was saved and no service was restarted."
                    .into();
            return;
        }
        let Some(backend) = self.backend.clone() else {
            return;
        };
        let original = self.original.clone();
        let (sender, receiver) = mpsc::channel();
        self.task = Some(receiver);
        self.notice = "Validating with Voxtype, backing up and saving…".into();
        self.error = None;
        thread::spawn(move || {
            let _ = sender.send(Completion::Save(backend.save(original, source), restart));
        });
    }
    pub fn restart(&mut self) {
        self.modal = None;
        if self.busy() || self.demo || self.dirty() {
            return;
        }
        let Some(backend) = self.backend.clone() else {
            return;
        };
        let (sender, receiver) = mpsc::channel();
        self.task = Some(receiver);
        self.notice = "Checking that Voxtype is idle before restarting…".into();
        self.error = None;
        thread::spawn(move || {
            let _ = sender.send(Completion::Restart(backend.restart()));
        });
    }
    pub fn poll(&mut self, time: f64, tx: &mut Tx) -> bool {
        let mut changed = false;
        if time >= self.next_theme_check {
            changed |= self.theme.refresh();
            self.next_theme_check = time + 1.0;
        }
        if let Some(task) = &self.task {
            match task.try_recv() {
                Ok(completion) => {
                    self.task = None;
                    changed = true;
                    match completion {
                        Completion::Load(Ok(loaded)) => {
                            self.accept(loaded.schema, loaded.source);
                            self.original = loaded.original;
                            self.inventory = loaded.inventory;
                            self.notice = "Loaded from disk. Apply validates and backs up before saving. Restart is separate.".into();
                        }
                        Completion::Save(Ok(saved), restart) => {
                            self.accept(saved.schema, saved.source);
                            self.restart_needed |= restart;
                            self.notice = format!(
                                "Saved. {}{}",
                                saved
                                    .backup
                                    .map(|p| format!("Backup: {}. ", p.display()))
                                    .unwrap_or_default(),
                                if self.restart_needed {
                                    "Restart required; the running daemon has not been changed."
                                } else {
                                    "No daemon restart needed."
                                }
                            );
                        }
                        Completion::Restart(Ok(message)) => {
                            self.notice = message;
                            self.restart_needed = false;
                        }
                        Completion::Load(Err(e))
                        | Completion::Save(Err(e), _)
                        | Completion::Restart(Err(e)) => {
                            self.error = Some(e);
                            self.notice = "Operation failed. Your draft has been kept.".into();
                        }
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.task = None;
                    self.error = Some("Background operation failed unexpectedly.".into());
                    changed = true;
                }
                _ => (),
            }
        }
        tx.wake_after(std::time::Duration::from_millis(if self.busy() {
            100
        } else {
            1000
        }));
        changed
    }
}

pub fn on_key(s: &mut Editor, event: &KeyEvent, tx: &mut Tx) -> bool {
    if event.is_command('q') {
        if s.busy() {
            s.error = Some("Wait for the current operation to finish before closing.".into());
        } else if s.dirty() {
            s.modal = Some(Modal::Quit);
        } else {
            tx.quit();
        }
        return true;
    }
    if event.is_command('s') || event.mods.cmd && matches!(event.key, Key::Enter) {
        if s.dirty() && !s.busy() && s.validation().is_none() {
            s.modal = Some(Modal::Save);
        }
        return true;
    }
    if event.is_command('r') && !s.busy() {
        s.modal = Some(Modal::Reload);
        return true;
    }
    if matches!(&event.key, Key::Text(t) if t == "?") {
        s.modal = Some(Modal::Help);
        return true;
    }
    if matches!(event.key, Key::Escape) {
        s.open_select = None;
        s.custom = None;
        s.modal = None;
        return true;
    }
    false
}

pub fn summary_value(s: &Editor, key: &str) -> String {
    s.field_index(key)
        .map(|i| {
            let value = &s.fields[i].edit.text;
            if value.is_empty() {
                "(unset)".into()
            } else if schema::is_model_file_key(key) {
                schema::model_label(value)
            } else {
                value.clone()
            }
        })
        .unwrap_or_else(|| "Unavailable".into())
}
