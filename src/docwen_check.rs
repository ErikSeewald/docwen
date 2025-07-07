//! Implements the doc match check functionality of docwen

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
#[derive(PartialEq, Eq, Hash)]
pub struct FunctionID
{
    pub qualified_name: String,
    pub params: String
}

pub fn check(toml_path: impl AsRef<Path>) -> anyhow::Result<Vec<String>>
{
    let docfig = Docfig::from_file(toml_path)?;

    c_parse::find_function_positions(docfig.file_groups.get(1).unwrap().files.clone())?;

    let mismatches: Vec<String> = Vec::new();
    Ok(mismatches)
}