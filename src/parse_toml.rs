//! Handles parsing *docwen.toml* into a suitable data structure

use std::{fs, path::{Path, PathBuf}};
use std::collections::HashSet;
use anyhow::Context;
use serde::{Serialize, Deserialize};


/// Represents the entire of *docwen.toml*
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Docfig
{
    pub settings: Settings,

    #[serde(rename = "filegroup", default)]
    pub file_groups: Vec<FileGroup>,
}

/// Represents the user-defined settings
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings
{
    pub target: PathBuf,

    #[serde(default)] // Default to false if key is missing
    pub match_case: bool,

    #[serde(default)]
    pub match_extensions: Vec<String>,

    pub mode: Mode,

    #[serde(default)]
    pub ignore: Vec<String>
}

/// Operational modes of docwen
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Mode
{
    MatchFunctionDocs
}

/// A single group of files that will be checked for matching docs
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileGroup
{
    pub name: String,
    pub files: Vec<PathBuf>
}

impl Docfig
{
    /// Reads and parses a *docwen.toml*
    pub fn from_file(path: impl AsRef<Path>) -> anyhow::Result<Self>
    {
        let raw = fs::read_to_string(&path).with_context(||
            format!("Failed to read {}", path.as_ref().display()))?;

        let docfig: Self = toml::from_str(&raw).with_context(||
            format!("Failed to parse {}", path.as_ref().display()))?;
        docfig.validate()?;
        Ok(docfig)
    }

    fn validate(&self) -> anyhow::Result<()>
    {
        // No duplicate filegroup names
        let mut seen = HashSet::new();
        for fg in &self.file_groups
        {
            if !seen.insert(&fg.name)
            {
                return Err(anyhow::anyhow!("Duplicate filegroup name: {}", fg.name));
            }
        }
        Ok(())
    }
}