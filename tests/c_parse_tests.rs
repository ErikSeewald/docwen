#[cfg(test)]
mod c_parse_tests
{
    use std::{fs, io::Write};
    use std::path::PathBuf;
    use tempfile::tempdir;
    use tree_sitter::{Node, Parser, Tree};
    use docwen::c_parse::{find_declarator, find_function_positions, get_function_id, get_name_and_params, has_definition_ancestor, mask_preprocessor, visit_all_nodes};
    use docwen::docwen_check::FunctionID;
    use once_cell::sync::Lazy;
    use rand::{distr::Alphanumeric, Rng};

    ///Writes the given src to the given tmp dir with the given name.
    fn write(tmp: &tempfile::TempDir, name: &str, src: &str) -> PathBuf
    {
        let p = tmp.path().join(name);
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(src.as_bytes()).unwrap();
        p
    }

    /// Strips all ASCII white-space.
    fn compact(s: &str) -> String
    {
        s.chars().filter(|c| !c.is_ascii_whitespace()).collect()
    }


    /// Parses src and returns the full Tree.
    fn parse_tree(src: &str) -> Tree
    {
        let mut p = Parser::new();
        p.set_language(&tree_sitter_cpp::LANGUAGE.into()).unwrap();
        p.parse(src, None).unwrap()
    }

    /// Finds and returns the first function_declarator in the given tree.
    fn first_decl(tree: &Tree) -> Node
    {
        let mut stack = vec![tree.root_node()];
        while let Some(n) = stack.pop()
        {
            if n.kind() == "function_declarator"
            {
                return n;
            }
            let mut cur = n.walk();
            for child in n.children(&mut cur)
            {
                stack.push(child);
            }
        }
        panic!("No function_declarator found in tree");
    }

    #[test]
    fn mask_preprocessor_preserves_layout()
    {
        const CODE: &str = "#include   <iostream>\n  #pragma once\nint foo();\n";
        let masked = mask_preprocessor(CODE);

        assert!(masked
            .lines()
            .next()
            .unwrap()
            .chars()
            .all(|c| c == ' '));
        assert_eq!(masked.len(), CODE.len());

        // Declaration should still parse
        let tree = parse_tree(&masked);
        let decl = first_decl(&tree);
        let id = get_function_id(decl, &masked, true).unwrap();
        assert_eq!(id.name, "foo");
    }

    #[test]
    fn indented_macros_are_masked()
    {
        const SRC: &str = "   #define X 1\r\nvoid foo();\r\n";
        let masked = mask_preprocessor(SRC);
        assert!(masked.lines().next().unwrap().chars().all(|c| c == ' '));
        assert_eq!(masked.len(), SRC.len());
        let tree = parse_tree(&masked);
        assert!(get_function_id(first_decl(&tree), &masked, true).is_some());
    }

    #[test]
    fn tab_indented_macro_masked()
    {
        const SRC: &str = "\t\t#define ON 1\nint h();";
        let masked = mask_preprocessor(SRC);
        assert!(masked
            .lines()
            .next()
            .unwrap()
            .chars()
            .all(|c| c == ' '));
        assert_eq!(masked.len(), SRC.len());
    }

    #[test]
    fn fuzz_mask_preprocessor_random_macros()
    {
        static DUMMY_DECL: Lazy<String> = Lazy::new(|| "void fuzz();".to_string());

        for _ in 0..1_000
        {
            let rand_len = rand::random_range(1..200);
            let random_body: String = rand::rng()
                .sample_iter(&Alphanumeric)
                .take(rand_len)
                .map(char::from)
                .collect();

            let whitespace: String = ["", " ", "   ", "\t", "\t  "]
                [rand::rng().random_range(0..5)]
                .to_string();
            let macro_line = format!("{whitespace}#{random_body}\n");

            let src = format!("{macro_line}{}\n", *DUMMY_DECL);
            let masked = mask_preprocessor(&src);

            assert_eq!(masked.len(), src.len());

            let mut src_iter = src.lines();
            let _macro_orig = src_iter.next().unwrap();
            let decl_orig = src_iter.next().unwrap();

            let mut masked_iter = masked.lines();
            let _macro_masked = masked_iter.next().unwrap();
            let decl_masked = masked_iter.next().unwrap();

            assert_eq!(decl_orig, decl_masked);

            let tree = parse_tree(&masked);
            assert!(get_function_id(first_decl(&tree), &masked, true).is_some());
        }
    }

    #[test]
    fn multiline_macro_fully_masked()
    {
        const SRC: &str = "#define SUM(a,b) \\\n ((a)+(b))\n\n#define PRINT_VALUES(x, y) \\\n do \
        { \\\n     printf(\"x = %d\\n\", (x)); \\\n     printf(\"y = %d\\n\", (y)); \\\n } \
        while(0)\n\nint foo();\n";

        let masked = mask_preprocessor(SRC);
        assert!(masked
            .lines()
            .take(9)
            .all(|l| l.chars().all(|c| c == ' ')));
        assert_eq!(masked.len(), SRC.len());

        let tree = parse_tree(&masked);
        let id = get_function_id(first_decl(&tree), &masked, true).unwrap();
        assert_eq!(id.name, "foo");
    }

    #[test]
    fn template_plus_trailing_return_type()
    {
        const SRC: &str = r#"
            template<typename T>
            auto add(T a, T b) -> T;
        "#;
        let tree = parse_tree(SRC);
        let id = get_function_id(first_decl(&tree), SRC, true).unwrap();
        assert_eq!(id.name, "add");
        assert!(
            compact(&id.params).starts_with("(Ta,Tb)"),
            "params were: {}",
            id.params
        );
    }

    #[test]
    fn visit_all_nodes_traverses_everything()
    {
        const SRC: &str = "int foo(); struct S { int bar(); };";
        let tree = parse_tree(SRC);
        let mut count = 0usize;
        visit_all_nodes(tree.root_node(), &mut |_| count += 1);
        assert!(count > 3, "Visited only {count} nodes");
    }

    #[test]
    fn same_name_different_params_not_considered_duplicates()
    {
        let tmp = tempdir().unwrap();
        let p1 = write(&tmp, "a.cpp", "void dup(int);");
        let p2 = write(&tmp, "b.cpp", "void dup(double);");
        let map = find_function_positions([p1, p2], true).unwrap();
        assert!(map.is_empty(), "Map should be empty, got {map:?}");
    }

    #[test]
    fn declaration_and_definition_flagged_as_duplicate()
    {
        let tmp = tempdir().unwrap();
        let p1 = write(&tmp, "decl.hpp", "void same();");
        let p2 = write(&tmp, "def.cpp",  "void same() {}");
        let map = find_function_positions([p1.clone(), p2.clone()], true).unwrap();
        assert_eq!(map.len(), 1);
        let fid = FunctionID { name: "same".into(), params: "()".into() };
        let spots = map.get(&fid).expect("Missing key");
        assert_eq!(spots.len(), 2);
        let paths: Vec<_> = spots.iter().map(|p| p.path.clone()).collect();
        assert!(paths.contains(&p1) && paths.contains(&p2));
    }

    #[test]
    fn simple_global_fn_signature()
    {
        const CODE: &str = "int foo(int a, float b);";
        let tree = parse_tree(CODE);
        let decl = first_decl(&tree);
        let (name, params) = get_name_and_params(decl, CODE);

        assert_eq!(name.as_deref(), Some("foo"));
        assert_eq!(compact(params.as_deref().unwrap()), "(inta,floatb)");
    }

    #[test]
    fn namespaced_and_class_scopes()
    {
        const CODE: &str = r#"
            namespace util { struct A { double bar() const; }; }
        "#;
        let tree = parse_tree(CODE);
        let decl = first_decl(&tree);
        let id = get_function_id(decl, CODE, true).unwrap();

        assert_eq!(id.name, "util::A::bar");
        assert_eq!(compact(&id.params), "()");
    }

    #[test]
    fn nested_namespace_and_union_scope()
    {
        const SRC: &str = r#"
            namespace outer::inner {
                union U { void poke(); };
            }
        "#;
        let tree = parse_tree(SRC);
        let id = get_function_id(first_decl(&tree), SRC, true).unwrap();
        assert_eq!(id.name, "outer::inner::U::poke");
        assert_eq!(compact(&id.params), "()");
    }

    #[test]
    fn definition_ancestor_is_detected()
    {
        const CODE: &str = "int foo() { return 0; }";
        let tree = parse_tree(CODE);
        let decl = first_decl(&tree);
        assert!(has_definition_ancestor(decl));
    }

    #[test]
    fn find_declarator_recurses()
    {
        const CODE: &str = "void ns::foo();";
        let tree = parse_tree(CODE);
        assert!(find_declarator(tree.root_node()).is_some());
    }

    #[test]
    fn duplicate_functions_across_files_are_detected()
    {
        let tmp = tempdir().unwrap();
        let p1 = write(&tmp, "f1.cpp", "void dup();");
        let p2 = write(&tmp, "f2.cpp", "void dup();");
        let p3 = write(&tmp, "other.cpp", "void unique();");

        let map = find_function_positions([p1, p2, p3], true).unwrap();
        assert_eq!(map.len(), 1);

        let fid = FunctionID {
            name: "dup".into(),
            params: "()".into(),
        };
        let positions = map.get(&fid).unwrap();
        assert_eq!(positions.len(), 2);

        assert!(positions.iter().all(|p| p.row == 0 && p.column == 5));
    }

    #[test]
    fn test_row_column_offsets()
    {
        let tmp = tempdir().unwrap();

        for _ in 0..100
        {
            let row_offset = rand::random::<u8>() as usize;
            let column_offset = rand::random::<u8>() as usize;

            let mut text = "\n".repeat(row_offset);
            text.push_str(" ".repeat(column_offset).as_str());
            text.push_str("void dup();");

            let p1 = write(&tmp, "f1.cpp", &text);
            let p2 = write(&tmp, "f2.cpp", &text);

            let map = find_function_positions([p1, p2], true).unwrap();

            let fid = FunctionID {
                name: "dup".into(),
                params: "()".into(),
            };
            let positions = map.get(&fid).unwrap();
            assert_eq!(positions.len(), 2);
            assert!(positions.iter().all(|p| p.row == row_offset && p.column == column_offset + 5));
        }
    }

    #[test]
    fn operator_new_array_name()
    {
        const SRC: &str = r#"
            #include <cstddef>
            struct Mem { void* operator new[](std::size_t); };
        "#;
        let tree = parse_tree(SRC);
        let id = get_function_id(first_decl(&tree), SRC, true).unwrap();
        assert_eq!(id.name, "Mem::operator new[]");
    }

    #[test]
    fn default_param_same_string_counts_as_duplicate()
    {
        let tmp = tempdir().unwrap();
        let p1 = write(&tmp, "a.cpp", "void f(int x = 0);");
        let p2 = write(&tmp, "b.cpp", "void f(int x = 0);");

        let map = find_function_positions([p1, p2], true).unwrap();
        assert_eq!(map.len(), 1);
        let fid = FunctionID {
            name: "f".into(),
            params: "(int x = 0)".into(),
        };
        assert_eq!(map[&fid].len(), 2);
    }

    #[test]
    fn default_param_vs_no_default_not_duplicate()
    {
        let tmp = tempdir().unwrap();
        let p1 = write(&tmp, "a.cpp", "void g(int x = 0);");
        let p2 = write(&tmp, "b.cpp", "void g(int x);");

        let map = find_function_positions([p1, p2], true).unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn same_name_different_namespaces_not_duplicate()
    {
        let tmp = tempdir().unwrap();
        let p1 = write(&tmp, "n1.cpp", "namespace n1 { void h(); }");
        let p2 = write(&tmp, "n2.cpp", "namespace n2 { void h(); }");

        let map = find_function_positions([p1, p2], true).unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn variadic_template_pack()
    {
        const SRC: &str = r#"
            template<typename... Args>
            void log(Args&&...);
        "#;
        let tree = parse_tree(SRC);
        let id = get_function_id(first_decl(&tree), SRC, true).unwrap();
        assert_eq!(id.name, "log");
        assert!(
            compact(&id.params).contains("..."),
            "expected pack, got {}", id.params
        );
    }

    #[test]
    fn triplet_duplicates()
    {
        let tmp = tempdir().unwrap();
        let p1 = write(&tmp, "a.cpp", "void triple();");
        let p2 = write(&tmp, "b.cpp", "void triple();");
        let p3 = write(&tmp, "c.cpp", "void triple();");

        let map = find_function_positions([p1, p2, p3], true).unwrap();
        let fid = FunctionID { name: "triple".into(), params: "()".into() };
        assert_eq!(map[&fid].len(), 3);
    }

    #[test]
    fn non_function_returns_none()
    {
        const SRC: &str = "int global_variable;";
        let tree = parse_tree(SRC);
        assert!(get_function_id(tree.root_node(), SRC, true).is_none());
    }

    #[test]
    fn declaration_inside_class_has_no_definition_ancestor()
    {
        const SRC: &str = r#"
            struct S { void foo(); };
        "#;
        let tree = parse_tree(SRC);
        let decl = first_decl(&tree);
        assert!(!has_definition_ancestor(decl));
    }

    #[test]
    fn out_of_line_member_definition_gets_correct_qualified_name()
    {
        const SRC: &str = r#"
            namespace ns { struct C { void bar(); }; }
            void ns::C::bar() {}
        "#;
        let tree = parse_tree(SRC);

        let mut cur = tree.root_node().walk();
        let def = tree
            .root_node()
            .children(&mut cur)
            .find(|n| n.kind() == "function_definition")
            .expect("no definition");

        let decl = def
            .child_by_field_name("declarator")
            .expect("missing declarator");
        let id = get_function_id(decl, SRC, true).unwrap();
        assert_eq!(id.name, "ns::C::bar");
        assert_eq!(compact(&id.params), "()");
    }

    #[test]
    fn qualified_name_contains_template_arguments()
    {
        const SRC: &str = r#"
            template<typename T>
            struct Outer { struct Inner { static void baz(); }; };

            void Outer<int>::Inner::baz() {}
        "#;
        let tree = parse_tree(SRC);
        let decl = first_decl(&tree);
        let id = get_function_id(decl, SRC, true).unwrap();

        assert_eq!(id.name, "Outer<int>::Inner::baz");
    }

    #[test]
    fn parameter_pack_with_nested_namespace_chain()
    {
        const SRC: &str = r#"
            namespace n1::n2 {
                template<typename... Args>
                void log(Args&&...);
            }
        "#;
        let tree = parse_tree(SRC);
        let id = get_function_id(first_decl(&tree), SRC, true).unwrap();
        assert_eq!(id.name, "n1::n2::log");
        assert!(
            compact(&id.params).ends_with("...);") || compact(&id.params).contains("..."),
            "pack missing in params: {}", id.params
        );
    }

    
    #[test]
    fn friend_function_is_captured() 
    {
        const SRC: &str = r#"
            struct W {
                friend void friend_fn(W&);
            };
        "#;
        let tree = parse_tree(SRC);
        let id = get_function_id(first_decl(&tree), SRC, true).unwrap();
        assert_eq!(id.name, "W::friend_fn");
        assert_eq!(compact(&id.params), "(W&)");
    }
    
    #[test]
    fn user_defined_literal_operator()
    {
        const SRC: &str =
            r#"long double operator"" _deg(long double);"#;
        let tree = parse_tree(SRC);
        let id = get_function_id(first_decl(&tree), SRC, true).unwrap();
        assert!(id.name.contains("_deg"), "name was {}", id.name);
        assert!(id.name.starts_with("operator"));
    }
    
    #[test]
    fn placement_new_operator() 
    {
        const SRC: &str = r#"
            #include <cstddef>
            void* operator new(std::size_t, void* place);
        "#;
        let tree = parse_tree(SRC);
        let id = get_function_id(first_decl(&tree), SRC, true).unwrap();
        assert_eq!(id.name, "operator new");
        assert!(compact(&id.params).starts_with("(std::size_t"));
    }
    
    #[test]
    fn constrained_template_function() 
    {
        const SRC: &str = r#"
            template<typename T>
                requires sizeof(T) > 0
            void constrained_fn(T);
        "#;
        let tree = parse_tree(SRC);
        let id = get_function_id(first_decl(&tree), SRC, true).unwrap();
        assert_eq!(id.name, "constrained_fn");
        assert!(compact(&id.params).starts_with("(T"));
    }

    #[test]
    fn mismatches_unqualified_correctly()
    {
        let tmp = tempdir().unwrap();
        let p1 = write(&tmp, "a.cpp", "void A::f(int x = 0);");
        let p2 = write(&tmp, "b.cpp", "void B::f(int x = 0);");

        let map = find_function_positions([p1, p2], true).unwrap();
        assert_eq!(map.len(), 0);
    }

    #[test]
    fn matches_unqualified_correctly()
    {
        let tmp = tempdir().unwrap();
        let p1 = write(&tmp, "a.cpp", "void A::f(int x = 0);");
        let p2 = write(&tmp, "b.cpp", "void B::f(int x = 0);");
        let p3 = write(&tmp, "cd.cpp", "void C::D::f(int x = 0);");
        let p4 = write(&tmp, "f.cpp", "void f(int x = 0);");

        let map = find_function_positions([p1, p2, p3, p4], false).unwrap();
        assert_eq!(map.len(), 1);
        let fid = FunctionID {
            name: "f".into(),
            params: "(int x = 0)".into(),
        };
        assert_eq!(map[&fid].len(), 4);
    }
}