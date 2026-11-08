use crate::{
    editor::{Editor, Modal, SECTIONS, summary_value},
    icons, schema,
};
use rune::{Color, Cx, El, FileDialogOptions, ScrollSpec, Tx, col, el, icon, row, text};
use rune_kit::{
    Button, ButtonVariant, Kit, ScrollBar, Select, SelectAppearance, TextArea, TextInput,
};

fn caption(value: impl Into<String>, k: &Kit) -> El {
    text(value).font(k.ty.small).color(k.c.text_muted).wrap()
}
fn rule(k: &Kit) -> El {
    el().h(1.0).w_full().fill(k.c.border_muted).a11y_hidden()
}
fn button(
    label: &str,
    enabled: bool,
    primary: bool,
    k: &Kit,
    handler: impl Fn(&mut Editor) + 'static,
) -> El {
    let mut style = k.clone();
    if primary {
        style.c.text_base = k.c.text_accent;
        style.c.border_muted = k.c.text_accent;
    }
    Button::new(label)
        .variant(ButtonVariant::Outline)
        .disabled(!enabled)
        .on_press(handler)
        .build(&style)
        .fill(if primary {
            k.c.bg_accent.alpha(0.10)
        } else {
            Color::TRANSPARENT
        })
}
fn panel(body: El, k: &Kit) -> El {
    col()
        .w_full()
        .radius(k.radius.xl)
        .border(1.0, k.c.border_base)
        .fill(k.c.layer_01)
        .padding(18.0)
        .gap(12.0)
        .child(body)
}

fn navigation(s: &Editor, k: &Kit) -> El {
    let mut items = col().w_full().padding(10.0).gap(3.0);
    for (page, section) in SECTIONS.iter().enumerate() {
        let count = s
            .fields
            .iter()
            .filter(|f| f.spec.section == *section)
            .count();
        items = items.child(
            row()
                .key(format!("nav-{page}"))
                .role("button")
                .label(*section)
                .focusable(true)
                .focus_ring(2.0, -2.0, k.c.border_focus)
                .w_full()
                .h(36.0)
                .px(10.0)
                .gap(9.0)
                .center_y()
                .radius(k.radius.lg)
                .fill(if s.page == page {
                    k.c.bg_accent.alpha(0.18)
                } else {
                    Color::TRANSPARENT
                })
                .hover(k.c.hover)
                .on_press(move |s: &mut Editor| s.navigate(page))
                .child(
                    icon(
                        match page {
                            1 => &icons::SPEECH,
                            3 => &icons::AUDIO,
                            4 => &icons::OUTPUT,
                            9 => &icons::INDICATOR,
                            _ => &icons::OVERVIEW,
                        },
                        16.0,
                    )
                    .color(k.c.icon_accent)
                    .a11y_hidden(),
                )
                .child(text(*section).font(k.ty.ui).color(k.c.text_base).grow(1.0))
                .child(caption(
                    if count == 0 {
                        String::new()
                    } else {
                        count.to_string()
                    },
                    k,
                )),
        );
    }
    col()
        .key("navigation")
        .w(214.0)
        .h_full()
        .fill(k.c.bg_deep)
        .child(
            row()
                .h(42.0)
                .w_full()
                .px(18.0)
                .gap(9.0)
                .center_y()
                .child(
                    icon(&icons::INDICATOR, 18.0)
                        .color(k.c.icon_accent)
                        .a11y_hidden(),
                )
                .child(text("Voxtype").font(k.ty.ui_medium).color(k.c.text_base)),
        )
        .child(rule(k))
        .child(
            col()
                .key("nav-scroll")
                .grow(1.0)
                .min_h(0.0)
                .w_full()
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(items)
                .scroll_bar(k),
        )
        .child(
            col()
                .padding(14.0)
                .gap(6.0)
                .child(rule(k))
                .child(caption(
                    if s.demo {
                        "DEMO · no file writes"
                    } else {
                        "Working editor"
                    },
                    k,
                ))
                .child(
                    text(s.theme.label())
                        .font(k.ty.small)
                        .color(k.c.text_accent),
                ),
        )
}

fn toggle(s: &Editor, index: usize, k: &Kit) -> El {
    let f = &s.fields[index];
    let on = f.edit.text == "true";
    let enabled = s.editable(index);
    let mut control = el()
        .key(format!("field-{index}"))
        .role("switch")
        .label(f.spec.label.clone())
        .checked(on)
        .focusable(enabled)
        .disabled(!enabled)
        .focus_ring(2.0, 2.0, k.c.border_focus)
        .size(46.0, 24.0)
        .radius(12.0)
        .fill(if on {
            k.c.bg_accent.alpha(0.25)
        } else {
            k.c.layer_04
        })
        .border(
            1.0,
            if on {
                k.c.bg_accent.alpha(0.65)
            } else {
                k.c.border_strong
            },
        )
        .child(
            el().key("thumb")
                .size(20.0, 20.0)
                .radius(10.0)
                .absolute(if on { 24.0 } else { 2.0 }, 2.0)
                .fill(if on { k.c.bg_accent } else { k.c.text_muted })
                .motion(k.motion.control)
                .a11y_hidden(),
        );
    if enabled {
        control = control.on_press(move |s: &mut Editor| {
            s.fields[index].set(if on { "false" } else { "true" });
            s.error = None;
        });
    }
    control
}

fn field_default(s: &Editor, index: usize, k: &Kit) -> El {
    let f = &s.fields[index];
    if f.spec.file_value.is_null() && !f.clear {
        return el().size(0.0, 0.0);
    }
    button("Default", s.editable(index), false, k, move |s| {
        s.fields[index].reset_override()
    })
    .key(format!("default-{index}"))
}

fn browse_model(index: usize, enabled: bool, k: &Kit) -> El {
    Button::new("Browse…")
        .variant(ButtonVariant::Outline)
        .disabled(!enabled)
        .on_press_tx(move |s: &mut Editor, tx: &mut Tx| {
            let options = FileDialogOptions {
                title: "Choose a model file".into(),
                ..Default::default()
            }
            .filter("Model files", &["bin", "onnx"]);
            let key = s.fields[index].spec.key.clone();
            tx.open_file(options, move |s: &mut Editor, result, _| match result {
                Ok(Some(path)) => {
                    if let Some(index) = s.field_index(&key) {
                        let value = path.to_string_lossy().into_owned();
                        s.fields[index].set(&value);
                        s.fields[index].clear = false;
                        s.custom = None;
                        s.open_select = None;
                        s.error = None;
                        s.notice = format!("Selected model: {}", schema::model_label(&value));
                    } else {
                        s.error = Some(format!(
                            "{key} is no longer available; reload and try again."
                        ));
                    }
                }
                Ok(None) => {}
                Err(error) => s.error = Some(format!("Could not open the file picker: {error}")),
            });
        })
        .build(k)
        .key(format!("browse-{index}"))
}

fn field_input(s: &Editor, index: usize, k: &Kit, layers: &mut Vec<El>) -> El {
    let f = &s.fields[index];
    let enabled = s.editable(index);
    if f.spec.kind == "bool" {
        return toggle(s, index, k);
    }
    if matches!(f.spec.kind.as_str(), "enum" | "dynamic_enum") && s.custom != Some(index) {
        let values = s.choices(index);
        let refs: Vec<_> = values.iter().map(String::as_str).collect();
        let shown = s.choice_display(index, &f.edit.text);
        let current = (!f.edit.text.is_empty()).then_some(shown.as_str());
        let select = Select::new(format!("choice-{index}"), &refs)
            .label(&f.spec.label)
            .appearance(SelectAppearance::Field)
            .width(264.0)
            .current(current)
            .placeholder("Unset · built-in default")
            .open(s.open_select == Some(index))
            .disabled(!enabled)
            .on_toggle(move |s: &mut Editor| {
                s.open_select = if s.open_select == Some(index) {
                    None
                } else {
                    Some(index)
                }
            })
            .on_close(|s: &mut Editor| s.open_select = None)
            .on_choose(move |s: &mut Editor, value, _| {
                let value = s.choice_value(index, value);
                s.fields[index].set(&value);
                s.open_select = None;
                s.error = None;
            })
            .build(k, layers)
            .h(34.0)
            .border(1.0, k.c.border_base);
        let custom = f.spec.open || f.spec.kind == "dynamic_enum";
        let browse = schema::is_model_file_key(&f.spec.key);
        let mut actions = row().gap(6.0).justify_end();
        if browse {
            actions = actions.child(browse_model(index, enabled, k));
        }
        if custom {
            actions = actions.child(
                button("Custom…", enabled, false, k, move |s| {
                    s.custom = Some(index);
                    s.open_select = None;
                })
                .key(format!("custom-{index}")),
            );
        }
        return col()
            .gap(6.0)
            .items_end()
            .child(select)
            .child(if browse || custom {
                actions
            } else {
                el().size(0.0, 0.0)
            });
    }
    let mut input = TextInput::new(&f.edit)
        .label(&f.spec.label)
        .width(Some(264.0))
        .height(34.0)
        .placeholder("Optional · use Default to unset")
        .disabled(!enabled)
        .invalid(f.error().is_some())
        .on_edit(move |s: &mut Editor| &mut s.fields[index].edit)
        .on_change(move |s: &mut Editor, _| {
            s.fields[index].clear = false;
            s.error = None;
        });
    if f.spec.key.contains("api_key") {
        input = input.secure();
    }
    input
        .build(k)
        .key(format!("field-{index}"))
        .border(1.0, k.c.border_base)
}

fn field_row(s: &Editor, index: usize, k: &Kit, layers: &mut Vec<El>) -> El {
    let f = &s.fields[index];
    let input = field_input(s, index, k, layers);
    let mut copy = col()
        .min_w(0.0)
        .grow(1.0)
        .gap(5.0)
        .child(
            text(&f.spec.label)
                .font(k.ty.ui_medium)
                .color(k.c.text_base)
                .wrap(),
        )
        .child(caption(&f.spec.description, k))
        .child(caption(&f.spec.key, k));
    if !f.spec.compiled {
        copy = copy.child(
            text("Unavailable in the active Voxtype binary")
                .font(k.ty.small)
                .color(k.c.text_accent)
                .wrap(),
        );
    }
    if f.clear {
        copy = copy.child(caption("Override will be removed on Apply", k));
    }
    if f.spec.key.ends_with(".language") {
        copy = copy.child(caption(
            "Comma-separated codes are saved as a language array, e.g. en,ms.",
            k,
        ));
    }
    if let Some(error) = f.error() {
        copy = copy.child(text(error).font(k.ty.small).color(k.c.danger.fg).wrap());
    }
    row()
        .key(format!("setting-{}", f.spec.key))
        .w_full()
        .px(18.0)
        .py(16.0)
        .gap(20.0)
        .center_y()
        .child(copy)
        .child(
            col()
                .gap(6.0)
                .items_end()
                .child(input)
                .child({
                    let mut actions = row().gap(6.0).justify_end();
                    if s.custom == Some(index) {
                        if schema::is_model_file_key(&f.spec.key) {
                            actions = actions.child(browse_model(index, s.editable(index), k));
                        }
                        actions = actions.child(
                            button("Done", s.editable(index), false, k, move |s| {
                                s.custom = None;
                                s.error = None;
                            })
                            .key(format!("done-{index}")),
                        );
                    }
                    actions
                })
                .child(field_default(s, index, k)),
        )
}

fn overview(s: &Editor, k: &Kit) -> El {
    let path = s
        .backend
        .as_ref()
        .map(|b| b.path.display().to_string())
        .unwrap_or_else(|| "Demo fixture (never saved)".into());
    let mut content = col().w_full().gap(16.0)
        .child(panel(col().gap(10.0)
            .child(text(format!("Voxtype {} · {} settings", s.version, s.fields.len())).font(k.ty.ui_medium).color(k.c.text_base))
            .child(caption(path, k))
            .child(caption(format!("Engine: {}\nModel: {}\nInput: {}\nOutput: {}", s.engine(), summary_value(s, &format!("{}.model", s.engine())), summary_value(s, "audio.device"), summary_value(s, "output.mode")), k))
            .child(caption("Defaults are read from the installed daemon's schema. Only changed fields are written; unrelated settings and comments are preserved.", k)), k))
        .child(panel(col().gap(10.0).child(caption(&s.inventory.service, k)).child(caption(&s.inventory.variants, k))
            .child(caption("This editor never installs packages, swaps binary variants, downloads models, records audio, or executes your configured hooks. Engine support is shown for the active binary.", k)), k))
        .child(row().w_full().child(button("Reload configuration & devices", !s.busy(), false, k, |s| if s.demo { s.notice = "Demo inventory is synthetic.".into(); } else { s.modal = Some(Modal::Reload); }).key("reload")));
    for warning in s
        .model_warning()
        .into_iter()
        .chain(s.device_warning())
        .chain(s.inventory.warnings.iter().cloned())
    {
        content = content.child(
            col()
                .w_full()
                .padding(12.0)
                .radius(k.radius.lg)
                .border(1.0, k.c.danger.fg.alpha(0.5))
                .fill(k.c.danger.bg)
                .child(text(warning).font(k.ty.small).color(k.c.danger.fg).wrap()),
        );
    }
    content
}

fn config_file(s: &Editor, k: &Kit) -> El {
    let fields_dirty = s.fields.iter().any(|f| f.dirty());
    col().w_full().gap(14.0)
        .child(caption("Advanced full-document editor: word replacements, profiles, modifier maps, driver arrays, recipes, and settings beyond the schema. TOML syntax is checked here and the entire result is checked by Voxtype before saving. This view can contain credentials; do not share screenshots.", k))
        .child(caption(if fields_dirty { "Apply or reset the field edits first. Full-document editing is locked to avoid overwriting them." } else { "Field controls are locked while full-document edits are pending. Reset restores the loaded file." }, k))
        .child(TextArea::new(&s.document).label("Full Voxtype TOML document").width(None).rows(24).max_height(480.0)
            .disabled(s.busy() || fields_dirty).on_edit(|s: &mut Editor| &mut s.document)
            .on_change(|s: &mut Editor, _| s.error = None).build(k).key("full-document").w_full().border(1.0, k.c.border_base))
}

fn settings(s: &Editor, k: &Kit, layers: &mut Vec<El>) -> El {
    let mut content = col().w_full().gap(14.0);
    if s.raw_dirty() {
        content = content.child(caption(
            "Full-document edits are pending. Apply or reset them to unlock field controls.",
            k,
        ));
    }
    if s.page == 1 || s.page == 11 {
        content = content.child(
            row().w_full().child(
                button(
                    if s.all_engines {
                        "Show active engine only"
                    } else {
                        "Show all engine settings"
                    },
                    true,
                    false,
                    k,
                    |s| s.all_engines = !s.all_engines,
                )
                .key("all-engines"),
            ),
        );
    }
    if s.page == 5 {
        content = content.child(button(
            "Edit word replacements in Config file",
            !s.busy(),
            false,
            k,
            |s| s.navigate(12),
        ));
    }
    if s.page == 9 {
        content = content.child(caption("The installed custom recording overlay remains in place. Host size/layout/frame options may not affect custom QML; changing them does not edit its implementation.", k));
    }
    let page_warning = match s.page {
        1 => s.model_warning(),
        3 => s.device_warning(),
        _ => None,
    };
    if let Some(warning) = page_warning {
        content = content.child(caption(warning, k));
    }
    let mut group = col()
        .w_full()
        .radius(k.radius.xl)
        .clip_rounded(k.radius.xl)
        .border(1.0, k.c.border_base)
        .fill(k.c.layer_01);
    let mut count = 0;
    for (index, field) in s.fields.iter().enumerate().filter(|(_, f)| s.visible(f)) {
        if count > 0 {
            group = group.child(rule(k));
        }
        let _ = field;
        group = group.child(field_row(s, index, k, layers));
        count += 1;
    }
    if count == 0 {
        content.child(caption(
            "No matching settings. Try another search or show all engines.",
            k,
        ))
    } else {
        content.child(group)
    }
}

fn footer(s: &Editor, k: &Kit) -> El {
    let message = s
        .validation()
        .or_else(|| s.error.clone())
        .unwrap_or_else(|| s.notice.clone());
    col().w_full().child(rule(k)).child(
        row()
            .w_full()
            .min_h(84.0)
            .px(24.0)
            .py(12.0)
            .gap(10.0)
            .center_y()
            .child(
                col()
                    .grow(1.0)
                    .min_w(0.0)
                    .gap(4.0)
                    .child(
                        text(if s.busy() {
                            "Working…".into()
                        } else if s.raw_dirty() {
                            "Full-document edits pending".into()
                        } else {
                            format!("{} unsaved changes", s.changed_count())
                        })
                        .font(k.ty.ui_medium)
                        .color(k.c.text_base),
                    )
                    .child(
                        text(message)
                            .key("editor-status")
                            .font(k.ty.small)
                            .color(if s.error.is_some() || s.validation().is_some() {
                                k.c.danger.fg
                            } else {
                                k.c.text_muted
                            })
                            .wrap(),
                    ),
            )
            .child(
                button("Reset", !s.busy() && s.dirty(), false, k, |s| {
                    s.modal = Some(Modal::Reset)
                })
                .key("reset"),
            )
            .child(
                button(
                    if s.demo { "Apply demo" } else { "Apply" },
                    !s.busy() && s.dirty() && s.validation().is_none(),
                    true,
                    k,
                    |s| s.modal = Some(Modal::Save),
                )
                .key("apply"),
            ),
    )
}

fn modal(s: &Editor, kind: Modal, k: &Kit) -> El {
    let (title, copy, confirm) = match kind {
        Modal::Save => (if s.demo { "Apply demo changes?" } else { "Save configuration?" }, s.review(), "Save"),
        Modal::Reset => ("Discard unsaved edits?", "This restores the loaded values. No files will be changed.".into(), "Discard edits"),
        Modal::Reload => ("Reload from disk?", "Unsaved edits will be discarded. Configuration, models and devices will be read again.".into(), "Reload"),
        Modal::Restart => ("Restart Voxtype?", "This interrupts the daemon. It is refused unless a status check reports idle; start a recording immediately after checking and the race is unavoidable. Saved settings take effect after restart.".into(), "Restart now"),
        Modal::Quit => ("Discard edits and close?", "Your unsaved changes have not been written to disk.".into(), "Discard & close"),
        Modal::Help => ("Keyboard shortcuts", "Ctrl+S / Ctrl+Enter   Review and apply\nCtrl+R   Confirm reload\nCtrl+Q   Close (asks about unsaved edits)\nTab / Shift+Tab   Move focus\nEscape   Dismiss menus or dialogs\n?   This help".into(), "Got it"),
    };
    let confirm_enabled = kind != Modal::Restart
        || !s.demo && !s.dirty() && s.backend.as_ref().is_some_and(|b| b.restart_allowed);
    let mut confirm_button = button(confirm, confirm_enabled, true, k, move |s| match kind {
        Modal::Save => s.save(),
        Modal::Reset => s.reset(),
        Modal::Reload => s.load(),
        Modal::Restart => s.restart(),
        _ => s.modal = None,
    })
    .key("confirm");
    if kind == Modal::Quit {
        confirm_button = Button::new(confirm)
            .variant(ButtonVariant::Outline)
            .on_press_tx(|_: &mut Editor, tx| tx.quit())
            .build(k)
            .key("confirm");
    }
    el().key("modal-backdrop").absolute(0.0, 0.0).w_full().h_full().fill(Color::BLACK.alpha(0.60)).center()
        .child(col().key("confirmation-dialog").w(540.0).max_h(540.0).padding(24.0).gap(18.0).fill(k.c.layer_01).radius(k.radius.xl).border(1.0, k.c.border_base)
            .modal(true, |s: &mut Editor| s.modal = None)
            .child(text(title).font(k.ty.ui_medium).color(k.c.text_base))
            .child(col().key("review-scroll").w_full().max_h(320.0).shrink().scroll(ScrollSpec { stick_bottom: false }).child(caption(copy, k)).scroll_bar(k))
            .child(caption(if s.demo { "Demo mode: no writes or service actions." } else if kind == Modal::Save { "Only after validation succeeds: create a private backup and atomically replace the file. Restart is NOT automatic." } else { "" }, k))
            .child(row().w_full().gap(10.0).justify_end().child(button("Cancel", true, false, k, |s| s.modal = None).key("cancel")).child(confirm_button)))
}

pub fn view(s: &Editor, _: &mut Cx<Editor>) -> El {
    let k = s.theme.palette.kit();
    let mut layers = Vec::new();
    let body = if !s.fields.is_empty() && !s.search.text.is_empty() {
        settings(s, &k, &mut layers)
    } else if s.page == 12 && !s.source.is_empty() {
        config_file(s, &k)
    } else if s.fields.is_empty() {
        col()
            .w_full()
            .gap(12.0)
            .child(caption(s.error.as_deref().unwrap_or(&s.notice), &k))
            .child(
                row()
                    .gap(8.0)
                    .child(button("Retry", !s.busy(), false, &k, Editor::load))
                    .child(button(
                        "Edit full file",
                        !s.busy() && s.backend.is_some(),
                        false,
                        &k,
                        Editor::open_raw,
                    )),
            )
    } else if s.page == 0 {
        overview(s, &k)
    } else if s.page == 12 {
        config_file(s, &k)
    } else {
        settings(s, &k, &mut layers)
    };
    let main = col()
        .grow(1.0)
        .min_w(0.0)
        .h_full()
        .child(
            row()
                .w_full()
                .h(42.0)
                .px(24.0)
                .gap(12.0)
                .center_y()
                .fill(k.c.bg_deep)
                .child(
                    text(format!("Settings / {}", SECTIONS[s.page]))
                        .font(k.ty.ui_medium)
                        .color(k.c.text_accent)
                        .grow(1.0),
                )
                .child(button("?", true, false, &k, |s| {
                    s.modal = Some(Modal::Help)
                }))
                .child(
                    button(
                        "Restart…",
                        !s.demo
                            && !s.busy()
                            && !s.dirty()
                            && s.backend.as_ref().is_some_and(|b| b.restart_allowed),
                        false,
                        &k,
                        |s| s.modal = Some(Modal::Restart),
                    )
                    .key("restart"),
                ),
        )
        .child(rule(&k))
        .child(
            row().w_full().px(24.0).py(12.0).child(
                TextInput::new(&s.search)
                    .label("Search every setting")
                    .placeholder("Search labels, keys and descriptions…")
                    .width(None)
                    .height(34.0)
                    .on_edit(|s: &mut Editor| &mut s.search)
                    .on_change(|s: &mut Editor, _| s.open_select = None)
                    .build(&k)
                    .key("search")
                    .grow(1.0)
                    .border(1.0, k.c.border_base),
            ),
        )
        .child(
            col()
                .key("content-scroll")
                .w_full()
                .grow(1.0)
                .min_h(0.0)
                .scroll(ScrollSpec {
                    stick_bottom: false,
                })
                .child(
                    col()
                        .key(format!("page-{}-{}", s.page, s.generation))
                        .w_full()
                        .px(24.0)
                        .pb(24.0)
                        .gap(16.0)
                        .child(
                            text(if s.search.text.is_empty() {
                                SECTIONS[s.page].to_owned()
                            } else {
                                "Search results".into()
                            })
                            .key("page-title")
                            .font(k.ty.ui_medium)
                            .color(k.c.text_base),
                        )
                        .child(body),
                )
                .scroll_bar(&k),
        )
        .child(footer(s, &k));
    let base = row()
        .key("settings-window")
        .role("window")
        .label("Voxtype Settings")
        .w_full()
        .h_full()
        .fill(k.c.bg_base)
        .child(navigation(s, &k))
        .child(el().w(1.0).h_full().fill(k.c.border_muted))
        .child(main);
    if let Some(kind) = s.modal {
        layers.push(modal(s, kind, &k));
    }
    rune_kit::root(base, layers)
}
