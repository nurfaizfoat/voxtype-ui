use super::*;
use backend::{Backend, patch_document};
use editor::{Editor, Modal, SECTIONS};
use rune::{Env, Key, KeyEvent, Mods, Runtime, TextEdit, Tx};
use schema::Schema;
use std::path::PathBuf;

fn runtime(size: (f32, f32), scale: f32) -> Runtime<Editor> {
    runtime_with_text_scale(size, scale, 1.0)
}

fn runtime_with_text_scale(size: (f32, f32), scale: f32, text_scale: f32) -> Runtime<Editor> {
    let mut editor = Editor::demo();
    editor.theme.text_scale = crate::text_scale::DesktopTextScale::fixed(text_scale);
    editor.navigate(0);
    let mut rt = Runtime::with_text(
        editor,
        editor_ui::view,
        size,
        Env::new(true, scale),
        rune::text::TextSystem::new(&[], theme::FONT_FAMILIES),
    );
    rt.poll = Some(poll);
    rt.on_key = Some(on_key);
    rt.frame(0.0);
    rt
}

#[test]
fn desktop_font_scale_changes_type_without_rescaling_the_window_or_draft() {
    let mut rt = runtime_with_text_scale((980.0, 720.0), 1.25, 1.0);
    let index = rt.state.field_index("whisper.model").unwrap();
    rt.state.fields[index].set("/models/ggml-custom.bin");
    let candidate = rt.state.candidate().unwrap();
    let title_height = |rt: &Runtime<Editor>| {
        rt.elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some("page-title"))
            .unwrap()
            .rect[3]
    };
    let before = title_height(&rt);
    rt.state.theme.text_scale = crate::text_scale::DesktopTextScale::fixed(0.8182);
    rt.invalidate();
    for _ in 0..24 {
        rt.frame(1.0 / 60.0);
    }
    assert!(title_height(&rt) < before);
    assert!((rt.state.theme.kit().ty.ui.size - 11.0).abs() < 0.001);
    assert_eq!(rt.state.candidate().unwrap(), candidate);
    let window = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("settings-window"))
        .unwrap();
    assert_eq!(window.rect, [0.0, 0.0, 980.0, 720.0]);
}

#[test]
fn desktop_scaled_pages_and_dialogs_stay_inside_the_viewport() {
    for text_scale in [0.8182, 1.0, 1.25] {
        let mut rt = runtime_with_text_scale((840.0, 620.0), 1.25, text_scale);
        for page in 0..SECTIONS.len() {
            rt.state.navigate(page);
            rt.invalidate();
            rt.frame(0.0);
            for key in ["page-title", "apply", "reset", "search"] {
                let el = rt
                    .elements()
                    .into_iter()
                    .find(|e| e.key.as_deref() == Some(key))
                    .unwrap();
                let [x, y, w, h] = el.rect;
                assert!(
                    x >= 0.0 && y >= 0.0 && x + w <= 840.1 && y + h <= 620.1,
                    "{text_scale}/{page}/{key}: {:?}",
                    el.rect
                );
            }
        }
        rt.state.modal = Some(Modal::Help);
        rt.invalidate();
        rt.frame(0.0);
        let dialog = rt
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some("confirmation-dialog"))
            .unwrap();
        assert!(dialog.rect[1] >= 0.0 && dialog.rect[1] + dialog.rect[3] <= 620.1);
    }
}

fn tap(rt: &mut Runtime<Editor>, key: &str) {
    for _ in 0..24 {
        rt.frame(1.0 / 60.0);
    }
    let (x, y) = rt
        .find(key)
        .unwrap_or_else(|| panic!("Missing interactive element: {key}"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    rt.frame(0.0);
}

fn command(rt: &mut Runtime<Editor>, key: char) {
    rt.key(KeyEvent {
        key: Key::Command(key),
        mods: Mods {
            cmd: true,
            ctrl: true,
            ..Mods::default()
        },
    });
    rt.frame(0.0);
}

fn editor_with(source: &str) -> Editor {
    let mut editor = Editor::demo();
    editor.accept(Schema::demo(), source.into());
    editor
}

#[test]
fn demo_covers_every_installed_schema_section_and_key() {
    let schema = Schema::demo();
    assert_eq!(schema.schema_version, 1);
    assert!(schema.keys.len() >= 187, "{} keys", schema.keys.len());
    let mut sections: Vec<_> = schema.keys.iter().map(|k| k.section.as_str()).collect();
    sections.sort_unstable();
    sections.dedup();
    for section in &sections {
        assert!(SECTIONS.contains(section), "missing page for {section}");
    }
    for key in [
        "engine",
        "whisper.model",
        "audio.device",
        "output.mode",
        "osd.frontend",
        "status.icon_theme",
    ] {
        assert!(schema.keys.iter().any(|k| k.key == key), "{key}");
    }
}

#[test]
fn editable_fields_round_trip_typed_values_and_arrays() {
    let mut s = editor_with("engine = \"whisper\"\n[audio]\nmax_duration_secs = 60\n");
    let i = s.field_index("audio.max_duration_secs").unwrap();
    s.fields[i].set("90");
    assert_eq!(
        s.candidate().unwrap(),
        "engine = \"whisper\"\n[audio]\nmax_duration_secs = 90\n"
    );

    s.fields[i].set("0");
    assert!(s.validation().is_some());
    s.fields[i].set("90");
    s.fields[i].reset_override();
    let candidate = s.candidate().unwrap();
    assert!(
        !candidate.contains("max_duration_secs"),
        "Default removes the override: {candidate}"
    );
    assert!(s.fields[i].clear);

    let lang = s.field_index("whisper.language").unwrap();
    s.fields[i].set("");
    assert!(
        s.candidate().unwrap().contains("max_duration_secs = 60")
            || !s.candidate().unwrap().contains("max_duration_secs")
    );
    s.fields[i].set("90");
    s.fields[lang].set("en,fr");
    let saved = s.candidate().unwrap();
    assert!(
        saved.contains("language = [\"en\", \"fr\"]")
            || saved.contains("language = [\"en\",\"fr\"]"),
        "{saved}"
    );
}

#[test]
fn patch_preserves_comments_unrelated_fields_and_rejects_table_collisions() {
    let source = "# keep me\n[audio]\n# device comment\ndevice = \"default\"\n\n[text.replacements]\n\"vox type\" = \"Voxtype\"\n";
    let patched = patch_document(
        source,
        &[(
            "audio.max_duration_secs".into(),
            Some(serde_json::json!(90)),
        )],
    )
    .unwrap();
    assert!(patched.contains("# keep me") && patched.contains("# device comment"));
    assert!(patched.contains("max_duration_secs = 90"));
    assert!(patched.contains("\"vox type\" = \"Voxtype\""));

    let err = patch_document(
        "audio = \"not a table\"\n",
        &[("audio.device".into(), Some(serde_json::json!("default")))],
    )
    .unwrap_err();
    assert!(err.contains("not a table"));
}

#[test]
fn raw_document_edits_are_validated_and_lock_field_edits() {
    let mut s = editor_with("[audio]\ndevice = \"default\"\n");
    s.navigate(12);
    s.document = TextEdit::new("[audio]\ndevice = \"bad\\n");
    assert!(s.validation().unwrap().contains("Invalid TOML"));
    s.document = TextEdit::new("[audio]\ndevice = \"usb\"\n");
    assert_eq!(s.candidate().unwrap(), "[audio]\ndevice = \"usb\"\n");
    let i = s.field_index("audio.max_duration_secs").unwrap();
    s.fields[i].set("90");
    assert!(s.validation().unwrap().contains("full document"));
}

#[test]
fn demo_apply_is_memory_only_and_reset_restores_loaded_values() {
    let mut s = editor_with("[audio]\nmax_duration_secs = 60\n");
    let i = s.field_index("audio.max_duration_secs").unwrap();
    s.fields[i].set("90");
    assert!(s.dirty());
    s.modal = Some(Modal::Save);
    s.save();
    assert!(!s.dirty());
    assert!(s.notice.contains("Demo applied in memory"));
    s.fields[i].set("120");
    s.reset();
    assert!(!s.dirty());
    assert_eq!(s.fields[i].edit.text, "90");
}

#[test]
fn save_validator_backup_atomicity_and_races_are_safe() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "original = true\n").unwrap();
    let backend = Backend::new(path.clone());
    let original = std::fs::read(&path).unwrap();
    let saved = backend
        .save_validated(Some(original.clone()), "edited = true\n".into(), |p| {
            assert_eq!(std::fs::read_to_string(p).unwrap(), "edited = true\n");
            Ok(Schema::demo())
        })
        .unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "edited = true\n");
    let backup = saved.backup.unwrap();
    assert_eq!(std::fs::read(&backup).unwrap(), original);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_ne!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0
        );
    }

    let err = backend
        .save_validated(Some(original), "stale = true\n".into(), |_| {
            Ok(Schema::demo())
        })
        .unwrap_err();
    assert!(err.contains("changed on disk"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "edited = true\n");

    let before = std::fs::read_to_string(&path).unwrap();
    let err = backend
        .save_validated(
            Some(before.as_bytes().to_vec()),
            "invalid = true\n".into(),
            |_| Err("schema rejected".into()),
        )
        .unwrap_err();
    assert!(err.contains("schema rejected"));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn save_refuses_symlink_targets() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real.toml");
    let link = dir.path().join("config.toml");
    std::fs::write(&real, "a = 1\n").unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();
    let backend = Backend::new(link);
    assert!(
        backend
            .save_validated(Some(b"a = 1\n".to_vec()), "a = 2\n".into(), |_| Ok(
                Schema::demo()
            ))
            .unwrap_err()
            .contains("symlink")
    );
}

#[test]
fn all_pages_work_by_pointer_keyboard_and_search_filters_fields() {
    let mut rt = runtime((980.0, 720.0), 1.25);
    for (page, section) in SECTIONS.iter().enumerate() {
        tap(&mut rt, &format!("nav-{page}"));
        assert_eq!(rt.state.page, page);
        let title = rt
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some("page-title"))
            .unwrap();
        assert_eq!(
            title.text.as_deref(),
            Some(if page == 12 { "Config file" } else { section })
        );
    }
    for page in 0..SECTIONS.len() {
        command(&mut rt, char::from(b'1' + (page % 10) as u8));
    }
    rt.state.navigate(0);
    rt.state.search = TextEdit::new("buffer");
    rt.invalidate();
    for _ in 0..24 {
        rt.frame(1.0 / 60.0);
    }
    let elements = rt.elements();
    let titles: Vec<_> = elements
        .iter()
        .filter(|e| e.key.as_deref() == Some("page-title"))
        .filter_map(|e| e.text.clone())
        .collect();
    assert!(titles.iter().any(|t| t == "Search results"), "{titles:?}");
    assert!(
        elements
            .iter()
            .any(|e| e.key.as_deref() == Some("setting-streaming.max_buffer_secs"))
    );
    assert!(
        !elements
            .iter()
            .any(|e| e.key.as_deref() == Some("setting-audio.max_duration_secs")),
        "unrelated settings should be filtered out"
    );
}

#[test]
fn edited_and_invalid_scenes_show_validation_and_apply_state() {
    let mut rt = runtime((980.0, 720.0), 1.25);
    rt.state.navigate(4);
    let i = rt.state.field_index("output.type_delay_ms").unwrap();
    rt.state.fields[i].set("25");
    rt.invalidate();
    for _ in 0..24 {
        rt.frame(1.0 / 60.0);
    }
    assert!(rt.find("apply").is_some());
    rt.state.navigate(3);
    let i = rt.state.field_index("audio.max_duration_secs").unwrap();
    rt.state.fields[i].set("0");
    rt.invalidate();
    for _ in 0..24 {
        rt.frame(1.0 / 60.0);
    }
    let status = rt
        .elements()
        .into_iter()
        .find(|e| e.key.as_deref() == Some("editor-status"))
        .unwrap();
    assert!(
        status.text.as_deref().unwrap().contains("between"),
        "{:?}",
        status.text
    );
}

#[test]
fn layout_stays_inside_minimum_and_fractional_viewports() {
    for size in [
        (840.0, 620.0),
        (980.0, 720.0),
        (1100.0, 720.0),
        (1440.0, 900.0),
    ] {
        for scale in [1.0, 1.25, 2.0] {
            let mut rt = runtime(size, scale);
            for page in 0..SECTIONS.len() {
                rt.state.navigate(page);
                rt.frame(0.0);
                for key in ["page-title", "apply", "reset", "search"] {
                    let element = rt
                        .elements()
                        .into_iter()
                        .find(|e| e.key.as_deref() == Some(key))
                        .unwrap();
                    let [x, y, w, h] = element.rect;
                    assert!(
                        w > 0.0
                            && h > 0.0
                            && x >= -0.1
                            && y >= -0.1
                            && x + w <= size.0 + 0.1
                            && y + h <= size.1 + 0.1,
                        "page {page}/{key} outside {size:?}@{scale}: {:?}",
                        element.rect
                    );
                }
            }
        }
    }
}

#[test]
fn setting_copy_reflows_without_crossing_the_control_column() {
    for width in [840.0, 980.0, 1100.0] {
        for scale in [1.0, 1.25, 2.0] {
            let mut rt = runtime((width, 720.0), scale);
            rt.state.navigate(1);
            let i = rt.state.field_index("whisper.model").unwrap();
            rt.state.fields[i].spec.label =
                "Model for multilingual transcription and custom local recordings".into();
            rt.state.fields[i].spec.description = "Choose an installed model, or browse for a custom model file. The full path stays in the configuration while the filename is shown here.".into();
            rt.invalidate();
            for _ in 0..3 {
                rt.frame(0.0);
            }
            let elements = rt.elements();
            let find = |key: &str| {
                elements
                    .iter()
                    .find(|e| e.key.as_deref() == Some(key))
                    .unwrap()
            };
            let control = find(&format!("choice-{i}-trigger"));
            let mut previous_bottom = 0.0;
            for key in [
                "label-whisper.model",
                "description-whisper.model",
                "key-whisper.model",
            ] {
                let copy = find(key);
                let [x, y, w, h] = copy.rect;
                assert!(w > 0.0 && h > 0.0, "{key} has no area");
                assert!(
                    x + w <= control.rect[0] - 16.0,
                    "{key} overlaps control at {width}@{scale}"
                );
                assert!(y >= previous_bottom, "{key} overlaps preceding text");
                previous_bottom = y + h;
            }
            let browse = find(&format!("browse-{i}"));
            let custom = find(&format!("custom-{i}"));
            let default = find(&format!("default-{i}"));
            assert!((browse.rect[1] - custom.rect[1]).abs() < 0.1);
            assert!((custom.rect[1] - default.rect[1]).abs() < 0.1);
            assert!(default.rect[0] + default.rect[2] <= width - 24.0);
        }
    }
}

#[test]
fn typography_in_dialogs_stays_inside_the_minimum_viewport() {
    let mut rt = runtime((840.0, 620.0), 1.25);
    for modal in [
        Modal::Save,
        Modal::Reset,
        Modal::Reload,
        Modal::Restart,
        Modal::Quit,
        Modal::Help,
    ] {
        rt.state.modal = Some(modal);
        rt.invalidate();
        for _ in 0..3 {
            rt.frame(0.0);
        }
        let elements = rt.elements();
        for key in ["confirmation-dialog", "confirm", "cancel"] {
            let el = elements
                .iter()
                .find(|e| e.key.as_deref() == Some(key))
                .unwrap();
            let [x, y, w, h] = el.rect;
            assert!(
                x >= 0.0 && y >= 0.0 && x + w <= 840.1 && y + h <= 620.1,
                "{modal:?}: {key} {:?}",
                el.rect
            );
        }
    }
}

#[test]
fn shortcut_descriptions_have_a_shared_column_instead_of_manual_spaces() {
    let mut rt = runtime((840.0, 620.0), 1.25);
    rt.state.modal = Some(Modal::Help);
    rt.invalidate();
    rt.frame(0.0);
    let elements = rt.elements();
    let mut description_x = None;
    for i in 0..6 {
        let key = format!("shortcut-description-{i}");
        let el = elements
            .iter()
            .find(|e| e.key.as_deref() == Some(&key))
            .unwrap();
        let x = *description_x.get_or_insert(el.rect[0]);
        assert!((el.rect[0] - x).abs() < 0.1);
        assert!(el.rect[2] > 0.0 && el.rect[0] + el.rect[2] < 840.0);
    }
}

#[test]
fn unsaved_close_asks_and_can_cancel() {
    let mut rt = runtime((980.0, 720.0), 1.25);
    let i = rt.state.field_index("output.type_delay_ms").unwrap();
    rt.state.fields[i].set("25");
    command(&mut rt, 'q');
    assert_eq!(rt.state.modal, Some(Modal::Quit));
    tap(&mut rt, "cancel");
    assert_eq!(rt.state.modal, None);
    assert!(rt.state.dirty());
}

#[test]
fn keyboard_help_and_shortcuts_work() {
    let mut rt = runtime((980.0, 720.0), 1.25);
    rt.key(KeyEvent {
        key: Key::Text("?".into()),
        mods: Mods::default(),
    });
    rt.frame(0.0);
    assert_eq!(rt.state.modal, Some(Modal::Help));
    tap(&mut rt, "confirm");
    assert_eq!(rt.state.modal, None);
    rt.state.navigate(3);
    command(&mut rt, 'r');
    assert_eq!(rt.state.modal, Some(Modal::Reload));
}

#[test]
fn interface_uses_monospace_metrics() {
    fn probe(_: &(), _: &mut rune::Cx<()>) -> rune::El {
        rune::row()
            .child(
                rune::text("iiii")
                    .key("narrow")
                    .font(rune::TextStyle::new(13.0, 400.0)),
            )
            .child(
                rune::text("WWWW")
                    .key("wide")
                    .font(rune::TextStyle::new(13.0, 400.0)),
            )
    }
    let mut probe = Runtime::with_text(
        (),
        probe,
        (400.0, 100.0),
        Env::new(true, 1.25),
        rune::text::TextSystem::new(&[], theme::FONT_FAMILIES),
    );
    probe.frame(0.0);
    let widths: Vec<_> = probe
        .elements()
        .into_iter()
        .filter(|e| matches!(e.key.as_deref(), Some("narrow" | "wide")))
        .map(|e| e.rect[2])
        .collect();
    assert_eq!(widths.len(), 2);
    assert!(
        (widths[0] - widths[1]).abs() < 0.1,
        "Expected monospace: {widths:?}"
    );
}

#[test]
fn polling_updates_theme_without_changing_draft() {
    let path = std::env::temp_dir().join(format!("voxtype-poll-{}.toml", std::process::id()));
    let source = include_str!("../fixtures/forest.toml");
    std::fs::write(&path, source).unwrap();
    let mut s = editor_with("[audio]\ndevice = \"default\"\n");
    s.theme = theme::Theme::from_path(path.clone());
    let i = s.field_index("audio.max_duration_secs").unwrap();
    s.fields[i].set("90");
    let draft = s.fields[i].edit.text.clone();
    let mut tx = Tx::default();
    assert!(!poll(&mut s, 0.0, &mut tx));
    std::fs::write(&path, source.replace("#6e9cf7", "#ff6688")).unwrap();
    assert!(!poll(&mut s, 0.5, &mut tx));
    assert!(poll(&mut s, 1.0, &mut tx));
    assert_eq!(s.theme.palette.accent.hex(), "#ff6688ff");
    assert_eq!(s.fields[i].edit.text, draft);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn alternate_paths_cannot_restart_the_service() {
    let backend = Backend {
        path: PathBuf::from("/tmp/example.toml"),
        voxtype: "voxtype".into(),
        systemctl: "systemctl".into(),
        restart_allowed: false,
    };
    assert!(backend.restart().unwrap_err().contains("disabled"));
}

/// Manual integration check against the installed daemon. Ignored by default
/// because Voxtype's version and model inventory are machine-specific, and
/// because it spawns the CLI. It never touches the user's real config.
#[test]
#[ignore]
fn installed_schema_and_inventory_load_from_a_temporary_copy() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, include_str!("../fixtures/example.toml")).unwrap();
    let loaded = Backend::new(path.clone()).load().unwrap();
    assert!(loaded.schema.keys.len() >= 100);
    assert!(!loaded.inventory.devices.is_empty());
    assert_eq!(loaded.source, std::fs::read_to_string(&path).unwrap());
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        include_str!("../fixtures/example.toml")
    );
}

#[test]
#[ignore]
fn installed_backend_saves_a_temporary_copy_with_a_backup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(&path, include_str!("../fixtures/example.toml")).unwrap();
    let backend = Backend::new(path.clone());
    let original = std::fs::read(&path).unwrap();
    let changed = include_str!("../fixtures/example.toml")
        .replace("max_duration_secs = 60", "max_duration_secs = 90");
    let saved = backend.save(Some(original), changed).unwrap();
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.contains("max_duration_secs = 90"));
    assert!(
        written.contains("# Made-up configuration"),
        "comments must survive a real save"
    );
    assert!(saved.backup.as_ref().is_some_and(|p| p.exists()));
}

#[test]
fn missing_model_and_device_are_reported_without_blocking_edits() {
    let mut s = Editor::demo();
    assert!(s.model_warning().is_none());
    let model = s.field_index("whisper.model").unwrap();
    s.fields[model].set("not-installed");
    assert!(s.model_warning().unwrap().contains("not installed"));
    let device = s.field_index("audio.device").unwrap();
    s.fields[device].set("Missing microphone");
    assert!(
        s.device_warning()
            .unwrap()
            .contains("not in the current device list")
    );
    assert!(s.validation().is_none(), "warnings do not block saving");
}

#[test]
fn model_paths_display_a_friendly_name_while_saving_the_full_path() {
    assert_eq!(
        schema::model_label("/home/user/models/ggml-malaysian-whisper-small-v2.bin"),
        "malaysian-whisper-small-v2"
    );
    assert_eq!(schema::model_label("base.en"), "base.en");
    assert_eq!(schema::model_label("/models/ggml-base.en.bin"), "base.en");

    let mut s = Editor::demo();
    let i = s.field_index("whisper.model").unwrap();
    s.fields[i].set("/tmp/ggml-browsed-model.bin");
    assert_eq!(
        s.choice_display(i, "/tmp/ggml-browsed-model.bin"),
        "browsed-model"
    );
    assert_eq!(
        s.choice_value(i, "browsed-model"),
        "/tmp/ggml-browsed-model.bin"
    );
    assert!(
        s.candidate()
            .unwrap()
            .contains("/tmp/ggml-browsed-model.bin")
    );
    assert!(!s.candidate().unwrap().contains("Model = \"browsed-model\""));
}

#[test]
fn browse_button_resolves_a_model_file_and_shows_its_name() {
    let mut rt = runtime((980.0, 720.0), 1.25);
    rt.state.navigate(1);
    rt.invalidate();
    for _ in 0..24 {
        rt.frame(1.0 / 60.0);
    }
    let index = rt.state.field_index("whisper.model").unwrap();
    tap(&mut rt, &format!("browse-{index}"));
    assert!(rt.file_dialog().is_some(), "Browse must open a file dialog");
    rt.resolve_file_dialog(Some(PathBuf::from("/tmp/ggml-picked-model.bin")))
        .unwrap();
    for _ in 0..3 {
        rt.frame(1.0 / 60.0);
    }
    assert_eq!(
        rt.state.fields[index].edit.text,
        "/tmp/ggml-picked-model.bin"
    );
    assert_eq!(
        rt.state.choice_display(index, "/tmp/ggml-picked-model.bin"),
        "picked-model"
    );
    assert!(rt.state.custom.is_none());
}
