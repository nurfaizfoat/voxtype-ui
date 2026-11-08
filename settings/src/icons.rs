use rune::Icon;

pub static OVERVIEW: Icon = Icon {
    name: "overview",
    view: 24.0,
    body: r#"<rect x="3.5" y="3.5" width="6.5" height="6.5" stroke="currentColor" fill="none"/><rect x="14" y="3.5" width="6.5" height="6.5" stroke="currentColor" fill="none"/><rect x="3.5" y="14" width="6.5" height="6.5" stroke="currentColor" fill="none"/><rect x="14" y="14" width="6.5" height="6.5" stroke="currentColor" fill="none"/>"#,
};
pub static SPEECH: Icon = Icon {
    name: "speech",
    view: 24.0,
    body: r#"<path d="M4 4.5H20V16.5H11L6 20V16.5H4Z M8 9H16 M8 12H14" stroke="currentColor" stroke-width="1.5" fill="none"/>"#,
};
pub static AUDIO: Icon = Icon {
    name: "audio",
    view: 24.0,
    body: r#"<path d="M9 5A3 3 0 0 1 15 5V11A3 3 0 0 1 9 11Z M5.5 10V11A6.5 6.5 0 0 0 18.5 11V10 M12 17.5V21 M8.5 21H15.5" stroke="currentColor" stroke-width="1.5" fill="none"/>"#,
};
pub static OUTPUT: Icon = Icon {
    name: "output",
    view: 24.0,
    body: r#"<path d="M9 4H4V20H9 M8 12H21 M16 7L21 12L16 17" stroke="currentColor" stroke-width="1.5" fill="none"/>"#,
};
pub static INDICATOR: Icon = Icon {
    name: "indicator",
    view: 24.0,
    body: r#"<rect x="2.5" y="5.5" width="19" height="13" stroke="currentColor" stroke-width="1.5" fill="none"/><path d="M6.5 11V13 M10 9V15 M13.5 8V16 M17 10V14" stroke="currentColor" stroke-width="2"/>"#,
};
