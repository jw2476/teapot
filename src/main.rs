#![feature(let_chains)]
#![feature(path_file_prefix)]

mod cli;
mod compiler;
mod config;

use std::{collections::HashMap, path::{PathBuf, Path}, io::Write};
use clap::{error::ErrorKind, CommandFactory, Parser};
use cli::{NewData, BrewData, Cli, Commands};
use colored::Colorize;
use compiler::{OutputType, Compiler};
use config::TeaConfig;
use toml_edit::Document;
use walkdir::WalkDir;

fn new(cmd: NewData) {
    if cmd.lib == cmd.bin {
        panic!("Only --lib or --bin can be set, and one must be set");
    }

    let mut config = Document::new();
    config["package"] = toml_edit::table();
    config["package"]["name"] = toml_edit::value(&cmd.name);
    config["package"]["version"] = toml_edit::value("0.1.0");

    config["dependencies"] = toml_edit::table();

    std::fs::create_dir_all(format!("{}", &cmd.name)).unwrap();
    std::fs::write(format!("{}/tea.toml", &cmd.name), config.to_string()).unwrap();

    std::fs::create_dir_all(format!("{}/src", &cmd.name)).unwrap();
    
    if cmd.lib {
        std::fs::create_dir_all(format!("{}/include", &cmd.name)).unwrap();
        std::fs::write(format!("{0}/include/{0}.h", &cmd.name), "#pragma once").unwrap();
        std::fs::write(format!("{0}/src/{0}.c", &cmd.name), format!("#include \"{}.h\"", cmd.name)).unwrap();
    } else {
        std::fs::write(format!("{}/src/main.c", &cmd.name), "#include <stdio.h>\n\nint main() {\n\tprintf(\"Hello, World!\");\n\treturn 0;\n}").unwrap();
    }
}

fn load_config(path: &Path) -> TeaConfig {
    TeaConfig::parse(path).expect("Can't find/parse tea.toml")
}

fn brew(cmd: BrewData) {
    let config = load_config(Path::new(""));
    println!("{:#?}", config);
}

fn main() {
    let cli = Cli::parse();

    match cli.commands {
        Commands::New(data) => new(data),
        Commands::Brew(data) => brew(data),
    };
}
