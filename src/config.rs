use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use toml_edit::{Document, Item, Table, Value};

#[derive(Debug)]
pub struct TeaConfig {
    pub package: Package,
    pub dependencies: Dependencies,
    pub defines: Defines,
    pub libraries: Libraries
}

pub const BASE_FEATURES: &[&str] = &["windows", "linux"];

impl TeaConfig {
    pub fn parse(path: &Path) -> Option<Self> {
        let text = String::from_utf8(std::fs::read(path.join("tea.toml")).unwrap_or_else(|_| {
            println!("Can't find tea.toml in {}", path.display());
            std::process::exit(1);
        }))
        .unwrap();
        let document = text.parse::<Document>().ok()?;
        let package = Package::parse(document.get("package")?.as_table()?)?;

        let mut all_features = BASE_FEATURES
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<String>>();
        all_features.append(&mut package.features.clone());
        let dependencies =
            Dependencies::parse(document.get("dependencies")?.as_table()?, &all_features);
        let defines = document
            .get("defines")
            .map(|item| Defines::parse(item.as_table().unwrap(), &all_features))
            .unwrap_or_else(Defines::default);
        let libraries = document.get("libraries")
            .map(|item| Libraries::parse(item.as_table().unwrap(), &all_features))
            .unwrap_or_else(Libraries::default);

        Some(Self {
            package,
            dependencies,
            defines,
            libraries
        })
    }
}

#[derive(Debug)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub features: Vec<String>,
}

impl Package {
    pub fn parse(table: &Table) -> Option<Self> {
        Some(Self {
            name: table.get("name")?.as_str()?.to_owned(),
            version: table.get("version")?.as_str()?.to_owned(),
            features: table
                .get("features")
                .map(|item| item.as_array())
                .flatten()
                .map(|array| {
                    array
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(|str| str.to_owned())
                        .collect::<Vec<String>>()
                })
                .unwrap_or(Vec::new()),
        })
    }
}

#[derive(Debug)]
pub struct Dependencies {
    pub base: Vec<Dependency>,
    pub features: HashMap<String, Vec<Dependency>>,
}

impl Dependencies {
    pub fn parse(table: &Table, feature_names: &[String]) -> Self {
        let base = table
            .iter()
            .filter(|(name, _)| !feature_names.contains(&name.to_string()))
            .map(|(name, item)| Dependency::parse(name, item.as_value().unwrap()))
            .collect::<Vec<Dependency>>();

        let mut features = HashMap::new();
        feature_names
            .iter()
            .filter_map(|feature| table.iter().find(|(name, _)| name == feature))
            .for_each(|(name, value)| {
                let feature_table = value.as_table().unwrap();
                features.insert(
                    name.to_owned(),
                    feature_table
                        .iter()
                        .map(|(dep_name, item)| {
                            Dependency::parse(dep_name, item.as_value().unwrap())
                        })
                        .collect::<Vec<Dependency>>(),
                );
            });

        Self { base, features }
    }
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub path: Option<PathBuf>,
    pub features: Vec<String>,
}

impl Dependency {
    pub fn parse(name: &str, value: &Value) -> Self {
        match value {
            Value::InlineTable(table) => {
                let path: Option<PathBuf> = table
                    .get("path")
                    .map(|item| item.as_str())
                    .flatten()
                    .map(|str| Path::new(str).to_owned());
                let features: Vec<String> = table
                    .get("features")
                    .map(|item| item.as_array())
                    .flatten()
                    .map(|array| {
                        array
                            .iter()
                            .filter_map(|v| v.as_str())
                            .map(|str| str.to_owned())
                            .collect::<Vec<String>>()
                    })
                    .unwrap_or_else(Vec::new);
                Self {
                    name: name.to_owned(),
                    path,
                    features,
                }
            }
            _ => panic!("Teapot doesn't support non table based dependencies"),
        }
    }
}

#[derive(Debug, Default)]
pub struct Defines {
    pub base: Vec<(String, Option<String>)>,
    pub features: HashMap<String, Vec<(String, Option<String>)>>,
}

impl Defines {
    fn parse_define(name: &str, item: &Item) -> (String, Option<String>) {
        (
            name.to_owned(),
            match item.as_value().unwrap() {
                Value::String(data) => {
                    if data.value() == "" {
                        None
                    } else {
                        Some(data.to_string())
                    }
                }
                Value::Integer(data) => Some(data.to_string()),
                Value::Float(data) => Some(data.to_string()),
                Value::Boolean(data) => Some(data.to_string()),
                _ => panic!("Unsupported define type"),
            },
        )
    }

    pub fn parse(table: &Table, feature_names: &[String]) -> Self {
        let base = table
            .iter()
            .filter(|(name, _)| !feature_names.contains(&name.to_string()))
            .map(|(name, item)| Self::parse_define(name, item))
            .collect();

        let mut features = HashMap::new();
        feature_names
            .iter()
            .filter_map(|feature| table.iter().find(|(name, _)| name == feature))
            .for_each(|(name, item)| {
                features.insert(
                    name.to_owned(),
                    item.as_table()
                        .unwrap()
                        .iter()
                        .map(|(define, item)| Self::parse_define(define, item))
                        .collect(),
                );
            });

        Self { base, features }
    }
}

#[derive(Default, Debug)]
pub struct Libraries {
    pub base: Vec<String>,
    pub features: HashMap<String, Vec<String>>,
}

impl Libraries { 
    pub fn parse(table: &Table, feature_names: &[String]) -> Self {
        let base = table
            .iter()
            .filter(|(name, _)| !feature_names.contains(&name.to_string()))
            .map(|(name, _)| name.to_owned())
            .collect();

        let mut features = HashMap::new();
        feature_names
            .iter()
            .filter_map(|feature| table.iter().find(|(name, _)| name == feature))
            .for_each(|(name, item)| {
                features.insert(
                    name.to_owned(),
                    item.as_table()
                        .unwrap()
                        .iter()
                        .map(|(lib, _)| lib.to_owned())
                        .collect(),
                );
            });

        Self { base, features }
    }
}
