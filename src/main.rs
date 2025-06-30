use std::path::{PathBuf};
use clap::{Parser, Subcommand};
use docwen::{docwen_check, toml_manager};

/// 'docwen' - A tool for automatically checking if docs match between C/C++ header and source files
#[derive(Parser)]
#[command(
    version,
    author = "Erik Seewald",
    about = "Scans file pairs and reports documentation mismatches",
    propagate_version = true
)]
struct CLI
{
    #[command(subcommand)]
    command: Command,
}

/// All commands for *docwen*. More information about the commands
/// can be found in *README.md*.
#[derive(Subcommand)]
enum Command
{
    /// create [<path>] - Creates a default docwen.toml file at the specified path
    Create
    {
        path: Option<PathBuf>
    },

    /// update [<docwen.toml path>] - Updates the list of files tracked by the specified docwen.toml
    Update
    {
      path: Option<PathBuf>
    },

    /// check [<docwen.toml path>] - Runs the docwen check and outputs mismatches between docs
    /// if any are found
    Check
    {
        path: Option<PathBuf>
    }
}

fn main() -> anyhow::Result<()>
{
    let cli = CLI::parse();

    match cli.command
    {
        Command::Create { path } =>
            {
                let path = path_or_default_toml(path);
                toml_manager::create_default(&path)?;
                println!("Created default docwen.toml at {:?}", path);
            }
        Command::Update { path } =>
            {
                let path = path_or_default_toml(path);
                toml_manager::update_toml(&path)?;
                println!("Updated {:?} successfully", path);
            }
        Command::Check { path } =>
            {
                let path = path_or_default_toml(path);
                let mismatches: Vec<String> = docwen_check::check(path)?;
                match mismatches.len()
                {
                    0 => println!("Found no mismatches!"),
                    _ =>
                        {
                            for m in &mismatches
                            {
                                println!("MISMATCH: {}\n", m);
                            }
                        }
                }
            }
    }

    Ok(())
}

/// Unwraps the given path option or defaults to the default *docwen.toml* path.
fn path_or_default_toml(path: Option<PathBuf>) -> PathBuf
{
    path.unwrap_or_else(|| PathBuf::from("./docwen.toml"))
}