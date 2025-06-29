# docwen
Docwen is a tool for automatically checking if docs match between C/C++ header and source files.

By setting up a workflow in a (automatically managed) docwen.toml file, docwen can scan all the specified files and look for documentation inconsistencies.
File pairs can be automatically generated (e.g. .h and .c files with matching names), auto ignored, and manually specified.

## Setup
### Installation

### docwen.toml setup
Docwen needs a *docwen.toml* file to work with. With
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
| ```docwen update [<docwen.toml path>]``` | Updates the list of files tracked by the specified docwen.toml (only adds new files to be tracked, does not untrack)
| ```docwen check [<docwen.toml path>]``` | Runs the docwen check and outputs mismatches between docs if any are found

## Settings
The *docwen.toml* file is split into two parts: The settings and a list of tracked files.
Both can be modified by the user but the tracked files are also intended to be managed automatically.

Example:
```
# The settings used by docwen
[settings]
target = "target_dir"  # This directory will be checked
match_case = false     # Specifies whether file names have to match case exactly to be paired
match_extensions = ["h", "c", "hpp", "cc", "cpp"]  # Files of any of these extensions will be paired together if their names match
mode = MATCH_FUNCTION_DOCS  # Currently the only mode of operation. The docs of functions with matching names and arguments between file pairs will be checked.
ignore = ["ignore_this_1", "ignore_this_2"] # List of file names to ignore

# The file pairs that are currently being tracked by docwen
[[filegroup]]
name = "example_file_1"
files = ["example_file_1.h", "example_file_1.c"]

[[filegroup]]
name = "example_file_2"
files = ["example_file_2.h", "example_file_2.cpp", "alt_example_file_2.cpp"]
```
