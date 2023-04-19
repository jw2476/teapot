#![feature(let_chains)]
#![feature(path_file_prefix)]

use std::{collections::HashMap, path::{PathBuf, Path}, process::Command, io::Write};
use colored::Colorize;
use serde::{Serialize, Deserialize};
use clap::{Subcommand, Parser, Args, error::ErrorKind, CommandFactory};
use walkdir::WalkDir;

#[derive(Deserialize, Serialize, Clone)]
pub struct Dependency {
    path: Option<String>
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Package {
    name: String,
    version: String,
    authors: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Defines {
    #[serde(default)]
    all: Option<toml::Table>,
    #[serde(default)]
    windows: Option<toml::Table>,
    #[serde(default)]
    linux: Option<toml::Table>
}

#[derive(Deserialize, Serialize, Clone)]
pub struct TeaConfig {
    package: Package,
    dependencies: HashMap<String, toml::Value>,
    #[serde(default)]
    defines: Option<Defines>
}

#[derive(Parser, Debug)]
struct Cli {
    #[command(subcommand)]
    commands: Commands
}

#[derive(Debug, Args)]
struct NewData { 
    #[arg(long, default_value_t = false)]
    lib: bool,
    #[arg(long, default_value_t = false)]
    bin: bool,

    name: String
}

#[derive(Debug, Args)]
struct BrewData {
    #[arg(long, default_value_t = false)]
    release: bool,
    #[arg(long, default_value_t = false)]
    debug: bool,
    #[arg(long, default_value_t = false)]
    pour: bool
}

#[derive(Debug, Args)]
struct PourData {
    #[arg(long, default_value_t = false)]
    release: bool
}

#[derive(Subcommand, Debug)]
enum Commands {
    New(NewData),
    Brew(BrewData),
}

fn create_leaf(data: NewData) {
    if data.lib == data.bin {
        let mut cmd = Cli::command();
        cmd.error(ErrorKind::ArgumentConflict, "Either --lib or --bin must be set").exit();
    } else {
        std::fs::create_dir(&data.name).expect("Failed to create root directory");
        std::fs::create_dir(format!("{}/src", &data.name)).expect("Failed to create src directory");
        
        if data.lib {
            std::fs::write(format!("{0}/src/{0}.c", &data.name), "void test() {}").expect(&format!("Failed to create {}.c", &data.name));
            std::fs::write(format!("{0}/src/{0}.h", &data.name), "#pragma once\n\nvoid test();").expect(&format!("Failed to create {}.h", &data.name));
        }

        if data.bin {
            std::fs::write(format!("{0}/src/main.c", &data.name), "#include <stdio.h>\n\n int main() {\n\tprintf(\"Hello, World!\\n\");\n\treturn 0;\n}").expect("Failed to create main.c");
        }

        let config = TeaConfig {
            package: Package {
                name: data.name.clone(),
                version: "0.0.1".to_owned(),
                authors: Vec::new()
            },
            dependencies: HashMap::new(),
            defines: None
        };

        let config_string = toml::to_string(&config).expect("Failed to serialise config");
        std::fs::write(format!("{0}/tea.toml", &data.name), config_string).expect("Failed to write tea.toml");
    }
}

enum OutputType {
    Binary,
    Library
}

struct Compiler {
    target_directory: PathBuf,
    compile_flags: Vec<String>,
    link_flags: Vec<String>,
    defines: Vec<(String, Option<String>)>,

    objects: Vec<PathBuf>
}

impl Compiler {
    pub fn new(target_directory: &Path) -> Self {
        Self {
            target_directory: target_directory.to_owned(),
            compile_flags: Vec::new(),
            link_flags: vec!["-lm".to_owned()],
            objects: Vec::new(),
            defines: Vec::new()
        }
    }

    pub fn include(&mut self, path: &str) {
        self.compile_flags.push(format!("-I{}", path));
    }

    pub fn link(&mut self, name: &str) {
        self.objects.push(self.target_directory.join(format!("lib{}.a", name)));
    }

    pub fn define<T: ToString>(&mut self, name: &str, value: Option<T>) {
       self.defines.push((name.to_owned(), value.map(|s| s.to_string()))); 
    }

    pub fn set_optimization_level(&mut self, level: u32) {
        self.compile_flags.push(format!("-O{}", level));
    }

    pub fn enable_debug_info(&mut self) {
        self.compile_flags.push("-g".to_owned());
    }

    pub fn file(&mut self, path: &Path) {
        let obj = self.target_directory.clone().join("objects").join(path.with_extension("o"));
        std::fs::create_dir_all(obj.parent().unwrap()).unwrap();
        let mut cmd = Command::new("tcc");

        self.defines.iter().for_each(|(name, value)| {
            if let Some(v) = value {
                cmd.arg(&format!("-D{}={}", name, v));
            } else {
                cmd.arg(&format!("-D{}", name));
            }
        });

        cmd.args(&self.compile_flags)
            .arg("-c")
            .arg(path)
            .arg("-o")
            .arg(obj.clone());
        let output = cmd.output().expect("Failed to compile");

        if !output.status.success() {
            println!("{}", String::from_utf8(output.stdout).unwrap());
            println!("{}", String::from_utf8(output.stderr).unwrap());
            panic!("{} failed to compile", path.display());
        }

        self.objects.insert(0, obj);
    }

    pub fn compile(&self, name: &str, output: OutputType) {
         let file: String = match output {
            OutputType::Binary => name.to_owned(),
            OutputType::Library => format!("lib{}.a", name)
         };

         let artifact_path = self.target_directory.join(file);
        
         let output = match output {
             OutputType::Binary => Command::new("tcc")
                 .args(&self.link_flags)
                 .args(&self.objects)
                 .arg("-o")
                 .arg(artifact_path)
                 .output()
                 .expect("Failed to link"),
             OutputType::Library => Command::new("ar")
                 .arg("rcs")
                 .arg(artifact_path)
                 .args(&self.objects)
                 .output()
                 .expect("Failed to archive")
         };
        
         if !output.status.success() { 
            println!("{}", String::from_utf8(output.stdout).unwrap());
            println!("{}", String::from_utf8(output.stderr).unwrap());
            panic!("{} failed to link", name);
        }
    }
}

struct Leaf {
    config: TeaConfig,
    path: PathBuf,
    dependencies: Vec<Leaf>,
}

fn resolve_dependencies(config: TeaConfig, path: PathBuf) -> Leaf {
    let mut leaf = Leaf { config: config.clone(), path: path.clone(), dependencies: Vec::new() };

    config.dependencies.clone().iter().for_each(|(name, dependency)| {
        match dependency {
            toml::Value::Table(table) => {
                let dependency: Dependency = table.clone().try_into().unwrap();
                let dependency_path = path.join(dependency.path.unwrap());
                let dependency_config_string = std::fs::read(dependency_path.join("tea.toml")).expect(&format!("Can't find tea.toml for {}", name));
                let dependency_config: TeaConfig = toml::from_str(&String::from_utf8(dependency_config_string).unwrap()).unwrap();
                let dependency_leaf = resolve_dependencies(dependency_config, dependency_path);
                leaf.dependencies.push(dependency_leaf);
            },
            _ => {}
        }
    });

    leaf
}

fn link_dependencies(leaf: &Leaf, compiler: &mut Compiler) {
    leaf.dependencies.iter().for_each(|dep| {
        compiler.link(&dep.config.package.name);
        link_dependencies(dep, compiler);
    })
}

fn get_defines(leaf: &Leaf) -> Vec<(String, String)> {
    let base_defines = leaf.config.defines.clone().unwrap_or_else(Defines::default);

    let mut defines = base_defines.all.unwrap_or_else(toml::Table::new).into_iter().collect::<Vec<(String, toml::Value)>>();
    #[cfg(target_os = "windows")]
    defines.append(&mut base_defines.windows.unwrap_or_else(toml::Table::new).into_iter().collect::<Vec<(String, toml::Value)>>());
    #[cfg(target_os = "linux")]
    defines.append(&mut base_defines.linux.unwrap_or_else(toml::Table::new).into_iter().collect::<Vec<(String, toml::Value)>>());

    defines.iter().filter_map(|(name, value)| {
        Some((name.to_owned(), match value {
            toml::Value::Integer(data) => data.to_string(),
            toml::Value::Float(data) => data.to_string(),
            toml::Value::Boolean(data) => data.to_string(),
            toml::Value::String(data) => format!("\"{}\"", data),
            _ => return None
        }))
    }).collect()
}

fn get_sources(leaf: &Leaf) -> Vec<PathBuf> { 
    let os_list = vec!["windows", "linux"];

    WalkDir::new(leaf.path.join("src")).into_iter().filter_map(|e| e.ok()).filter_map(|entry| {
        if let Some(extension) = entry.path().extension() && extension == "c" {
            Some(entry.path().to_owned())
        } else {
            None
        }
    }).filter_map(|path| {
        let os_extension = path.file_stem()?.to_str()?.split(".").last()?;
        if !os_list.contains(&os_extension) || (os_list.contains(&os_extension) && os_extension == std::env::consts::OS) {
            Some(path)
        } else {
            None
        }
    }).collect::<Vec<PathBuf>>()
}

fn compile_leaf(leaf: &Leaf, target_path: &Path, cmd: &BrewData) {
    for dependency in &leaf.dependencies {
        compile_leaf(dependency, target_path, cmd);
    }
 
    let sources = get_sources(leaf);
    let bin = sources.iter().find(|source| source.file_prefix().unwrap() == "main").is_some();
    
    let mut compiler = Compiler::new(target_path);
    if cmd.release { 
        compiler.set_optimization_level(3);
    } else if cmd.debug {
        compiler.enable_debug_info();
    }

    compiler.include(leaf.path.join("include").to_str().unwrap());
    compiler.include(leaf.path.join("src").to_str().unwrap());

    for dependency in &leaf.dependencies {
        compiler.include(dependency.path.join("include").to_str().unwrap());
    }

    link_dependencies(leaf, &mut compiler); 
    
    #[cfg(target_os = "windows")]
    compiler.define::<String>("PLATFORM_WINDOWS", None);
    #[cfg(target_os = "linux")]
    compiler.define::<String>("PLATFORM_LINUX", None);
    
    get_defines(leaf).iter().for_each(|(name, value)| {
        if value == "\"\"" {
            compiler.define::<String>(name, None);
        } else {
            compiler.define(name, Some(value));
        }
    });

    for (i, source) in sources.iter().enumerate() {
        let red: f32 = 1.0 - (i as f32 / sources.len() as f32);
        let green: f32 = i as f32 / sources.len() as f32;
        let red: u8 = (red * 256.0) as u8;
        let green = (green * 256.0) as u8;

        let progress = format!("[{}/{}]", i, sources.len());

        print!("\r                                                   ");
        print!("\r{:13} {} {} v{}", progress.truecolor(red, green, 0), "Compiling:".green().bold(), leaf.config.package.name, leaf.config.package.version);
        std::io::stdout().flush().unwrap();
        compiler.file(source);
    }

    let progress = format!("[{0}/{0}]", sources.len());
    print!("\r                                                        ");
    print!("\r{:13} {} {} v{}", progress.truecolor(0, 255, 0), "Linking:".green().bold(), leaf.config.package.name, leaf.config.package.version);
    compiler.compile(&leaf.config.package.name, if bin { OutputType::Binary } else { OutputType::Library });
    println!("");

    if cmd.pour && bin {
        let pour = Command::new(format!("./{}/{}", target_path.display(), leaf.config.package.name)).output().expect("Failed to pour");
        println!("{}", String::from_utf8(pour.stdout).unwrap());
        println!("{}", String::from_utf8(pour.stderr).unwrap());
    }
}

fn brew(cmd: BrewData) {
    let config_string = std::fs::read("tea.toml").expect("Can't read tea.toml");
    let config: TeaConfig = toml::from_str(&String::from_utf8(config_string).unwrap()).unwrap();

    let leaf = resolve_dependencies(config, PathBuf::new());
    compile_leaf(&leaf, if cmd.release { Path::new("target/release") } else { Path::new("target/debug") }, &cmd);
}

fn main() {
    let cli = Cli::parse();

    match cli.commands {
        Commands::New(data) => create_leaf(data),
        Commands::Brew(data) => brew(data),
    };
}
