//! Handles parsing c/c++ code

use std::path::PathBuf;
use tree_sitter::{Parser, Node};
use std::{collections::HashMap, fs, iter};
use anyhow::Context;
use crate::docwen_check::{FilePosition, FunctionID};

/// Finds all function matches (based on qualifiers, name and parameters)
/// in the given list of files. Maps them by FunctionID -> Vec<FilePosition>.
/// 'use_qualifiers' specifies whether qualifiers should be used to differentiate as well
pub fn find_function_positions<I>(paths: I, use_qualifiers: bool)
    -> anyhow::Result<HashMap<FunctionID, Vec<FilePosition>>>
where
    I: IntoIterator<Item = PathBuf>,
{
    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_cpp::LANGUAGE.into())?;

    let mut functions: HashMap<FunctionID, Vec<FilePosition>> = HashMap::new();
    for path in paths
    {
        let source = fs::read_to_string(&path)?;

        let filtered: String = mask_preprocessor(&source);
        let tree = parser.parse(&filtered, None).with_context(|| "Failed to parse tree")?;

        let root = tree.root_node();
        extract_functions(root, &filtered, path, &mut functions, use_qualifiers);
    }

    functions.retain(|_, vec| vec.len() > 1 );
    Ok(functions)
}

/// Extracts all functions from the tree spanned by the given root node.
/// Uses the given source text and file path to insert the functions into the given map.
/// 'use_qualifiers' defines whether qualifiers are used to differentiate functions instead of
/// basic name and param matching.
pub fn extract_functions(root: Node, source: &str, file: PathBuf,
                     map: &mut HashMap<FunctionID, Vec<FilePosition>>, use_qualifiers: bool)
{
    // Recursively visit all nodes and apply the function
    visit_all_nodes(root, &mut |node| {
        match node.kind()
        {
            "function_definition" | "function_declarator" if !has_definition_ancestor(node) =>
                {
                    if let Some(id) = get_function_id(node, source, use_qualifiers)
                    {
                        let pos = FilePosition{
                            path: file.clone(),
                            row: node.start_position().row,
                            column: node.start_position().column
                        };

                        let entry = map.entry(id).or_insert(Vec::new());
                        entry.push(pos);
                    }
                },

                _ => {}
        }
    });
}

/// Returns the full (optionally: qualified) function signature as a FunctionID.
/// If no FunctionID can be derived from the given node, None is returned.
pub fn get_function_id(node: Node, source: &str, with_qualifiers: bool) -> Option<FunctionID>
{
    let declarator = find_declarator(node)?;

    let (name_option, params) = get_name_and_params(declarator, source);
    let name = name_option?;
    let params = params.unwrap_or_else(|| String::from("()"));

    if with_qualifiers
    {
        let qualified_name = get_qualified_name(node, source, name);
        Some(FunctionID{name: qualified_name, params})
    }
    else
    {
        let unqualified = String::from(name.split("::").last().unwrap_or(&name));
        Some(FunctionID{name: unqualified, params})
    }
}

/// Returns whether the given node has a 'function_definition' as an ancestor.
/// This way you can avoid tracking a function twice.
pub fn has_definition_ancestor(mut n: Node) -> bool
{
    while let Some(parent) = n.parent()
    {
        if parent.kind() == "function_definition" { return true; }
        n = parent;
    }
    false
}

/// Walks from the given node until the function_declarator is found.
/// Returns None if it could not be found.
pub fn find_declarator(n: Node) -> Option<Node>
{
    if n.kind() == "function_declarator"
    {
        return Some(n);
    }
    let mut cur = n.walk();
    for child in n.children(&mut cur)
    {
        if let Some(d) = find_declarator(child)
        {
            return Some(d);
        }
    }
    None
}

/// Gets ((optional) Name, (optional) Params) of the given declarator node based on the given
/// source text.
pub fn get_name_and_params(declarator: Node, source: &str) -> (Option<String>, Option<String>)
{
    let mut cur = declarator.walk();
    let mut name: Option<String>   = None;
    let mut params: Option<String> = None;

    // WALK THROUGH DECLARATOR TO FIND NAME AND PARAMS
    for child in declarator.children(&mut cur)
    {
        match child.kind()
        {
            "identifier" | "qualified_identifier" | "operator_name" |
            "field_identifier" | "destructor_name" =>
                {
                    if let Ok(txt) = child.utf8_text(source.as_bytes())
                    {
                        name = Some(txt.to_string())
                    }
                },

            "parameter_list" =>
                {
                    if let Ok(txt) = child.utf8_text(source.as_bytes())
                    {
                        params = Some(txt.to_string())
                    }
                },
            _ => {}
        }
    }
    (name, params)
}

/// Formats the given func_name with all its scope qualifiers based on the given
/// source text and starting node.
pub fn get_qualified_name(node: Node, source: &str, func_name: String) -> String
{
    let mut qualifiers = Vec::<String>::new();
    let mut current = node;

    while let Some(parent) = current.parent()
    {
        match parent.kind()
        {
            "class_specifier" | "struct_specifier" | "union_specifier" | "namespace_definition" =>
                {
                    if let Some(id) = parent.child_by_field_name("name")
                    {
                        if let Ok(txt) = id.utf8_text(source.as_bytes())
                        {
                            qualifiers.push(txt.to_string());
                        }
                    }
                }

            _ => {}
        }
        current = parent;
    }

    qualifiers.reverse();
    if qualifiers.is_empty() { func_name }  else {
        format!("{}::{}", qualifiers.join("::"), func_name)
    }
}

/// Masks out all preprocessor sections of the given src by replacing
/// them with whitespace that preserves column and row positioning.
/// - Lines that begin (after optional whitespace) with '#' are masked.
/// - Any following lines that continue via a trailing backslash are also masked.
pub fn mask_preprocessor(src: &str) -> String
{
    let mut out = String::with_capacity(src.len());
    let mut in_continuation = false;

    // HANDLE EACH LINE SEPARATELY
    for line in src.split_inclusive(['\n', '\r'])
    {
        // SPLIT BODY FROM END OF LINE
        let (body, eol) = match line.strip_suffix('\n')
        {
            Some(rest) =>
                {
                    match rest.strip_suffix('\r')
                    {
                        Some(r) => (r, "\r\n"),
                        None => (rest, "\n"),
                    }
                },

            None => (line, ""), // Last line of file, no newline
        };

        let starts_with_hash = body.trim_start().starts_with('#');
        if in_continuation || starts_with_hash
        {
            out.extend(iter::repeat(' ').take(body.len()));
            in_continuation = body.as_bytes().last() == Some(&b'\\');
        }
        else
        {
            out.push_str(body);
            in_continuation = false;
        }

        out.push_str(eol);
    }

    out
}

/// Performs the given FnMut(Node) on all descendents of the given node recursively
pub fn visit_all_nodes<F>(node: Node, visit: &mut F)
where
    F: FnMut(Node),
{
    let mut cursor = node.walk();
    visit(node);
    for child in node.children(&mut cursor)
    {
        visit_all_nodes(child, visit);
    }
}