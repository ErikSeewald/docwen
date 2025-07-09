#[cfg(test)]
mod docwen_check_tests
{
    use std::collections::HashSet;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::io::Write;
    use tempfile::tempdir;
    use docwen::docwen_check;
    use docwen::docwen_check::{format_mismatch, FilePosition, FunctionID, LineSource};

    /// Creates a FilePosition from the arguments
    fn fp(path: &str, row: usize, column: usize) -> FilePosition
    {
        FilePosition {
            path: PathBuf::from(path),
            row,
            column,
        }
    }

    /// Writes 'content' to 'path', creates parent dirs as needed.
    fn write_file<P: AsRef<Path>>(path: P, content: &str)
    {
        if let Some(parent) = path.as_ref().parent()
        {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    /// Produce a single '[[filegroup]]' for the given files.
    fn toml_group(files: &[&str]) -> String
    {
        let list = files
            .iter()
            .map(|f| format!("\"{f}\""))
            .collect::<Vec<_>>()
            .join(", ");
        format!("[[filegroup]]\nname = \"{}\"\nfiles = [{list}]\n", files[0])
    }

    /// Creates a throw-away workspace on disk:
    ///   * `file_specs` (`relative path`, `file contents`)
    ///   * `groups` slice of slices grouping the files
    /// Returns the absolute path to the new `docwen.toml`.
    fn workspace(file_specs: &[(&str, &str)], groups: &[&[&str]], ) -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        for (file, contents) in file_specs
        {
            write_file(dir.path().join(file), contents);
        }

        let mut toml = String::from("[settings]\ntarget = \".\"\n mode=\"MATCH_FUNCTION_DOCS\"\n\n");
        toml.extend(groups.iter().cloned().map(toml_group));

        write_file(dir.path().join("docwen.toml"), &toml);
        dir
    }

    /// Calls 'check' and unwraps the 'Ok(_)' so
    /// the test can work directly with the returned vector.
    macro_rules! run_check
    {
        ($toml:expr) => {{
            let res = docwen_check::check(&$toml);
            assert!(res.is_ok(), "check() returned Err: {res:?}");
            res.unwrap()
        }};
    }


    #[test]
    fn formats_relative_path()
    {
        let target_path = PathBuf::from("project");
        let positions = vec![fp("project/src/lib.c", 42, 7)];

        let expected = format!(
            "\"{}\"\n-> [{}]",
            "needle",
            format!("{:?}:42:7", PathBuf::from("src/lib.c"))
        );

        assert_eq!(format_mismatch("needle", &positions, &target_path), expected);
    }

    #[test]
    fn format_keeps_full_path_for_unrelated_files()
    {
        let target_path = PathBuf::from("project");
        let position_path = "other_workspace/src/main.c";
        let positions = vec![fp(position_path, 1, 0)];

        let expected = format!(
            "\"{}\"\n-> [{}]",
            "token",
            format!("{:?}:1:0", PathBuf::from(position_path))
        );

        assert_eq!(format_mismatch("token", &positions, &target_path), expected);
    }

    #[test]
    fn format_joins_multiple_positions_with_comma()
    {
        let target_path = PathBuf::from("project");
        let positions = vec![
            fp("project/src/foo.h", 10, 1),
            fp("project/src/foo.c", 11, 2),
        ];

        let expected_group = vec![
            format!("{:?}:10:1", PathBuf::from("src/foo.h")),
            format!("{:?}:11:2", PathBuf::from("src/foo.c")),
        ]
            .join(", ");

        let expected = format!("\"{}\"\n-> [{}]", "multi", expected_group);

        assert_eq!(format_mismatch("multi", &positions, &target_path), expected);
    }

    #[test]
    fn format_handles_empty_vector()
    {
        let target_path = PathBuf::from("project");
        let positions: Vec<FilePosition> = Vec::new();

        assert_eq!(
            format_mismatch("nothing", &positions, &target_path),
            "\"nothing\"\n-> []"
        );
    }

    #[test]
    fn line_source_returns_trimmed_line() -> anyhow::Result<()>
    {
        let src = "   // doc comment   \nfn foo() {}\n";
        let ls = LineSource { src: src.into(), init_row: 1 };

        let line = ls.trimmed_line_by_offset(-1);
        assert_eq!(line, "// doc comment");

        let same_line = ls.trimmed_line_by_offset(0);
        assert_eq!(same_line, "fn foo() {}");
        Ok(())
    }

    #[test]
    fn line_source_out_of_bounds_is_empty()
    {
        let ls = LineSource { src: "only-line".into(), init_row: 0 };

        let out1 = ls.trimmed_line_by_offset(-1);
        assert_eq!(out1, "", "Negative index should return empty");

        let out2 = ls.trimmed_line_by_offset(5);
        assert_eq!(out2, "", "Out of bounds index should return empty");
    }

    #[test]
    fn function_id_equality_and_hashing()
    {
        let f1 = FunctionID { qualified_name: "pkg::foo".into(), params: "(i32)".into() };
        let f2 = FunctionID { qualified_name: "pkg::foo".into(), params: "(i32)".into() };
        let f3 = FunctionID { qualified_name: "pkg::foo".into(), params: "(i32, i32)".into() };
        let f4 = FunctionID { qualified_name: "pkg::bar".into(), params: "(i32)".into() };

        assert_eq!(f1, f2);
        assert_ne!(f1, f3);
        assert_ne!(f1, f4);

        let mut set = HashSet::new();
        set.insert(f1);
        set.insert(f2);
        set.insert(f3);
        assert_eq!(set.len(), 2);
        assert!(set.contains(&FunctionID {
            qualified_name: "pkg::foo".into(),
            params: "(i32)".into()
        }));
    }

    #[test]
    fn check_detects_mismatching_docs() -> anyhow::Result<()>
    {
        let dir = tempdir()?;
        let src_a = dir.path().join("a.c");
        let src_b = dir.path().join("b.c");
        let mut fa = fs::File::create(&src_a)?;
        let mut fb = fs::File::create(&src_b)?;

        writeln!(fa, "// one\nint foo();")?;
        writeln!(fb, "// two\nint foo();")?;

        let toml_path = dir.path().join("docwen.toml");
        fs::write(
            &toml_path,
            r#"
            [settings]
            target = "."
            mode = "MATCH_FUNCTION_DOCS"

            [[filegroup]]
            name = "a"
            files = ["a.c", "b.c"]
            "#,
        )?;

        let mismatches = docwen_check::check(&toml_path)?;
        assert_eq!(mismatches.len(), 1);
        assert!(
            mismatches[0].contains("// one") || mismatches[0].contains("// two"),
            "Mismatch should mention the offending line"
        );
        Ok(())
    }

    #[test]
    fn check_reports_zero_mismatches_with_identical_docs()
    {
        let code = "\n// doc line 1\n// doc line 2\nint foo() { return 0; }\n";
        let dir = workspace(
            &[
                ("a.c", code),
                ("b.c", code),
            ],
            &[&["a.c", "b.c"]],
        );

        let toml_path = dir.path().join("docwen.toml");
        let mismatches = run_check!(toml_path);
        assert!(mismatches.is_empty(), "Expected zero mismatches");
    }

    #[test]
    fn check_detects_single_mismatch()
    {
        let a = "\n// doc line 1\n// shared line\nint foo() { return 0; }\n";
        let b = "\n// doc line 1\n// ***DIFFERENT***\nint foo() { return 0; }\n";
        let dir = workspace(
            &[("a.c", a), ("b.c", b)],
            &[&["a.c", "b.c"]],
        );

        let mismatches = run_check!(dir.path().join("docwen.toml"));
        assert_eq!(mismatches.len(), 1, "Should see exactly one mismatch");
        assert!(
            mismatches[0].contains("***DIFFERENT***") || mismatches[0].contains("shared line"),
            "Mismatch should output one of the differing lines: {:?}",
            mismatches
        );
        assert!(mismatches[0].contains("a.c"));
        assert!(mismatches[0].contains("b.c"));
    }

    #[test]
    fn check_multiple_groups_yield_multiple_mismatches()
    {
        let dir = workspace(
            &[
                // group 1
                ("g1/x.c", "\n// X1\nint foo() {}\n"),
                ("g1/y.c", "\n// X2\nint foo() {}\n"),

                // group 2
                ("g2/u.c", "\n// U\nint bar() {}\n"),
                ("g2/v.c", "\n// V\nint bar() {}\n"),
            ],
            &[
                &["g1/x.c", "g1/y.c"],
                &["g2/u.c", "g2/v.c"],
            ],
        );

        let mismatches = run_check!(dir.path().join("docwen.toml"));
        assert_eq!(
            mismatches.len(),
            2,
            "Each mismatching group should be one entry"
        );
    }

    #[test]
    fn check_flags_mixed_comment_styles()
    {
        let a = "\n// style slash\nint foo() {}\n";
        let b = "\n/* style block */\nint foo() {}\n";
        let dir = workspace(
            &[("a.c", a), ("b.c", b)],
            &[&["a.c", "b.c"]],
        );

        let mismatches = run_check!(dir.path().join("docwen.toml"));
        assert_eq!(mismatches.len(), 1);
        assert!(
            mismatches[0].contains("style slash") || mismatches[0].contains("style block"),
            "should mention one of the comment lines"
        );
    }

    #[test]
    fn check_ignores_whitespace_differences()
    {
        let a = "\n   // padded   \nint foo() {}\n";
        let b = "\n// padded\nint foo() {}\n";
        let dir = workspace(
            &[("a.c", a), ("b.c", b)],
            &[&["a.c", "b.c"]],
        );

        let mismatches = run_check!(dir.path().join("docwen.toml"));
        assert!(
            mismatches.is_empty(),
            "whitespace-only differences should not be reported"
        );
    }

    #[test]
    fn check_reports_per_function_not_per_file()
    {
        let common = "\n// ok\nint matched() { return 0; }\n";
        let a = format!("{common}\n// mismatchA\nint bad() {{}}\n");
        let b = format!("{common}\n// mismatchB\nint bad() {{}}\n");

        let dir = workspace(
            &[("a.c", &a), ("b.c", &b)],
            &[&["a.c", "b.c"]],
        );

        let mismatches = run_check!(dir.path().join("docwen.toml"));
        assert_eq!(mismatches.len(), 1, "Only the mismatching function line");
        assert!(mismatches[0].contains("mismatchA") || mismatches[0].contains("mismatchB"));
    }

    #[test]
    fn check_detects_missing_docs()
    {
        let a = "\n// doc only in A\nint foo() {}\n";
        let b = "\nint foo() {}\n";
        let dir = workspace(&[("a.c", a), ("b.c", b)], &[&["a.c", "b.c"]]);

        let mismatches = run_check!(dir.path().join("docwen.toml"));
        assert_eq!(mismatches.len(), 1);
        assert!(mismatches[0].contains("doc only in A"));
    }

    #[test]
    fn check_detects_extra_doc_line()
    {
        let a = "\n// doc line 1\n// doc line 2\nint foo() {}\n";
        let b = "\n// doc line 1\nint foo() {}\n"; // one line fewer
        let dir = workspace(&[("a.c", a), ("b.c", b)], &[&["a.c", "b.c"]]);

        let mismatches = run_check!(dir.path().join("docwen.toml"));
        assert_eq!(mismatches.len(), 1);
        assert!(
            mismatches[0].contains("doc line 2"),
            "Should mention the offending line with the extra text"
        );
    }

    #[test]
    fn check_detects_mismatch_with_three_files()
    {
        let good = "\n// ok doc\nint foo() {}\n";
        let bad = "\n// WRONG doc\nint foo() {}\n";
        let dir = workspace(
            &[("x.c", good), ("y.c", good), ("z.c", bad)],
            &[&["x.c", "y.c", "z.c"]],
        );

        let mismatches = run_check!(dir.path().join("docwen.toml"));
        assert_eq!(mismatches.len(), 1);
        assert!(
            mismatches[0].contains("WRONG doc") || mismatches[0].contains("ok doc"),
            "Output should show the divergent line"
        );
    }

    #[test]
    fn check_reports_multiple_mismatches_in_same_group()
    {
        let a = "\n// A1\nint foo() {}\n// B1\nint bar() {}\n";
        let b = "\n// A2\nint foo() {}\n// B2\nint bar() {}\n";
        let dir = workspace(&[("a.c", a), ("b.c", b)], &[&["a.c", "b.c"]]);

        let mismatches = run_check!(dir.path().join("docwen.toml"));
        assert_eq!(mismatches.len(), 2, "One entry per mismatching function");
        assert!(mismatches.iter().any(|m| m.contains("A1") || m.contains("A2")));
        assert!(mismatches.iter().any(|m| m.contains("B1") || m.contains("B2")));
    }

    #[test]
    fn check_all_good_with_block_comments()
    {
        let code = "\n/* block style */\nint foo() {}\n";
        let dir = workspace(&[("a.c", code), ("b.c", code)], &[&["a.c", "b.c"]]);

        let mismatches = run_check!(dir.path().join("docwen.toml"));
        assert!(
            mismatches.is_empty(),
            "Identical block comments must not be flagged"
        );
    }
}