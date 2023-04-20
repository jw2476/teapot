use std::{path::{PathBuf, Path}, process::Command};


pub enum OutputType {
    Binary,
    Library
}

pub struct Compiler {
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

    pub fn add_static_library(&mut self, name: &str) {
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

    pub fn compile(&mut self, path: &Path) {
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

    pub fn link(&self, name: &str, output: OutputType) {
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