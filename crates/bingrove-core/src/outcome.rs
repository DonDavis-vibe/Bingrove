//! What became of files handed to Meshgrove. Newer Meshgroves answer a bundle with a
//! `bundle_finished` event. The plugin API this crate family is pinned to does not know that
//! event yet, so it is read from its JSON here; older hosts never send it.

use serde::Deserialize;

/// How Meshgrove dealt with a bundle.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum BundleOutcome {
    /// The change went through; `skipped` of its steps did nothing.
    Applied {
        #[serde(default)]
        steps: u32,
        #[serde(default)]
        skipped: u32,
    },
    /// Waiting in the review.
    Queued,
    /// The plugin may not put files into the library; they are gone.
    Refused,
    /// Something went wrong; the library is unchanged.
    Failed {
        #[serde(default)]
        error: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct BundleFinished {
    pub title: String,
    pub outcome: BundleOutcome,
}

impl BundleFinished {
    /// The event in a panel event's JSON; `None` for any other event.
    pub fn parse(event: &str) -> Option<Self> {
        #[derive(Deserialize)]
        struct Tagged {
            #[serde(rename = "type")]
            kind: String,
            #[serde(flatten)]
            finished: BundleFinished,
        }
        serde_json::from_str::<Tagged>(event).ok().filter(|t| t.kind == "bundle_finished").map(|t| t.finished)
    }

    /// What to tell the user, naming the setting as Meshgrove's own language does.
    pub fn text(&self, language: &str) -> String {
        let title = &self.title;
        let german = language.starts_with("de");
        match (&self.outcome, german) {
            (BundleOutcome::Applied { skipped: 0, .. }, true) => format!("{title} ist in der Bibliothek."),
            (BundleOutcome::Applied { skipped: 0, .. }, false) => format!("{title} is in the library."),
            (BundleOutcome::Applied { skipped, .. }, true) => {
                format!("{title} ist in der Bibliothek, {skipped} Schritte blieben ohne Wirkung.")
            }
            (BundleOutcome::Applied { skipped, .. }, false) => {
                format!("{title} is in the library; {skipped} steps did nothing.")
            }
            (BundleOutcome::Queued, true) => format!("{title} wartet in Meshgrove auf deine Prüfung."),
            (BundleOutcome::Queued, false) => format!("{title} is waiting for your review in Meshgrove."),
            (BundleOutcome::Refused, true) => format!(
                "Meshgrove hat {title} abgelehnt, die Dateien sind verworfen. Unter Einstellungen → Plugins → Bingrove „Dateien in die Bibliothek legen“ erlauben und neu generieren."
            ),
            (BundleOutcome::Refused, false) => format!(
                "Meshgrove refused {title} and discarded the files. Allow \"Put files into the library\" under Settings → Plugins → Bingrove and generate again."
            ),
            (BundleOutcome::Failed { error }, true) => format!("{title} kam nicht in die Bibliothek: {error}"),
            (BundleOutcome::Failed { error }, false) => format!("{title} did not reach the library: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(outcome: &str) -> String {
        format!(r#"{{"type":"bundle_finished","title":"Gridfinity Schublade","outcome":{outcome}}}"#)
    }

    #[test]
    fn reads_every_outcome_as_meshgrove_writes_it() {
        let outcome = |json: &str| BundleFinished::parse(&event(json)).unwrap().outcome;
        assert_eq!(
            outcome(r#"{"status":"applied","steps":56,"skipped":0}"#),
            BundleOutcome::Applied { steps: 56, skipped: 0 }
        );
        assert_eq!(outcome(r#"{"status":"queued"}"#), BundleOutcome::Queued);
        assert_eq!(outcome(r#"{"status":"refused"}"#), BundleOutcome::Refused);
        assert_eq!(
            outcome(r#"{"status":"failed","error":"disk full"}"#),
            BundleOutcome::Failed { error: "disk full".into() }
        );
    }

    #[test]
    fn leaves_other_events_alone() {
        assert_eq!(BundleFinished::parse(r#"{"type":"click","id":"generate"}"#), None);
        assert_eq!(BundleFinished::parse(r#"{"type":"click","title":"x","outcome":{"status":"queued"}}"#), None);
        assert_eq!(BundleFinished::parse(&event(r#"{"status":"something_new"}"#)), None);
        assert_eq!(BundleFinished::parse("not json"), None);
    }

    #[test]
    fn a_refusal_names_the_setting_in_meshgroves_language() {
        let done = BundleFinished::parse(&event(r#"{"status":"refused"}"#)).unwrap();
        assert!(done.text("de").contains("„Dateien in die Bibliothek legen“"), "{}", done.text("de"));
        assert!(done.text("en").contains("\"Put files into the library\""), "{}", done.text("en"));
    }

    #[test]
    fn skipped_steps_are_mentioned() {
        let done = BundleFinished::parse(&event(r#"{"status":"applied","steps":9,"skipped":2}"#)).unwrap();
        assert!(done.text("de").contains("2 Schritte"), "{}", done.text("de"));
    }
}
