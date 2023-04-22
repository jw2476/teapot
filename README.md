# teapot 🫖
A cargo inspired C build system with built-in formatting, linting, dependencies, registry (coming soon), and multithreaded building

## Getting Started
To install teapot run:
`cargo install tpot`

### Dependencies

There are a few dependencies that teapot relies on. Most of these can be installed with a few commands:
 - TCC - https://github.com/TinyCC/tinycc
 - clang-format (Only needed for formatting)
 - clang-tidy (Only needed for linting)

### Creating your first Leaf
Once teapot and its dependencies are installed you are ready to brew! Create a new leaf (teapot's version of crates) with the command:

`tpot new --bin my_first_leaf`

If everything went well, there will now be a folder called my_first_leaf containing a tea.toml and a src/main.c file.

### Building and Running Leaves

Building to an executable or static library file in the `target` folder:

`tpot brew`

Or to compile and run with one command:

`tpot pour`

From there it's just more of the same. Teapot will find new C files as you create them, building and linking them at blazingly fast speeds thanks to TCC.

### Dependencies

Eventually you'll need to add dependencies to your code. Assuming the dependency supports teapot, it's as simple as downloading the leaf to your computer and running `tpot add`. For example, to add raylib:

`tpot add raylib --path deps/raylib --features text,shapes`

The local folder will be checked for a tea.toml file, built into a static library, and linked into your program.

### Formatting

To format your code, run:

`tpot format`

If you don't like teapot's default clang-format configuration adjust the .clang-format file located in the same directory as the tea.toml file.

### Linting

To lint your code, run:

`tpot lint`

This currently relies on clang-tidy and just uses the default lints, but a system for specifying lints in a similar way to clippy is in development.

### Notes on Libraries

To create a library leaf, use the `--lib` flag on the `tpot new` command. Library leaves have an include directory in addition to the src directory. Any header files within the include directory will be made available to external use.

### Features

Features are teapot's way of configuring what's compiled based on operating system, desktop environments, etc.

The available features for a leaf are specified in the tea.toml file at `package.features`. In addition to the user-defined features, teapot comes with predefined features such as windows and linux which are defined based on the target operating system.

If a feature is enabled the macro `FEATURE_[NAME]` will be defined. For example, if the target OS is Windows the `FEATURE_WINDOWS` macro will be defined. In addition to macros, and files ending in `.[NAME].c` will only be compiled if the feature is enabled. For example, if the target OS is Linux the file `awesome.windows.c` will be ignored and `awesome.linux.c` compiled.

## Changelog

### v0.1.2
 - Added linting
 - Added formatting
 
### v0.1.1
 - Added MT building
 - Added features
 - Added defines
 - Added dependencies
 
### v0.1.0
 - Simple building
