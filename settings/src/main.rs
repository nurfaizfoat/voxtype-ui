mod backend;
mod editor;
mod editor_ui;
mod icons;
mod schema;
#[cfg(test)]
mod tests;
mod theme;

use std::path::PathBuf;

use editor::{Editor, LAUNCH};
use rune::{AppSpec, Key, KeyEvent, Tx};

fn parse_launch() -> (bool, Option<PathBuf>) {
    let mut demo = false;
    let mut config = None;
    let args = rune::tooling::extra_args();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--demo" => demo = true,
            "--config" => {
                index += 1;
                config = args.get(index).map(PathBuf::from);
            }
            _ => (),
        }
        index += 1;
    }
    (demo, config)
}

/// `editor_ui` calls need to hand these to the event loop; keep the Rune
/// signatures in one place so the UI stays free of persistence details.
fn poll(s: &mut Editor, time: f64, tx: &mut Tx) -> bool {
    s.poll(time, tx)
}

fn on_key(s: &mut Editor, event: &KeyEvent, tx: &mut Tx) -> bool {
    for page in 0..editor::SECTIONS.len() {
        if event.is_command(char::from(b'1' + page as u8)) {
            s.navigate(page);
            return true;
        }
    }
    if matches!(event.key, Key::Escape) && s.modal.is_none() && s.open_select.is_none() {
        return false;
    }
    editor::on_key(s, event, tx)
}

fn main() {
    let launch = parse_launch();
    let _ = LAUNCH.set(launch);
    rune::set_app_id("voxtype-settings");
    rune::run(AppSpec {
        families: theme::FONT_FAMILIES,
        min_size: Some((840.0, 620.0)),
        poll: Some(poll),
        on_key: Some(on_key),
        scenes: &[
            ("overview", |s| s.navigate(0)),
            ("engine", |s| s.navigate(1)),
            ("hotkey", |s| s.navigate(2)),
            ("audio", |s| s.navigate(3)),
            ("output", |s| s.navigate(4)),
            ("text", |s| s.navigate(5)),
            ("vad", |s| s.navigate(6)),
            ("meeting", |s| s.navigate(7)),
            ("notifications", |s| s.navigate(8)),
            ("osd", |s| s.navigate(9)),
            ("status", |s| s.navigate(10)),
            ("advanced", |s| s.navigate(11)),
            ("config", |s| s.navigate(12)),
            ("edited", |s| {
                s.navigate(4);
                if let Some(i) = s.field_index("output.type_delay_ms") {
                    s.fields[i].set("25");
                }
            }),
            ("invalid", |s| {
                s.navigate(3);
                if let Some(i) = s.field_index("audio.max_duration_secs") {
                    s.fields[i].set("0");
                }
            }),
            ("search", |s| {
                s.navigate(0);
                s.search = rune::TextEdit::new("buffer");
            }),
        ],
        ..AppSpec::new(
            "Voxtype Settings",
            (980.0, 720.0),
            Editor::init,
            editor_ui::view,
        )
    });
}
