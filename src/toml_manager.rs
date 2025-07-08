//! Handles creating and updating *docwen.toml* files

use std::collections::{HashMap, HashSet};
use std::ffi::OsStr;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use anyhow::Context;
use walkdir::WalkDir;
use crate::docfig::{Docfig, FileGroup, Settings};

pub const DEFAULT_TOML: &str = r#"[settings]
target = "src"
match_extensions = ["h", "c", "hpp", "cc", "cpp"]
mode = "MATCH_FUNCTION_DOCS"
ignore = []
"#;

/// Implements the docwen *create* command.
/// Creates a default *docwen.toml* file at the given path.
/// Returns an error if the path is invalid or already exists.
pub fn create_default(path: impl AsRef<Path>) -> anyhow::Result<()>
{
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .with_context(||
            format!("Failed to create new docwen.toml at {:?}", path.as_ref().display()))?;

    file.write_all(DEFAULT_TOML.as_bytes()).with_context(||
        format!("Failed to write to docwen.toml at {:?}", path.as_ref().display()))?;
    Ok(())
}

/// Implements the docwen *update* command.
/// Parses the *docwen.toml* at the given path and updates it based on the
/// settings it specifies.
/// Returns an error if the file cannot be parsed or updated.
pub fn update_toml(path: impl AsRef<Path>) -> anyhow::Result<()>
{
    let mut docfig = Docfig::from_file(&path)?;

    // Get all file paths
    let root = get_absolute_root(&path, &docfig.settings.target)?;
    let paths: Vec<PathBuf> = WalkDir::new(&root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e|
            e.path()
                .strip_prefix(&root) // as relative paths
                .ok()
                .map(Path::to_path_buf)
        )
        .collect();

    let mut groups: Vec<FileGroup> = group_by_stem(paths, &docfig.settings);
    groups.retain(|g| g.files.len() > 1);

    // Merge (overwrite existing with new versions but do not delete non-existing)
    for g in groups
    {
        // Replace old group with new one (equals only considers name, so different file list
        // gets updated)
        if let Some(slot) = docfig.file_groups.iter_mut().find(|x| **x == g)
        {
            *slot = g;
        }

        else
        {
            docfig.file_groups.push(g);
        }
    }
    docfig.write_file(&path)?;

    Ok(())
}

/// Groups all files defined by the given paths by matching name (stem)
/// based on the given settings.
pub fn group_by_stem<I>(paths: I, settings: &Settings) -> Vec<FileGroup>
where
    I: IntoIterator<Item = PathBuf>,
{
    let match_extensions: HashSet<String> =
        settings.match_extensions.clone().into_iter().map(|e| e.to_ascii_lowercase()).collect();

    let mut groups: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for path in paths
    {
        // SKIP OTHER EXTENSIONS
        match path.extension().and_then(OsStr::to_str)
        {
            Some(e) if match_extensions.contains(&e.to_ascii_lowercase()) => {},
            _ => continue,
        };

        // GET STEM
        let stem = match path.file_stem().and_then(OsStr::to_str)
        {
            Some(s) => s.to_owned().to_ascii_lowercase(),
            None => continue,
        };

        // CHECK IGNORE AND ADD
        if !settings.ignore.contains(&stem)
        {
            groups.entry(stem).or_default().push(path);
        }
    }

    // CONVERT
    groups
        .into_iter()
        .map(|(name, files)| { FileGroup { name, files } })
        .collect()
}

/// Returns the absolute root target path defined by the given toml_path and the
/// (optionally relative to toml_path) target path.
pub fn get_absolute_root(toml_path: impl AsRef<Path>, target: impl AsRef<Path>)
    -> anyhow::Result<PathBuf>
{
    let path = if target.as_ref().is_absolute() {PathBuf::from(target.as_ref())} else {
        toml_path.as_ref().parent()
            .with_context(|| format!("Could not access parent of {:?}", toml_path.as_ref()))?
            .join(target.as_ref())
    };
    Ok(path)
}