use std::path::PathBuf;

use clap::{Parser, Args, Subcommand};

#[derive(Parser, Debug)]
pub struct Cli {
    #[command(subcommand)]
    pub commands: Commands
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    New(NewData),
    Brew(BrewData),
    Pour,
    Add(AddData)
}
 
#[derive(Debug, Args)]
pub struct NewData { 
    #[arg(long, default_value_t = false)]
    pub lib: bool,
    #[arg(long, default_value_t = false)]
    pub bin: bool,

    pub name: String
}

#[derive(Debug, Args, Clone)]
pub struct BrewData {
    #[arg(long, default_value_t = false)]
    pub release: bool,
    #[arg(long, default_value_t = false)]
    pub debug: bool,
}

#[derive(Debug, Args)]
pub struct AddData {
    #[arg(long)]
    pub features: Option<String>,
    #[arg(long)]
    pub path: PathBuf,
    
    pub name: String
}

