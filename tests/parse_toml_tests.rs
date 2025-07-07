#[cfg(test)]
mod parse_toml_tests
{
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::{NamedTempFile, TempPath};
    use docwen::toml_parse::*;

    const MINIMAL_VALID_TOML: &str = r#"
        [settings]
        target = "src"
        mode = "MATCH_FUNCTION_DOCS"
        "#;

    /// Helper for writing the given content into a temp file and returning its path
    fn write_temp_toml(content: &str) -> TempPath
    {
        let mut tmp = NamedTempFile::new().expect("tmp file");
        tmp.write_all(content.as_bytes()).unwrap();
        tmp.flush().unwrap();
        tmp.into_temp_path()
    }

    #[test]
    fn parses_full_config()
    {
        let toml = r#"
        [settings]
        target = "src"
        match_extensions = ["h", "c"]
        mode = "MATCH_FUNCTION_DOCS"
        ignore = ["some", "thing"]

        [[filegroup]]
        name = "a"
        files = ["a.h", "a.c"]

        [[filegroup]]
        name = "b"
        files = ["b.hpp", "b.cc"]
        "#;

        let path = write_temp_toml(toml);
        let docfig = Docfig::from_file(&path).expect("parse");

        // SETTINGS
        assert_eq!(docfig.settings.target, path.parent().unwrap().join("src"));
        assert_eq!(docfig.settings.match_extensions, vec!["h", "c"]);
        matches!(docfig.settings.mode, Mode::MatchFunctionDocs);
        assert_eq!(docfig.settings.ignore, vec!["some", "thing"]);

        // FILE_GROUPS
        assert_eq!(docfig.file_groups.len(), 2);
        assert_eq!(docfig.file_groups[0].name, "a");
        assert_eq!(docfig.file_groups[0].files, vec![PathBuf::from("a.h"), PathBuf::from("a.c")]);
    }

    #[test]
    fn parses_minimal_config()
    {
        let path = write_temp_toml(MINIMAL_VALID_TOML);
        let docfig = Docfig::from_file(&path).expect("parse");

        assert!(docfig.settings.match_extensions.is_empty());
        assert!(docfig.settings.ignore.is_empty());
        assert!(docfig.file_groups.is_empty());
    }

    #[test]
    fn parses_minimal_config_with_one_group()
    {
        let toml = r#"
        [settings]
        target = "src"
        mode = "MATCH_FUNCTION_DOCS"

        [[filegroup]]
        name = "only"
        files = ["x.h", "x.c", "x.cpp"]
        "#;

        let path = write_temp_toml(toml);
        let docfig = Docfig::from_file(&path).expect("parse");

        assert!(docfig.settings.match_extensions.is_empty());
        assert!(docfig.settings.ignore.is_empty());
    }

    #[test]
    fn fails_on_incorrect_toml_syntax()
    {
        let toml = r#"
        [settings]
        target == "src"
        mode = "MATCH_FUNCTION_DOCS"
        "#;

        let path = write_temp_toml(toml);
        let docfig = Docfig::from_file(&path);
        let Err(_) = docfig else { panic!("Config::from_file unexpectedly succeeded"); };
    }

    #[test]
    fn fails_on_missing_target()
    {
        let toml = r#"
        [settings]
        mode = "MATCH_FUNCTION_DOCS"
        "#;

        let path = write_temp_toml(toml);
        let docfig = Docfig::from_file(&path);
        let Err(_) = docfig else { panic!("Config::from_file unexpectedly succeeded"); };
    }

    #[test]
    fn fails_on_invalid_mode()
    {
        let toml = r#"
        [settings]
        target = "src"
        mode = "WRONG"
        "#;

        let path = write_temp_toml(toml);
        let docfig = Docfig::from_file(&path);
        let Err(_) = docfig else { panic!("Config::from_file unexpectedly succeeded"); };
    }

    #[test]
    fn fails_on_duplicate_group_name()
    {
        let toml = r#"
        [settings]
        target = "src"
        mode = "MATCH_FUNCTION_DOCS"

        [[filegroup]]
        name = "a"
        files = ["a.h", "a.c"]

        [[filegroup]]
        name = "a"
        files = ["b.hpp", "b.cc"]
        "#;

        let path = write_temp_toml(toml);
        let docfig = Docfig::from_file(&path);
        let Err(_) = docfig else { panic!("Config::from_file unexpectedly succeeded"); };
    }

    #[test]
    fn fails_on_unknown_fields()
    {
        let toml = r#"
        [settings]
        target = "src"
        mode = "MATCH_FUNCTION_DOCS"

        [invalid]
        something = true
        "#;

        let path = write_temp_toml(toml);
        let docfig = Docfig::from_file(&path);
        let Err(_) = docfig else { panic!("Config::from_file unexpectedly succeeded"); };
    }

    #[test]
    fn fails_on_unknown_settings_field()
    {
        let toml = r#"
        [settings]
        target = "src"
        mode = "MATCH_FUNCTION_DOCS"
        invalid_field = true
        "#;

        let path = write_temp_toml(toml);
        let docfig = Docfig::from_file(&path);
        let Err(_) = docfig else { panic!("Config::from_file unexpectedly succeeded"); };
    }

    #[test]
    fn fails_on_unknown_filegroup_field()
    {
        let toml = r#"
        [settings]
        target = "src"
        mode = "MATCH_FUNCTION_DOCS"

        [[filegroup]]
        name = "a"
        files = ["a.h"]
        invalid_field = true
        "#;

        let path = write_temp_toml(toml);
        let docfig = Docfig::from_file(&path);
        let Err(_) = docfig else { panic!("Config::from_file unexpectedly succeeded"); };
    }
}