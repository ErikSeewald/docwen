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

/// Defines an ID for a function qualified name and params.
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct FunctionID
{
    pub qualified_name: String,
    pub params: String
}

/// Defines a structure used by the doc checker for indexing into the
/// src String by an offset to the init_row.
/// Generally, the docs are in [init_row-1, init_row-n] for docs of line length n.
struct LineSource
{
    pub src: String, // String containing the source file text
    pub init_row: usize, // The initial row in the src string (directly below docs)
}

impl LineSource
{
    /// Trims and returns the src line at the given offset from init_row.
    pub fn trimmed_line_by_offset(&self, offset: isize) -> anyhow::Result<&str>
    {
        let row = self.init_row as isize + offset;
        let trimmed = self.src.lines().nth(row as usize)
            .with_context(|| format!("Called nth({})", row))?
            .trim();
        Ok(trimmed)
    }
}

/// Performs 'docwen check'.
/// Returns a Result containing a Vec of all documentation mismatches that were found.
pub fn check(toml_path: impl AsRef<Path>) -> anyhow::Result<Vec<String>>
{
    let mut mismatches: Vec<String> = Vec::new();

    // GET DOCFIG FROM TOML
    let docfig = Docfig::from_file(&toml_path)?;

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
                .collect::<Result<_, _>>()?;

            // Check each comment line individually
            while cur_lines.iter()
                .any(|s| s.starts_with("//") || s.starts_with("/*") || s.starts_with("*"))
            {
                let match_str = cur_lines.first().with_context(||"Failed to get 'match_str'")?;
                if cur_lines.iter().any(|f| f != match_str)
                {
                    mismatches.push(format_mismatch(match_str, &vec, &toml_path));
                    break;
                }
                offset -= 1;
                cur_lines = sources.iter()
                    .map(|s| s.trimmed_line_by_offset(offset))
                    .collect::<Result<_, _>>()?;
            }
        }
    }

    Ok(mismatches)
}

/// Formats the given vec of file positions with a mismatch at 'match_str'.
/// Uses the given toml_path to display the file positions as relative paths if possible.
fn format_mismatch(match_str: &str, vec: &Vec<FilePosition>, toml_path: impl AsRef<Path>) -> String
{
    // If parent cannot be found, just use the path itself -> strip_prefix will fail and abs path
    // will be displayed.
    let parent = toml_path.as_ref().parent().unwrap_or(toml_path.as_ref());
    
    let group_str = vec.iter()
        .map(|p| format!("{:?}:{}:{}",
                         p.path.strip_prefix(parent).unwrap_or(&p.path),
                         p.row, p.column))
        .collect::<Vec<_>>().join(", ");
    format!("\"{}\"\n-> [{}]", match_str, group_str)
}