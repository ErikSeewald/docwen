//! Implements the doc match check functionality of docwen

use std::path::Path;
use crate::toml_parse::{Docfig};

pub fn check(toml_path: impl AsRef<Path>) -> anyhow::Result<Vec<String>>
{
    let _docfig = Docfig::from_file(toml_path)?;

    let mismatches: Vec<String> = Vec::new();
    Ok(mismatches)
}