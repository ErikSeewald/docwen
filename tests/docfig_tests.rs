#[cfg(test)]
mod docfig_tests
{
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::{NamedTempFile, TempPath};
    use docwen::docfig::*;

    const MINIMAL_VALID_TOML: &str = r#"
        [settings]
        target = "src"
        mode = "MATCH_FUNCTION_DOCS"
        "#;

    fn write_temp_toml(content: &str) -> TempPath
    {
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(content.as_bytes()).unwrap();
        tmp.flush().unwrap();
        tmp.into_temp_path()
    }

    fn random_valid_toml() -> String
    {
        let group_count = rand::random::<u8>() % 16;
        let groups = (0..group_count).map(|i| format!(
            r#"
        [[filegroup]]
        name = "group_{i}"
        files = ["file_{i}.h", "file_{i}.c"]
        "#
        )).collect::<String>();

        format!(
            r#"
        [settings]
        target = "src"
        mode = "MATCH_FUNCTION_DOCS"
        match_extensions = ["h", "c"]
        manual = ["temp", "test"]

        {}
        "#,
            groups
        )
    }

    #[test]
    fn fuzz_random_valid_tomls()
    {
        for _ in 0..100
        {
            let toml_str = random_valid_toml();
            let parsed = toml::from_str::<Docfig>(&toml_str);
            assert!(parsed.is_ok(), "Failed on TOML:\n{}", toml_str);
        }
    }

    #[test]
    fn parses_full_config()
    {
        let toml = r#"
        [settings]
        target = "src"
        match_extensions = ["h", "c"]
        mode = "MATCH_FUNCTION_DOCS"
        manual = ["some", "thing"]

        [[filegroup]]
        name = "a"
        files = ["a.h", "a.c"]

        [[filegroup]]
        name = "b"
        files = ["b.hpp", "b.cc"]
        "#;

        let path = write_temp_toml(toml);
        let docfig = Docfig::from_file(&path).unwrap();

        // SETTINGS
        assert_eq!(docfig.settings.target, PathBuf::from("src"));
        assert_eq!(docfig.settings.match_extensions, vec!["h", "c"]);
        matches!(docfig.settings.mode, Mode::MatchFunctionDocs);
        assert_eq!(docfig.settings.manual, vec!["some", "thing"]);

        // FILE_GROUPS
        assert_eq!(docfig.file_groups.len(), 2);
        assert_eq!(docfig.file_groups[0].name, "a");
        assert_eq!(docfig.file_groups[0].files, vec![PathBuf::from("a.h"), PathBuf::from("a.c")]);
    }

    #[test]
    fn parses_minimal_config()
    {
        let path = write_temp_toml(MINIMAL_VALID_TOML);
        let docfig = Docfig::from_file(&path).unwrap();

        assert!(docfig.settings.match_extensions.is_empty());
        assert!(docfig.settings.manual.is_empty());
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
        let docfig = Docfig::from_file(&path).unwrap();

        assert!(docfig.settings.match_extensions.is_empty());
        assert!(docfig.settings.manual.is_empty());
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
    fn fails_on_missing_mode()
    {
        let toml = r#"
        [settings]
        target = "src"
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
    fn fails_when_settings_section_absent()
    {
        let toml = r#"
        [[filegroup]]
        name = "only"
        files = ["x.h"]
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

    #[test]
    fn roundtrip_does_not_change_config()
    {
        let path_in  = write_temp_toml(MINIMAL_VALID_TOML);
        let docfig_in   = Docfig::from_file(&path_in).unwrap();

        let tmp_out = NamedTempFile::new().unwrap();
        docfig_in.write_file(tmp_out.path()).unwrap();

        let docfig_out = Docfig::from_file(tmp_out.path()).unwrap();
        assert_eq!(docfig_in, docfig_out);
    }

    #[test]
    fn filegroup_eq_ignores_files()
    {
        let a1 = FileGroup { name: "foo".into(), files: vec![PathBuf::from("a.h")] };
        let a2 = FileGroup { name: "foo".into(), files: vec![PathBuf::from("x.cpp"), PathBuf::from("y.rs")] };
        let b  = FileGroup { name: "bar".into(), files: vec![PathBuf::from("a.h")] };

        assert_eq!(a1, a2);
        assert_ne!(a1, b);
    }

    #[test]
    fn fails_when_file_cannot_be_read()
    {
        let path = PathBuf::from("this/file/does/not/exist.toml");
        let Err(e) = Docfig::from_file(&path) else { panic!("Expected error"); };
        assert!(e.to_string().contains("Failed to read"));
    }
}