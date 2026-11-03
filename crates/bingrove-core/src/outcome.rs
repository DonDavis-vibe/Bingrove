//! What became of files handed to Meshgrove, in the words the panel shows. The panel turns
//! Meshgrove's `BundleFinished` event into these types; hosts before 0.8.2 never send it.

/// How Meshgrove dealt with a bundle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleOutcome {
    /// The change went through; `skipped` of its steps did nothing.
    Applied { skipped: u32 },
    /// Waiting in the review.
    Queued,
    /// The plugin may not put files into the library; they are gone.
    Refused,
    /// Something went wrong; the library is unchanged.
    Failed { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BundleFinished {
    pub title: String,
    pub outcome: BundleOutcome,
}

impl BundleFinished {
    /// What to tell the user, naming the setting as Meshgrove's own language does.
    pub fn text(&self, language: &str) -> String {
        let title = &self.title;
        let german = language.starts_with("de");
        match (&self.outcome, german) {
            (BundleOutcome::Applied { skipped: 0 }, true) => format!("{title} ist in der Bibliothek."),
            (BundleOutcome::Applied { skipped: 0 }, false) => format!("{title} is in the library."),
            (BundleOutcome::Applied { skipped }, true) => {
                format!("{title} ist in der Bibliothek, {skipped} Schritte blieben ohne Wirkung.")
            }
            (BundleOutcome::Applied { skipped }, false) => {
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

    fn finished(outcome: BundleOutcome) -> BundleFinished {
        BundleFinished { title: "Gridfinity Schublade".into(), outcome }
    }

    #[test]
    fn a_refusal_names_the_setting_in_meshgroves_language() {
        let done = finished(BundleOutcome::Refused);
        assert!(done.text("de").contains("„Dateien in die Bibliothek legen“"), "{}", done.text("de"));
        assert!(done.text("en").contains("\"Put files into the library\""), "{}", done.text("en"));
    }

    #[test]
    fn says_what_happened() {
        assert!(finished(BundleOutcome::Applied { skipped: 0 }).text("de").ends_with("ist in der Bibliothek."));
        assert!(finished(BundleOutcome::Applied { skipped: 2 }).text("de").contains("2 Schritte"));
        assert!(finished(BundleOutcome::Queued).text("en").contains("review"));
        let failed = finished(BundleOutcome::Failed { error: "disk full".into() });
        assert!(failed.text("en").ends_with("disk full"), "{}", failed.text("en"));
    }
}
