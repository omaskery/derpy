# derpy

Derpy manages your derpendencies! ;)

I did a small amount of research for a dependency manager that had no
intrinsic internet dependencies, was not bound to a particular programming
language or specific version control system. I found no results that
met my requirements, so I wrote derpy!

# motivation

One of the development environments I interact with uses multiple version control systems
for different types of artefact and has sufficiently strict network security as to make
internet access essentially impossible.

Projects in that team often contain code copy & pasted between projects rather than shared
with libraries, largely because existing dependency management tools don't solve their problems.

So I thought I'd write a small tool to solve that specific problem and no more.

# high level design

The tool is version control system agnostic in that it doesn't have any built-in code for talking
to specific version control systems, it contains a simple data based system for "teaching" derpy how
to perform essential operations to interact with dependencies. See the `vcs_info` directory for examples.

The tool is language agnostic in that it is purely a mechanism for ensuring files from particular repositories
are retrieved and placed in the project directory at specified locations.

The tool has no intrinsic dependency on the internet due to having no central repository or similar,
though there is nothing stopping somebody from "teaching" derpy how to do.

# examples

## initialisation

In a project directory initialise derpy:

`derpy init`

This will create a derpy.json file that contains (an empty) list of dependencies.

## adding dependencies

To add a dependency you need at least three pieces of information:

- *version control system* - what version control system is the dependency contained in?
- *dependency name* - what will you call the dependency
  (*note:* that this will also be the name of the folder it is stored in within your project)
- *URL* - some string (typically a URL) that explains where to find the dependency

In this example, let's use _git_ as the *version control system*, _classdict_ as the *dependency name*
and http://github.com/omaskery/classdict.git as the *URL*:

`derpy add git classdict http://github.com/omaskery/classdict.git`

The derpy.json file will now contain information describing your new dependency, but nothing will be fetched yet.

- Want to track a specific version of a dependency? See the `--version` parameter
- Want to place the dependency somewhere other than `project_dir/deps/`? See the `--target` parameter
- Have specifal key:value pairs that your version control system needs per-dependency? See the `--option` parameter

## acquiring dependencies

To fetch dependencies for your project simply run:

`derpy acquire`

This will go through all dependencies and place them in your project directory, by default in
`<your project directory>/deps/<dependency name>` - though this can be overridden by passing
`--target <target directory>` to `derpy add`.

Notice that once this is done a new file will now exist in the project directory: derpy.lock.json.
This file is identical in structure to derpy.json but it will contain information that locks each
dependency to the exact version acquired. Typically this is a git commit hash or svn revision number, etc.

Once a derpy.lock.json is generated any subsequent `derpy acquire` invocation will automatically
fetch the specific version specified by the lock file, rather than the 'latest' that might otherwise
be retrieved.

## upgrading dependencies

If you wish to upgrade the version of a dependency, rather than using the version specified in your
lock file, use the upgrade command:

`derpy upgrade --all`

This will effectively perform an `acquire`, but will behave as though there is no lock file present.
To be more selective about which dependencies are upgraded simply specify the names of dependencies
to upgrade instead of the '--all' parameter. E.g. using the `classdict` example again:

`derpy upgrade classdict`

# to do

- [x] get the basics down - have something that could actually possibly solve the problem!
- [ ] make it recursive, fetch dependencies of dependencies
- [ ] document the code
- [ ] document the tool
