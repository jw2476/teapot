#![feature(let_chains)]
#![feature(path_file_prefix)]

mod cli;
mod compiler;
mod config;

use clap::{error::ErrorKind, CommandFactory, Parser};
use cli::{AddData, BrewData, Cli, Commands, NewData};
use colored::Colorize;
use compiler::{Compiler, OutputType};
use config::TeaConfig;
use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use toml_edit::Document;
use walkdir::WalkDir;

use crate::config::BASE_FEATURES;

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
    std::fs::write(
        format!("{}/.clang-format", &cmd.name),
        include_bytes!("../assets/.clang-format"),
    )
    .unwrap();

    std::fs::create_dir_all(format!("{}/src", &cmd.name)).unwrap();

    if cmd.lib {
        std::fs::create_dir_all(format!("{}/include", &cmd.name)).unwrap();
        std::fs::write(format!("{0}/include/{0}.h", &cmd.name), "#pragma once").unwrap();
        std::fs::write(
            format!("{0}/src/{0}.c", &cmd.name),
            format!("#include \"{}.h\"", cmd.name),
        )
        .unwrap();
    } else {
        std::fs::write(
            format!("{}/src/main.c", &cmd.name),
            format!("#include <stdio.h>\n\nint {}_main() {{\n\tprintf(\"Hello, World!\");\n\treturn 0;\n}}", &cmd.name),
        )
        .unwrap();
    }
}

fn load_config(path: &Path) -> TeaConfig {
    TeaConfig::parse(path).expect("Can't find/parse tea.toml")
}

#[derive(Debug, Clone)]
struct Feature {
    name: String,
    enabled: bool,
}

fn add_default_features(features: &[String]) -> Vec<String> {
    let mut features = features.to_owned();
    features.append(&mut vec![std::env::consts::OS.to_owned()]);
    features
}

#[derive(Debug)]
struct Leaf {
    config: TeaConfig,
    dependencies: Vec<Leaf>,
    features: Vec<Feature>,
    path: PathBuf,
    defines: Vec<(String, Option<String>)>,
    libraries: Vec<String>
}

impl Leaf {
    pub fn from_config(config: TeaConfig, enabled_features: Vec<String>, path: &Path) -> Self {
        let mut all_features = BASE_FEATURES
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>();
        all_features.append(&mut config.package.features.clone());

        let features: Vec<Feature> = all_features
            .iter()
            .map(|name| {
                if enabled_features.contains(name) {
                    Feature {
                        name: name.clone(),
                        enabled: true,
                    }
                } else {
                    Feature {
                        name: name.clone(),
                        enabled: false,
                    }
                }
            })
            .collect();

        let mut dependencies = config.dependencies.base.clone();
        for feature in &enabled_features {
            if let Some(deps) = config.dependencies.features.get(feature) {
                dependencies.append(&mut deps.clone());
            }
        }

        let dependencies = dependencies
            .iter()
            .map(|dependency| {
                let dep_config = load_config(
                    &path.join(
                        dependency
                            .path
                            .as_ref()
                            .expect("Teapot only supports path based dependencies currently"),
                    ),
                );
                Self::from_config(
                    dep_config,
                    add_default_features(&dependency.features),
                    &path.join(dependency.path.as_ref().unwrap()),
                )
            })
            .collect();

        let mut defines = config.defines.base.clone();
        features
            .iter()
            .filter(|feature| feature.enabled)
            .for_each(|feature| {
                if let Some(defs) = config.defines.features.get(&feature.name) {
                    defines.append(&mut defs.clone());
                }
            });

        let mut libraries = config.libraries.base.clone();
        features.iter().filter(|feature| feature.enabled).for_each(|feature| {
            if let Some(libs) = config.libraries.features.get(&feature.name) {
                libraries.append(&mut libs.clone());
            }
        });

        Leaf {
            config,
            dependencies,
            features,
            path: path.to_owned(),
            defines,
            libraries
        }
    }

    pub fn get_dependencies(&self) -> Vec<String> {
        let mut output = vec![self.config.package.name.clone()];
        self.dependencies.iter().for_each(|dependency| {
            output.append(&mut dependency.get_dependencies());
        });

        output
    }

    fn clear() {
        print!("\r                                                      ");
    }

    pub fn compile(&self, cmd: BrewData) {
        self.dependencies
            .iter()
            .for_each(|dependency| dependency.compile(cmd.clone()));

        let sources: Vec<PathBuf> = WalkDir::new(self.path.join("src"))
            .into_iter()
            .filter_map(|e| e.ok())
            .map(|entry| entry.path().to_owned())
            .filter(|path| {
                path.extension().is_some() && path.extension().unwrap().to_str().unwrap() == "c"
            })
            .filter(|path| {
                let feature_name = path
                    .file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .split(".")
                    .last()
                    .unwrap();
                let feature = self
                    .features
                    .iter()
                    .find(|feature| feature.name == feature_name);
                feature.is_none() || feature.unwrap().enabled // Use the source if the file isn't
                                                              // feature specific or the source is
                                                              // enabled
            })
            .collect();

        let mut compiler = Compiler::new(Path::new("target"));
        compiler.include(&self.path.join("include"));
        compiler.include(&self.path.join("src"));

        if cmd.release {
            compiler.set_optimization_level(3);
        }
        if cmd.debug {
            compiler.enable_debug_info()
        }

        self.features.iter().for_each(|feature| {
            if feature.enabled {
                compiler
                    .define::<String>(&format!("FEATURE_{}", feature.name.to_uppercase()), None);
            }
        });

        self.dependencies.iter().for_each(|dependency| {
            compiler.include(&dependency.path.join("include"));
        });

        self.defines.iter().for_each(|(name, value)| {
            compiler.define(name, value.clone());
        });

        compiler.compile(&sources, &self.config.package.name);

        let progress = format!("[{0}/{0}]", sources.len())
            .truecolor(0, 255, 0)
            .bold();
        Self::clear();
        println!(
            "\r{:13} {} {}",
            progress,
            "Linking".green().bold(),
            &self.config.package.name
        );
        compiler.link(
            &self.config.package.name,
            OutputType::Library
        );
    }

    pub fn link(&self, cmd: BrewData) {
        let mut compiler = Compiler::new(Path::new("target"));
        if cmd.release {
            compiler.set_optimization_level(3);
        }
        if cmd.debug {
            compiler.enable_debug_info()
        }

        compiler.compile(&[Path::new("target/main.c").to_owned()], &self.config.package.name);

        let dependencies = self.get_dependencies();
        dependencies.iter().for_each(|dependency| {
            compiler.add_static_library(dependency);
        });

        self.libraries.iter().for_each(|library| {
            compiler.add_system_library(library);
        });
        
        Self::clear();
        println!(
            "\r{:13} {} {}",
            String::new(),
            "Finishing".green().bold(),
            &self.config.package.name
        );
        compiler.link(&self.config.package.name, OutputType::Binary);
    }
}

fn brew(cmd: BrewData) {
    let config = load_config(Path::new(""));

    let leaf = Leaf::from_config(config, add_default_features(&[]), Path::new(""));
    leaf.compile(cmd.clone());

    let main = format!("void {0}_main();\nint main() {{\n\t{0}_main();\n}}", leaf.config.package.name);
    std::fs::write("target/main.c", main).unwrap();

    leaf.link(cmd);
}

fn pour() {
    let config = load_config(Path::new(""));
    let leaf = Leaf::from_config(config, add_default_features(&[]), Path::new(""));

    let brew = BrewData { release: false, debug: false };
    leaf.compile(brew.clone());

    let main = format!("void {0}_main();\nint main() {{\n\t{0}_main();\n}}", leaf.config.package.name);
    std::fs::write("target/main.c", main).unwrap();

    leaf.link(brew);

    duct::cmd!(format!("target/{}", leaf.config.package.name))
        .run()
        .unwrap();
}

fn add(cmd: AddData) {
    let config_string = String::from_utf8(std::fs::read("tea.toml").unwrap_or_else(|_| {
        println!("No tea.toml in this directory to add to");
        std::process::exit(1);
    }))
    .unwrap();

    let mut config = config_string.parse::<Document>().unwrap();
    config["dependencies"][&cmd.name] = toml_edit::value(toml_edit::InlineTable::new());
    config["dependencies"][&cmd.name]["path"] = toml_edit::value(cmd.path.to_str().unwrap());
    config["dependencies"][&cmd.name]["features"] = toml_edit::value(
        cmd.features
            .unwrap_or_else(String::new)
            .split(",")
            .collect::<toml_edit::Array>(),
    );

    std::fs::write("tea.toml", config.to_string()).unwrap();
}

fn get_sources(path: &Path) -> Vec<PathBuf> {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .map(|e| e.path().to_owned())
        .filter(|path| {
            path.extension().is_some() && path.extension().unwrap().to_str().unwrap() == "c"
        })
        .collect()
}

fn fmt() {
    load_config(Path::new(""));
    let sources = get_sources(Path::new("src"));

    let mut args = vec!["-i"];
    args.append(&mut sources.iter().map(|path| path.to_str().unwrap()).collect());
    duct::cmd("clang-format", args).run().unwrap();

    println!("Formatted");
}

fn lint() {
    let config = load_config(Path::new(""));
    let leaf = Leaf::from_config(config, add_default_features(&[]), Path::new(""));
    let sources = get_sources(Path::new("src"));

    let mut args: Vec<String> = sources.iter().map(|path| path.to_str().unwrap().to_owned()).collect();
    args.push("--".to_owned());
    args.push("-Isrc".to_owned());
    args.push("-Iinclude".to_owned());
    leaf.dependencies.iter().for_each(|dependency| {
        args.push(format!("-I{}", dependency.path.join("include").display()));
    });
    leaf.features.iter().filter(|feature| feature.enabled).for_each(|feature| {
        args.push(format!("-DFEATURE_{}", feature.name.to_uppercase()));
    });
    leaf.defines.iter().for_each(|(name, value)| {
        if let Some(v) = value {
            args.push(format!("-D{}={}", name, v));
        } else {
            args.push(format!("-D{}", name));
        }
    });

    duct::cmd("clang-tidy", args).run().unwrap();
}

fn sip() {
    let config = load_config(Path::new(""));
    let leaf = Leaf::from_config(config, add_default_features(&[]), Path::new(""));
    let brew = BrewData { release: false, debug: false };
    leaf.compile(BrewData { release: false, debug: false });

    let symbols = duct::cmd!("nm", "-f", "just-symbols", format!("target/lib{}.a", leaf.config.package.name)).read().unwrap();
    let tests = symbols.lines().filter(|symbol| symbol.starts_with("test_")).collect::<Vec<&str>>();
    println!("Found tests: {:?}", tests);

    let forward = tests.iter().map(|test| format!("void {}();", test)).collect::<Vec<String>>().join("\n");
    let body = tests.iter().map(|test| format!("\tprintf(\"Testing {0}\\n\");\n\t{0}();", test)).collect::<Vec<String>>().join("\n");
    let test_runner = format!("#include <stdio.h>\n\n{}\n\nint main() {{\n{}\n}}", forward, body);
    std::fs::write("target/main.c", test_runner).unwrap();
    
    leaf.link(brew);
    duct::cmd!(format!("./target/{}", leaf.config.package.name)).run().unwrap();
}

fn main() {
    let cli = Cli::parse();

    match cli.commands {
        Commands::New(data) => new(data),
        Commands::Brew(data) => brew(data),
        Commands::Pour => pour(),
        Commands::Add(data) => add(data),
        Commands::Format => fmt(),
        Commands::Lint => lint(),
        Commands::Sip => sip()
    };
}
