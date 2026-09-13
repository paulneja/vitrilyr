use gtk::prelude::*;
use std::{
    collections::BTreeMap,
    sync::{
        OnceLock,
        atomic::{AtomicU8, Ordering},
    },
};

static LANGUAGE: AtomicU8 = AtomicU8::new(0);
static CATALOG: OnceLock<BTreeMap<String, [String; 2]>> = OnceLock::new();

pub fn set_language(language: &str) {
    LANGUAGE.store(
        match language {
            "es" => 1,
            "zh" => 2,
            _ => 0,
        },
        Ordering::Relaxed,
    );
}
pub fn tr(text: &str) -> &str {
    translate(text, LANGUAGE.load(Ordering::Relaxed))
}
fn translate(text: &str, language: u8) -> &str {
    let catalog = CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../data/translations.json"))
            .expect("Bundled translations must be valid")
    });
    let entry = catalog
        .get_key_value(text)
        .or_else(|| catalog.iter().find(|(_, v)| v.iter().any(|s| s == text)));
    match (entry, language) {
        (Some((_, translations)), 1) => &translations[0],
        (Some((_, translations)), 2) => &translations[1],
        (Some((key, _)), _) => key,
        _ => text,
    }
}

pub fn refresh(widget: &impl IsA<gtk::Widget>) {
    let widget = widget.upcast_ref::<gtk::Widget>();
    if let Some(tooltip) = widget.tooltip_text() {
        let translated = tr(&tooltip);
        if translated != tooltip {
            widget.set_tooltip_text(Some(translated));
        }
    }
    if let Some(label) = widget.downcast_ref::<gtk::Label>()
        && !["track-title", "artist", "plain-lyrics"]
            .iter()
            .any(|class| label.has_css_class(class))
    {
        let text = label.text();
        let translated = tr(&text);
        if translated != text {
            label.set_text(translated);
        }
    }
    let mut child = widget.first_child();
    while let Some(current) = child {
        refresh(&current);
        child = current.next_sibling();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn languages_round_trip_and_leave_unknown_text_alone() {
        assert_eq!(translate("Settings", 1), "Ajustes");
        assert_eq!(translate("Settings", 2), "设置");
        assert_eq!(translate("设置", 0), "Settings");
        assert_eq!(translate("My own track title", 2), "My own track title");
    }
    #[test]
    fn catalogs_are_complete() {
        let catalog: BTreeMap<String, [String; 2]> =
            serde_json::from_str(include_str!("../data/translations.json")).unwrap();
        assert!(catalog.len() > 100);
        assert!(
            catalog
                .iter()
                .all(|(key, v)| !key.is_empty() && v.iter().all(|s| !s.is_empty()))
        );
    }
}
