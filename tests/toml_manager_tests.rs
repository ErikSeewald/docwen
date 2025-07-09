#[cfg(test)]
mod toml_manager_tests
{
    use std::fs;
    use std::path::PathBuf;
    use tempfile::{tempdir, NamedTempFile};
    use docwen::docfig::Mode::MatchFunctionDocs;
    use docwen::docfig::{Docfig, Settings};
    use docwen::toml_manager::*;

    #[test]
    fn create_default_writes_new_file()
    {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("docwen.toml");

        create_default(&file_path).unwrap();

        let written = fs::read_to_string(&file_path).unwrap();
        assert_eq!(written, DEFAULT_TOML);
    }

    #[test]
    fn create_default_fails_if_file_exists()
    {
        let tmp = NamedTempFile::new().unwrap();
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

    #[test]
    fn group_by_stem_empty_match_extensions()
    {
        let settings = make_settings(&[], &[]);
        let paths = vec![PathBuf::from("foo.c"), PathBuf::from("bar.h")];

        let groups = group_by_stem(paths, &settings);
        assert!(groups.is_empty());
    }

    #[test]
    fn group_by_stem_ignore_list_case_insensitive()
    {
        let settings = make_settings(&["c"], &["skipme"]);
        let paths = vec![PathBuf::from("SkipMe.c"), PathBuf::from("keepme.c")];

        let names: std::collections::HashSet<_> =
            group_by_stem(paths, &settings).into_iter().map(|g| g.name).collect();

        assert!(!names.contains("skipme"));
        assert!(names.contains("keepme"));
    }
    
    #[test]
    fn group_by_stem_handles_dot_files() 
    {
        let settings = make_settings(&["c"], &[]);
        let paths = vec![PathBuf::from(".hidden.c")];

        let groups = group_by_stem(paths, &settings);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, ".hidden");
    }

    #[test]
    fn update_toml_creates_and_updates_groups()
    {
        let dir = tempdir().unwrap();
        let root = dir.path().join("src");
        fs::create_dir(&root).unwrap();

        let c_path = root.join("foo.c");
        let h_path = root.join("foo.h");

        fs::write(&c_path, "").unwrap();
        fs::write(&h_path, "").unwrap();

        let toml_path = dir.path().join("docwen.toml");
        create_default(&toml_path).unwrap();

        update_toml(&toml_path).unwrap();
        let docfig = Docfig::from_file(&toml_path).unwrap();
        let files = &docfig.file_groups.get(0).unwrap().files;
        assert!(files.contains(&PathBuf::from(c_path.strip_prefix(&root).unwrap()))
            && files.contains(&PathBuf::from(h_path.strip_prefix(&root).unwrap())));
    }

    #[test]
    fn update_toml_augments_group_without_duplication()
    {
        let dir  = tempdir().unwrap();
        let root = dir.path().join("src");
        fs::create_dir(&root).unwrap();

        fs::write(root.join("foo.c"), "").unwrap();
        fs::write(root.join("foo.h"), "").unwrap();

        let toml_path = dir.path().join("docwen.toml");
        create_default(&toml_path).unwrap();

        // append a group ` with only foo.c
        let mut contents = fs::read_to_string(&toml_path).unwrap();
        contents.push_str(r#"
        [[filegroup]]
        name = "foo"
        files = ["src/foo.c"]
        "#);
        fs::write(&toml_path, contents).unwrap();

        update_toml(&toml_path).unwrap();
        let docfig: Docfig = Docfig::from_file(&toml_path).unwrap();
        let foo_groups: Vec<_> =
            docfig.file_groups.iter().filter(|g| g.name == "foo").collect();

        assert_eq!(foo_groups.len(), 1, "Group duplicated unexpectedly");
        assert_eq!(
            foo_groups[0].files.len(),
            2,
            "Group was not augmented to include new file"
        );
    }

    #[test]
    fn update_toml_deep_paths()
    {
        let dir = tempdir().unwrap();
        let root = dir.path().join("src");
        let some_dir = root.join("somedir");
        let even_more = root.join("someotherdir").join("evenmore");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&some_dir).unwrap();
        fs::create_dir_all(&even_more).unwrap();

        let c_path = some_dir.join("foo.c");
        let h_path = root.join(even_more).join("foo.h");

        fs::write(&c_path, "").unwrap();
        fs::write(&h_path, "").unwrap();

        let toml_path = dir.path().join("docwen.toml");
        create_default(&toml_path).unwrap();

        update_toml(&toml_path).unwrap();
        let docfig = Docfig::from_file(&toml_path).unwrap();
        let files = &docfig.file_groups.get(0).unwrap().files;
        assert!(files.contains(&PathBuf::from(c_path.strip_prefix(&root).unwrap()))
            && files.contains(&PathBuf::from(h_path.strip_prefix(&root).unwrap())));
    }


    #[test]
    fn update_toml_does_not_delete()
    {
        let dir  = tempdir().unwrap();
        let toml_path = dir.path().join("docwen.toml");
        create_default(&toml_path).unwrap();

        // append a group ` with only foo.c
        let mut contents = fs::read_to_string(&toml_path).unwrap();
        contents.push_str(r#"
        [[filegroup]]
        name = "foo"
        files = ["src/foo.c"]
        "#);
        fs::write(&toml_path, contents).unwrap();

        update_toml(&toml_path).unwrap();
        let docfig: Docfig = Docfig::from_file(&toml_path).unwrap();

        assert_eq!(docfig.file_groups.len(), 1, "Group was deleted unexpectedly");
        assert_eq!(
            docfig.file_groups[0].files.len(),
            1,
            "Group size changed unexpectedly"
        );
    }

    #[test]
    fn get_absolute_root_resolves_relative_target()
    {
        let toml_path = PathBuf::from("/home/user/project/docwen.toml");
        let target = PathBuf::from("src");
        let abs = get_absolute_root(&toml_path, &target).unwrap();

        assert_eq!(abs, PathBuf::from("/home/user/project/src"));
    }

    #[test]
    fn get_absolute_root_passes_through_absolute_path()
    {
        let target = PathBuf::from("/var/tmp/codebase");
        let result = get_absolute_root("some/path/docwen.toml", &target).unwrap();

        assert_eq!(result, target);
    }

    #[test]
    fn get_absolute_root_with_complex_relative_segments()
    {
        let toml_path = PathBuf::from("/home/user/project/dir/docwen.toml");
        let target    = PathBuf::from("../src/./backend");
        let abs = get_absolute_root(&toml_path, &target).unwrap();
        
        assert_eq!(abs, PathBuf::from("/home/user/project/dir/../src/./backend"));
    }

    #[test]
    fn create_default_fails_if_path_is_dir()
    {
        let dir = tempdir().unwrap();
        let err = create_default(dir.path()).unwrap_err();
        assert!(
            err.to_string().contains("Failed to create new docwen.toml"),
            "Unexpected error: {err}"
        );
    }

    #[test]
    fn create_default_fails_if_parent_missing()
    {
        let mut path = std::env::temp_dir();
        path.push("___missing___/docwen.toml");

        let err = create_default(&path).unwrap_err();
        assert!(
            err.to_string().contains("Failed to create new docwen.toml"),
            "Unexpected error: {err}"
        );
    }
}
