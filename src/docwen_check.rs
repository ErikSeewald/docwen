//! Implements the doc match check functionality of docwen

use std::collections::{HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Context;
use crate::{c_parse, toml_manager};
use crate::docfig::{Docfig};

/// Defines a position (column, row) inside a source file.
#[derive(Debug)]
pub struct FilePosition
{
    pub path: PathBuf,
    pub row: usize,
    pub column: usize
}

/// Defines an ID for a function through the qualified name and params.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct FunctionID
{
    pub qualified_name: String,
    pub params: String
}

/// Defines a structure used by the doc checker for indexing into the
/// src String by an offset to the init_row.
/// Generally, the docs are in [init_row-1, init_row-n] for docs of line length n.
pub struct LineSource
{
    pub src: String, // String containing the source file text
    pub init_row: usize, // The initial row in the src string (directly below docs)
}

impl LineSource
{
    /// Trims and returns the src line at the given offset from init_row.
    /// Returns "" if the line does not exist.
    pub fn trimmed_line_by_offset(&self, offset: isize) -> &str
    {
        let row = self.init_row as isize + offset;
        self.src.lines().nth(row as usize)
            .unwrap_or("")
            .trim()
    }
}

/// Performs 'docwen check'.
/// Returns a Result containing a Vec of all documentation mismatches that were found.
pub fn check(toml_path: impl AsRef<Path>) -> anyhow::Result<Vec<String>>
{
    let mut mismatches: Vec<String> = Vec::new();

    // GET DOCFIG FROM TOML
    let docfig = Docfig::from_file(&toml_path)?;
    let abs_target_path = toml_manager::get_absolute_root(&toml_path, &docfig.settings.target)?;

    // GET ALL FUNCTION POSITIONS THAT NEED TO BE CHECKED
    let root = toml_manager::get_absolute_root(&toml_path, &docfig.settings.target)?;
    let mut position_maps: Vec<HashMap<FunctionID, Vec<FilePosition>>> = Vec::new();
    for file_group in docfig.file_groups
    {
        let abs_files = file_group.files.iter().map(|f| root.join(f)).collect::<Vec<_>>();
        position_maps.push(c_parse::find_function_positions(abs_files)?);
    }

    // CHECK FOR MATCHING DOCS
    for map in position_maps
    {
        for (_, vec) in map
        {
            // Get all sources
            let sources: Vec<LineSource> = vec.iter()
                .map(|f| fs::read_to_string(&f.path).map(|src| LineSource{src, init_row: f.row}))
                .collect::<Result<_, _>>()?;

            // Get lines at the current offset
            let mut offset = -1; // Begin at the line directly above the function
            let mut cur_lines: Vec<&str> = sources.iter()
                .map(|s| s.trimmed_line_by_offset(offset))
                .collect::<Vec<_>>();

            // Check each comment line individually
            while cur_lines.iter()
                .any(|s| s.starts_with("//") || s.starts_with("/*") || s.starts_with("*"))
            {
                let match_str = cur_lines.first().with_context(||"Failed to get 'match_str'")?;
                if cur_lines.iter().any(|f| f != match_str)
                {
                    mismatches.push(format_mismatch(match_str, &vec, &abs_target_path));
                    break;
                }
                offset -= 1;
                cur_lines = sources.iter()
                    .map(|s| s.trimmed_line_by_offset(offset))
                    .collect::<Vec<_>>();
            }
        }
    }

    Ok(mismatches)
}

/// Formats the given vec of file positions with a mismatch at 'match_str'.
/// Uses the given (absolute!) target_path to display the file positions as relative paths if possible.
pub fn format_mismatch(match_str: &str, vec: &Vec<FilePosition>, abs_target_path: impl AsRef<Path>)
    -> String
{
    let group_str = vec.iter()
        .map(|p| format!("{:?}:{}:{}",
                         p.path.strip_prefix(&abs_target_path).unwrap_or(&p.path),
                         p.row, p.column))
        .collect::<Vec<_>>().join(", ");
    format!("\"{}\"\n-> [{}]", match_str, group_str)
}