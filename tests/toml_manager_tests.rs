#[cfg(test)]
mod toml_manager_tests
{
    use std::fs;
    use std::path::PathBuf;
    use tempfile::{tempdir, NamedTempFile};
    use docwen::toml_parse::Mode::MatchFunctionDocs;
    use docwen::toml_parse::Settings;
    use docwen::toml_manager::*;

    #[test]
    fn create_default_writes_new_file()
    {
        let dir = tempdir().expect("tmp dir");
        let file_path = dir.path().join("docwen.toml");

        create_default(&file_path).expect("create_default failed");

        let written = fs::read_to_string(&file_path).expect("read");
        assert_eq!(written, DEFAULT_TOML);
    }

    #[test]
    fn create_default_fails_if_file_exists()
    {
        let tmp = NamedTempFile::new().expect("tmp file");
        fs::write(tmp.path(), b"something").unwrap();

        let err = create_default(tmp.path()).unwrap_err();
        assert!(
            err
                .to_string()
                .contains("Failed to create new docwen.toml")
        );
    }

    /// Helper to build Settings with arbitrary match/ignore sets
    fn make_settings(match_extensions: &[&str], ignore: &[&str]) -> Settings
    {
        Settings
        {
            target: ".".into(),
            match_extensions: match_extensions.iter().map(|s| s.to_string()).collect(),
            mode: MatchFunctionDocs,
            ignore: ignore.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn group_by_stem_basic_grouping()
    {
        let settings = make_settings(&["h", "c"], &[]);
        let paths = vec![
            PathBuf::from("foo.c"),
            PathBuf::from("foo.h"),
            PathBuf::from("bar.h"),
            PathBuf::from("bar.txt"),
        ];

        let groups = group_by_stem(paths, &settings);

        let mut counts = std::collections::HashMap::new();
        for g in &groups
        {
            counts.insert(g.name.as_str(), g.files.len());
        }

        assert_eq!(counts.get("foo").copied(), Some(2));
        assert_eq!(counts.get("bar").copied(), Some(1));
    }

    #[test]
    fn group_by_stem_respects_ignore_list()
    {
        let settings = make_settings(&["c"], &["skipme"]);
        let paths = vec![PathBuf::from("skipme.c"), PathBuf::from("keepme.c")];

        let groups = group_by_stem(paths, &settings);
        let names: std::collections::HashSet<_> =
            groups.into_iter().map(|g| g.name).collect();

        assert!(!names.contains("skipme"));
        assert!(names.contains("keepme"));
    }

    #[test]
    fn group_by_stem_extension_case_insensitive()
    {
        let settings = make_settings(&["h", "c"], &[]);
        let paths = vec![PathBuf::from("FoO.H"), PathBuf::from("foo.c")];

        let groups = group_by_stem(paths, &settings);
        assert_eq!(groups[0].files.len(), 2);
        assert_eq!(groups[0].name, "foo");
    }
}