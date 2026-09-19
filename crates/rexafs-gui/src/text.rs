//! Small text helpers shared by the desktop panels.

/// Format a count with the correct singular or plural noun, e.g. `plural(1, "warning")`
/// → "1 warning" and `plural(3, "spectrum")` → "3 spectra". Irregular nouns used by
/// the interface are listed explicitly; other nouns take a trailing "s".
pub fn plural(count: usize, noun: &str) -> String {
    format!("{count} {}", noun_for(count, noun))
}

/// The noun alone, singular for a count of one and plural otherwise.
pub fn noun_for(count: usize, noun: &str) -> String {
    if count == 1 {
        return noun.to_string();
    }
    match noun {
        "spectrum" => "spectra".into(),
        "analysis" => "analyses".into(),
        "entry" => "entries".into(),
        "body" => "bodies".into(),
        _ => format!("{noun}s"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn counts_use_correct_nouns() {
        assert_eq!(plural(1, "warning"), "1 warning");
        assert_eq!(plural(0, "warning"), "0 warnings");
        assert_eq!(plural(1, "spectrum"), "1 spectrum");
        assert_eq!(plural(2, "spectrum"), "2 spectra");
        assert_eq!(noun_for(3, "component"), "components");
    }
}
