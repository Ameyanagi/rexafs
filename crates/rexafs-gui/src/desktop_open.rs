//! Decode desktop open requests before routing them through the analysis window.

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::PathBuf;

/// Preserve positional paths, including non-Unicode names. Older macOS launchers
/// may supply a process serial number (`-psn_…`) rather than a document path.
/// An explicit `--` ends launcher-argument handling.
pub(crate) fn argument_paths(arguments: impl IntoIterator<Item = OsString>) -> Vec<PathBuf> {
    let mut positional_only = false;
    let paths = arguments.into_iter().filter_map(|argument| {
        if !positional_only {
            if argument == "--" {
                positional_only = true;
                return None;
            }
            if argument
                .to_str()
                .is_some_and(|argument| argument.starts_with("-psn_"))
            {
                return None;
            }
        }
        (!argument.is_empty()).then(|| PathBuf::from(argument))
    });
    unique_paths(paths)
}

/// GPUI delivers macOS Finder documents as absolute file URLs, with spaces and
/// non-ASCII names percent-encoded. Ignore other URL schemes and URL suffixes
/// that cannot identify a filesystem document unambiguously.
pub(crate) fn paths_from_urls(urls: impl IntoIterator<Item = String>) -> Vec<PathBuf> {
    unique_paths(urls.into_iter().filter_map(|value| {
        let url = url::Url::parse(&value).ok()?;
        if url.scheme() != "file" || url.query().is_some() || url.fragment().is_some() {
            return None;
        }
        url.to_file_path().ok()
    }))
}

/// Remove repeated paths within one request while retaining selection order.
/// Do not retain a history: opening the same document again is a new request.
pub(crate) fn unique_paths(paths: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    paths
        .into_iter()
        .filter(|path| seen.insert(path.canonicalize().unwrap_or_else(|_| path.clone())))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finder_urls_decode_spaces_unicode_and_literal_url_punctuation() {
        let path = std::env::temp_dir().join("rexafs opening tests/銅 sample #1 ? 50%.rxs");
        let url = url::Url::from_file_path(&path).unwrap().to_string();
        assert!(url.contains("%20"));
        assert!(url.contains("%23"));
        assert!(url.contains("%25"));
        assert_eq!(paths_from_urls([url]), vec![path]);
    }

    #[test]
    fn non_document_urls_do_not_become_import_paths() {
        let valid = url::Url::from_file_path(std::env::temp_dir().join("test.rxs"))
            .unwrap()
            .to_string();
        assert!(
            paths_from_urls([
                "https://example.com/test.rxs".into(),
                "rexafs://project/test.rxs".into(),
                "not a URL".into(),
                format!("{valid}?download=1"),
                format!("{valid}#selection"),
            ])
            .is_empty()
        );
    }

    #[test]
    fn a_request_preserves_all_distinct_paths_in_selection_order() {
        let first = std::env::temp_dir().join("one.rxs");
        let second = std::env::temp_dir().join("two.rxs");
        let first_url = url::Url::from_file_path(&first).unwrap().to_string();
        let second_url = url::Url::from_file_path(&second).unwrap().to_string();
        assert_eq!(
            paths_from_urls([first_url.clone(), second_url, first_url.clone()]),
            vec![first.clone(), second]
        );
        // A later native event can intentionally reopen the same project.
        assert_eq!(paths_from_urls([first_url]), vec![first]);
    }

    #[test]
    fn equivalent_existing_paths_are_one_document_request() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("same.rxs");
        std::fs::write(&path, "{}").unwrap();
        let alias = directory.path().join(".").join("same.rxs");
        assert_eq!(unique_paths([alias.clone(), path]), vec![alias]);
    }

    #[test]
    fn launcher_arguments_are_not_imported_and_multiple_paths_are_retained() {
        assert_eq!(
            argument_paths([
                "-psn_0_12345".into(),
                "first.rxs".into(),
                "folder".into(),
                "first.rxs".into(),
                "--".into(),
                "-psn_a_real_filename".into(),
            ]),
            vec![
                PathBuf::from("first.rxs"),
                PathBuf::from("folder"),
                PathBuf::from("-psn_a_real_filename"),
            ]
        );
    }

    #[cfg(unix)]
    #[test]
    fn paths_do_not_require_unicode_file_names() {
        use std::os::unix::ffi::OsStringExt;

        let path = std::env::temp_dir().join(OsString::from_vec(b"sample-\xff.rxs".to_vec()));
        let url = url::Url::from_file_path(&path).unwrap().to_string();
        assert_eq!(
            argument_paths([path.clone().into_os_string()]),
            vec![path.clone()]
        );
        assert_eq!(paths_from_urls([url]), vec![path]);
    }
}
