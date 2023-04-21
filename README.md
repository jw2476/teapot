# teapot
A cargo inspired C build system with in-build formatting, linting, dependencies, registry (coming soon) and multithreaded building

## Getting Started
To install teapot run:
`cargo install teapot`

### Dependencies

There are a few dependencies that teapot relies on, most of these can be installed with a few commands:
 - TCC - https://github.com/TinyCC/tinycc
 - clang-format (Only needed for formatting)
 - clang-tidy (Only needed for linting)

### Creating your first Leaf
Once teapot and its dependencies are installed you are ready to brew! Create a new leaf (teapot's version of crates) with the command:

`tpot new --bin my_first_leaf`

If everything went well, there will now be a folder called my_first_leaf containing a tea.toml and a src/main.c file.

### Building and Running Leaves

To build run, an executable or static library file will be placed in the target folder:

`tpot brew`

Or to just compile and run all in one step use:

`tpot pour`

From there it's just more of the same, teapot will find new C files as you create them, building and linking them at blazingly fast speeds thanks to TCC.

### Dependencies

Eventually you'll need to add dependencies to your code and assuming the project already works with teapot, 
its as simple as downloading the leaf to your computer and running:

`tpot add raylib --path deps/raylib --features text,shapes`

The local folder will be checked for a tea.toml file, built into a static library and linked into your program.

### Formatting

To format your code, run:

`tpot format`

If you don't like teapot's default clang-format configuration adjust the .clang-format file located in the same directory as the tea.toml file.

### Linting

To lint your code, run:

`tpot lint`

This currently relies on clang-tidy and just uses the default lints, but a system for specifying lints in a similar way to clippy is in development.

### Notes on Libraries

To create a library leaf, use the --lib flag on the tpot new command. Library leaves have an include directory in addition to the src directory, any header files within the include directory will be made available for external use.

### Features

Features are the teapot's way of configuring what's compiled based on things like operating systems, desktop environments and so on. 
The available features for a leaf are specified in the tea.toml file at `package.features`. In addition to the user-defined features, there are many predefined features such as windows and linux which get defined based on target operating system. 
If a feature is enabled the macro `FEATURE_[NAME]` will be defined e.g. if the target OS is windows the `FEATURE_WINDOWS` macro will be defined.
In addition to macros, any files ending in `.[NAME].c` will only be compiled if the feature is enabled e.g. if the target OS is linux the file `awesome.windows.c` will not be compiled but `awesome.linux.c` will.

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
