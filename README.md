# docwen
Docwen is a tool for automatically checking if docs match between C/C++ header and source files.

By setting up a workflow in an automatically managed docwen.toml file, docwen can scan all the specified files and look for documentation inconsistencies.
File pairs can be automatically generated (e.g., .h and .c files with matching names) or manually specified.

## Setup
### Installation
At the root of the repository:
```
cargo install --path .
```

### docwen.toml setup
Docwen needs a *docwen.toml* file. With
```
docwen create [<path>]
```
you can generate a default *docwen.toml* file and modify it. Once you have specified the [settings](#settings), you can run
```
docwen update [<docwen.toml path>]
```
to update the tracked files based on these settings (toml path can be omitted if it is in the cwd).

## Commands
- Note: Whenever a path is optional in one of the following commands, omitting it defaults to the cwd 
  (e.g. ```docwen check``` will work if *docwen.toml* is in the cwd).
  
| Command | Description
|---------|-------------
| ```docwen create [<path>]``` | Creates a default docwen.toml file at the specified path
| ```docwen update [<docwen.toml path>]``` | Updates the list of files tracked by the specified docwen.toml (only adds new filegroups to be tracked, does not untrack old ones)
| ```docwen check [<docwen.toml path>]``` | Runs the docwen check and outputs mismatches between docs if any are found

## Settings
The *docwen.toml* file is split into two parts: the settings and a list of tracked files.
Both can be modified by the user, but the tracked files are also intended to be managed automatically.

Example:
```
# The settings used by docwen
[settings]
target = "target_dir"  # This directory will be checked
match_extensions = ["h", "c", "hpp", "cc", "cpp"]  # Files of any of these extensions will be paired together if their names match
mode = "MATCH_FUNCTION_DOCS"  # Or MATCH_FUNCTION_DOCS_UNQUALIFIED
manual = ["ignore_this_1", "ignore_this_2"] # List of file names that 'update' will ignore -> can be managed manually

# The file pairs that are currently being tracked by docwen
[[filegroup]]
name = "example_file_1"
files = ["example_file_1.h", "example_file_1.c"]

[[filegroup]]
name = "example_file_2"
files = ["example_file_2.h", "example_file_2.cpp"]
```

## Modes
#### MATCH_FUNCTION_DOCS
The docs of functions will be checked for matches. Within a filegroup, **only** functions with matching **names**, **params**, and **qualifiers** will be matched.

#### MATCH_FUNCTION_DOCS_UNQUALIFIED
The docs of functions will be checked for matches. Within a filegroup, functions with matching **names** and **params** will be matched even if they have different qualifiers (e.g. belong to a different class).

## Manual filegroups
If function docs in files with different names need to be checked, the user will have to specify the filegroup 
themselves and add their names to the "manual" list. Otherwise ```docwen update``` would overwrite the group.

Example:
```
[settings]
target = "target_dir"
match_extensions = ["h", "c"]
mode = "MATCH_FUNCTION_DOCS"
manual = ["example_file", "alt_example_file"] # Both names need to be added here

# This group is manually managed and won't be touched by 'update'
[[filegroup]]
name = "example_file"
files = ["example_file.h", "example_file.c", "alt_example_file.c"]
```

## TODO
- Multi-line macros currently break the docwen parser
