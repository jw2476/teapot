#![feature(let_chains)]
#![feature(path_file_prefix)]

mod cli;
mod compiler;
mod config;

use std::{collections::HashMap, path::{PathBuf, Path}, io::{Write, BufReader, BufRead}, process::{Command, Stdio}};
use clap::{error::ErrorKind, CommandFactory, Parser};
use cli::{NewData, BrewData, Cli, Commands, AddData};
use colored::Colorize;
use compiler::{OutputType, Compiler};
use config::TeaConfig;
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

#[derive(Debug, Clone)]
struct Feature {
    name: String,
    enabled: bool
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
    path: PathBuf
}

impl Leaf {
    pub fn from_config(config: TeaConfig, enabled_features: Vec<String>, path: &Path) -> Self {
        let mut all_features = BASE_FEATURES.iter().map(ToString::to_string).collect::<Vec<String>>();
        all_features.append(&mut config.package.features.clone());

        let features = all_features.iter().map(|name| { 
            if enabled_features.contains(name) { 
                Feature { name: name.clone(), enabled: true }
            } else { 
                Feature { name: name.clone(), enabled: false } 
            }
        }).collect();

        let mut dependencies = config.dependencies.base.clone();
        for feature in &enabled_features {
            if let Some(deps) = config.dependencies.features.get(feature) {
                dependencies.append(&mut deps.clone());
            }
        }

        let dependencies = dependencies.iter().map(|dependency| {
            let dep_config = load_config(&path.join(dependency.path.as_ref().expect("Teapot only supports path based dependencies currently")));
            Self::from_config(dep_config, add_default_features(&dependency.features), &path.join(dependency.path.as_ref().unwrap())) 
        }).collect();

        Leaf {
            config,
            dependencies,
            features,
            path: path.to_owned()
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
        self.dependencies.iter().for_each(|dependency| dependency.compile(cmd.clone()));

        let sources: Vec<PathBuf> = WalkDir::new(self.path.join("src")).into_iter()
            .filter_map(|e| e.ok())
            .map(|entry| entry.path().to_owned())
            .filter(|path| {
                path.extension().is_some() && path.extension().unwrap().to_str().unwrap() == "c"
            })
            .filter(|path| {
                let feature_name = path.file_stem().unwrap().to_str().unwrap().split(".").last().unwrap();
                let feature = self.features.iter().find(|feature| feature.name == feature_name);
                feature.is_none() || feature.unwrap().enabled // Use the source if the file isn't
                                                              // feature specific or the source is
                                                              // enabled
            })
            .collect();
        
        let bin = sources.iter().find(|path| path.file_name().unwrap().to_str().unwrap() == "main.c").is_some();
        let mut compiler = Compiler::new(Path::new("target"));
        compiler.include(&self.path.join("include"));
        compiler.include(&self.path.join("src"));

        if cmd.release { compiler.set_optimization_level(3); }
        if cmd.debug { compiler.enable_debug_info() }
    
        self.features.iter().for_each(|feature| {
            if feature.enabled {
                compiler.define::<String>(&format!("FEATURE_{}", feature.name.to_uppercase()), None);
            }
        });

        self.dependencies.iter().for_each(|dependency| {
            compiler.include(&dependency.path.join("include"));
        });

        let mut defines = self.config.defines.base.clone();
        self.features.iter().filter(|feature| feature.enabled).for_each(|feature| {
            if let Some(defs) = self.config.defines.features.get(&feature.name) {
                defines.append(&mut defs.clone());      
            }
        });
        defines.iter().for_each(|(name, value)| {
            compiler.define(name, value.clone());
        });

        compiler.compile(&sources, &self.config.package.name);

        self.get_dependencies().iter().filter(|name| name != &&self.config.package.name).for_each(|dependency| {
            compiler.add_static_library(dependency);
        });
        
        let progress = format!("[{0}/{0}]", sources.len()).truecolor(0, 255, 0).bold();
        Self::clear();
        println!("\r{:13} {} {}", progress, "Linking".green().bold(), &self.config.package.name);
        compiler.link(&self.config.package.name, if bin { OutputType::Binary } else { OutputType::Library });
    }
}

fn brew(cmd: BrewData) {
    let config = load_config(Path::new(""));

    let leaf = Leaf::from_config(config, add_default_features(&[]), Path::new(""));
    leaf.compile(cmd);
}

fn pour() {
    let config = load_config(Path::new(""));
    let leaf = Leaf::from_config(config, add_default_features(&[]), Path::new(""));
    leaf.compile(BrewData {
        release: false,
        debug: false
    });
   
    let cmd = duct::cmd!(format!("target/{}", leaf.config.package.name));
    let reader = cmd.stderr_to_stdout().reader().unwrap();
    let lines = BufReader::new(reader).lines();
    lines.for_each(|line| println!("{}", line.unwrap()));    
}

fn add(cmd: AddData) {
     let config_string = String::from_utf8(std::fs::read("tea.toml").unwrap_or_else(|_| {
        println!("No tea.toml in this directory to add to");
        std::process::exit(1);
     })).unwrap();

     let mut config = config_string.parse::<Document>().unwrap();
     config["dependencies"][&cmd.name] = toml_edit::value(toml_edit::InlineTable::new());
     config["dependencies"][&cmd.name]["path"] = toml_edit::value(cmd.path.to_str().unwrap());
     config["dependencies"][&cmd.name]["features"] = toml_edit::value(cmd.features.unwrap_or_else(String::new).split(",").collect::<toml_edit::Array>());

     std::fs::write("tea.toml", config.to_string()).unwrap();
}

fn main() {
    let cli = Cli::parse();

    match cli.commands {
        Commands::New(data) => new(data),
        Commands::Brew(data) => brew(data),
        Commands::Pour => pour(),
        Commands::Add(data) => add(data)
    };
}
