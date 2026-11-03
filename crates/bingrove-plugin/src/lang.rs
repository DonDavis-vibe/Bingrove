//! The panel speaks Meshgrove's language: German or, for anything else, English. Both texts stand
//! side by side where they are used, so none can go missing.

use std::cell::Cell;

thread_local! {
    static GERMAN: Cell<bool> = const { Cell::new(true) };
}

/// Takes the language Meshgrove reports; called whenever the panel is about to speak.
pub(crate) fn set(language: &str) {
    GERMAN.set(language.starts_with("de"));
}

pub(crate) fn german() -> bool {
    GERMAN.get()
}

/// The text for the current language.
pub(crate) fn tr<'a>(de: &'a str, en: &'a str) -> &'a str {
    if german() { de } else { en }
}

/// `format!` for the current language: both literals take the same arguments.
macro_rules! tf {
    ($de:literal, $en:literal $(, $arg:expr)* $(,)?) => {
        if $crate::lang::german() { format!($de $(, $arg)*) } else { format!($en $(, $arg)*) }
    };
}
pub(crate) use tf;

/// The name of a catalogue entry or set in the current language.
pub(crate) fn label(id: &str, german_label: &str) -> String {
    match (german(), bingrove_core::catalog::english_label(id)) {
        (false, Some(english)) => english,
        _ => german_label.to_string(),
    }
}
