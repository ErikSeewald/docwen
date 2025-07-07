//! Implements the doc match check functionality of docwen

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use crate::c_parse;
use crate::toml_parse::{Docfig};

/// Defines a position (column, row) inside a source file.
#[derive(Debug)]
pub struct FilePosition
{
    pub path: PathBuf,
    pub row: usize,
    pub column: usize
}

/// Defines an ID for a function qualified name and params.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct FunctionID
{
    pub qualified_name: String,
    pub params: String
}

pub fn check(toml_path: impl AsRef<Path>) -> anyhow::Result<Vec<String>>
{
    let mut mismatches: Vec<String> = Vec::new();

    // GET DOCFIG FROM TOML
    let docfig = Docfig::from_file(toml_path)?;

    // GET ALL FUNCTION POSITIONS THAT NEED TO BE CHECKED
    let mut position_maps: Vec<HashMap<FunctionID, Vec<FilePosition>>> = Vec::new();
    for file_group in docfig.file_groups
    {
        position_maps.push(c_parse::find_function_positions(file_group.files)?);
    }

    // CHECK FOR MATCHING DOCS
    for map in position_maps
    {
        for (id, vec) in map
        {
            // (Lines, starting_row)
            let sources: Vec<(String, usize)> = vec.iter()
                .map(|f| (fs::read_to_string(&f.path).expect("Failed to read source"), f.row))
                .collect::<Vec<_>>();

            let mut offset = 1; // Begin at the line directly above the function
            let mut cur_lines: Vec<&str> = sources.iter()
                .map(|s| s.0.lines().nth(s.1 - offset).unwrap()).collect::<Vec<_>>();

            // Check each comment line individually
            while cur_lines.clone().into_iter().any(is_comment)
            {
                if !doc_lines_match(cur_lines)
                {
                    mismatches.push(format!("{}{} <-- {:?}", id.qualified_name, id.params,
                                            vec.iter().map(|v| &v.path).collect::<HashSet<_>>()));
                    break;
                }
                offset += 1;
                cur_lines = sources.iter()
                    .map(|s| s.0.lines().nth(s.1 - offset).unwrap()).collect::<Vec<_>>();
            }
        }
    }

    Ok(mismatches)
}

/// Returns whether all lines in the given vec are matching comment lines.
/// Whitespace is trimmed before checking for equality.
fn doc_lines_match(lines: Vec<&str>) -> bool
{
    let first = lines.first().unwrap().trim();
    !lines.iter().any(|f| f.trim() != first)
}

/// Returns whether the given line is a comment line according to docwen c/c++ rules.
fn is_comment(line: &str) -> bool
{
    let s = line.trim();
    s.starts_with("//") || s.starts_with("/*") || s.starts_with("*")
}