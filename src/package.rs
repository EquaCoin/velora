//! Package Manager per Velora

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use colored::Colorize;

/// Manifest di un pacchetto Velora
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageManifest {
    pub package: PackageInfo,
    #[serde(default)]
    pub dependencies: HashMap<String, Dependency>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub main: Option<String>, // entry point
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Dependency {
    Simple(String), // "1.0.0" o "git:url"
    Detailed {
        version: Option<String>,
        git: Option<String>,
        path: Option<String>,
        branch: Option<String>,
    },
}

impl PackageManifest {
    /// Legge il manifest da un file velora.toml
    pub fn from_file(path: &Path) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
        
        let manifest: PackageManifest = toml::from_str(&content)
            .map_err(|e| format!("Failed to parse velora.toml: {}", e))?;
        
        Ok(manifest)
    }
    
    /// Crea un nuovo manifest di default
    pub fn new(name: &str, version: &str) -> Self {
        PackageManifest {
            package: PackageInfo {
                name: name.to_string(),
                version: version.to_string(),
                description: Some(format!("{} package", name)),
                author: None,
                license: Some("MIT".to_string()),
                main: Some("main.vel".to_string()),
            },
            dependencies: HashMap::new(),
        }
    }
    
    /// Salva il manifest su file
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize manifest: {}", e))?;
        
        fs::write(path, content)
            .map_err(|e| format!("Cannot write {}: {}", path.display(), e))?;
        
        Ok(())
    }
}

/// Gestisce l'installazione delle dipendenze
pub struct PackageManager {
    project_root: PathBuf,
    deps_dir: PathBuf,
}

impl PackageManager {
    pub fn new(project_root: PathBuf) -> Self {
        let deps_dir = project_root.join(".velora").join("deps");
        PackageManager { project_root, deps_dir }
    }
    
    /// Inizializza un nuovo progetto
    pub fn init(&self, name: &str) -> Result<(), String> {
        // Crea directory
        fs::create_dir_all(&self.project_root)
            .map_err(|e| format!("Cannot create project directory: {}", e))?;
        
        // Crea velora.toml
        let manifest = PackageManifest::new(name, "0.1.0");
        let toml_path = self.project_root.join("velora.toml");
        manifest.save(&toml_path)?;
        
        // Crea main.vel di esempio
        let main_path = self.project_root.join("main.vel");
        let main_content = format!(r#"# {}
# Version: 0.1.0

fn hello() -> String {{
    return "Hello from {}!"
}}

main:
    print(hello())
"#, name, name);
        
        fs::write(&main_path, main_content)
            .map_err(|e| format!("Cannot write main.vel: {}", e))?;
        
        println!("{} Created project '{}'", "✅".green(), name);
        println!("   {}", self.project_root.display());
        println!("   - velora.toml");
        println!("   - main.vel");
        
        Ok(())
    }
    
    /// Installa le dipendenze dal velora.toml
    pub fn install(&self) -> Result<(), String> {
        let manifest_path = self.project_root.join("velora.toml");
        if !manifest_path.exists() {
            return Err("No velora.toml found. Run 'velora init' first.".to_string());
        }
        
        let manifest = PackageManifest::from_file(&manifest_path)?;
        
        if manifest.dependencies.is_empty() {
            println!("{} No dependencies to install", "ℹ️".blue());
            return Ok(());
        }
        
        // Crea directory deps
        fs::create_dir_all(&self.deps_dir)
            .map_err(|e| format!("Cannot create deps directory: {}", e))?;
        
        println!("📦 Installing dependencies for '{}'...", manifest.package.name);
        
        for (name, dep) in &manifest.dependencies {
            self.install_dependency(name, dep)?;
        }
        
        println!("{} All dependencies installed", "✅".green());
        Ok(())
    }
    
    fn install_dependency(&self, name: &str, dep: &Dependency) -> Result<(), String> {
        let dep_dir = self.deps_dir.join(name);
        
        match dep {
            Dependency::Simple(spec) => {
                if spec.starts_with("git:") {
                    // Installa da git
                    let url = &spec[4..];
                    self.install_from_git(name, url, &dep_dir)?;
                } else if spec.starts_with("path:") {
                    // Installa da path locale
                    let path = &spec[5..];
                    self.install_from_path(name, path, &dep_dir)?;
                } else {
                    // Versione - per ora non supportata
                    return Err(format!("Version-based dependencies not yet supported: {}", spec));
                }
            }
            Dependency::Detailed { git, path, .. } => {
                if let Some(url) = git {
                    self.install_from_git(name, url, &dep_dir)?;
                } else if let Some(p) = path {
                    self.install_from_path(name, p, &dep_dir)?;
                } else {
                    return Err(format!("No source specified for dependency {}", name));
                }
            }
        }
        
        Ok(())
    }
    
    fn install_from_git(&self, name: &str, url: &str, dest: &Path) -> Result<(), String> {
        println!("  📥 {} from git: {}", name.cyan(), url);
        
        if dest.exists() {
            println!("     Already installed, skipping");
            return Ok(());
        }
        
        // Per ora usiamo git clone
        let status = std::process::Command::new("git")
            .args(["clone", "--depth", "1", url, &dest.to_string_lossy()])
            .status()
            .map_err(|e| format!("Failed to clone {}: {}", url, e))?;
        
        if !status.success() {
            return Err(format!("Git clone failed for {}", url));
        }
        
        println!("     {} Installed", "✓".green());
        Ok(())
    }
    
    fn install_from_path(&self, name: &str, source: &str, dest: &Path) -> Result<(), String> {
        println!("  📥 {} from path: {}", name.cyan(), source);
        
        if dest.exists() {
            fs::remove_dir_all(dest)
                .map_err(|e| format!("Cannot remove old {}: {}", name, e))?;
        }
        
        let source_path = self.project_root.join(source);
        
        // Copia ricorsiva
        Self::copy_dir(&source_path, dest)
            .map_err(|e| format!("Cannot copy {}: {}", name, e))?;
        
        println!("     {} Installed", "✓".green());
        Ok(())
    }
    
    fn copy_dir(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
        fs::create_dir_all(dst)?;
        
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = path.file_name().unwrap();
            let dest_path = dst.join(file_name);
            
            if path.is_dir() {
                Self::copy_dir(&path, &dest_path)?;
            } else {
                fs::copy(&path, &dest_path)?;
            }
        }
        
        Ok(())
    }
    
    /// Aggiunge una dipendenza al velora.toml
    pub fn add_dependency(&self, name: &str, source: &str) -> Result<(), String> {
        let manifest_path = self.project_root.join("velora.toml");
        let mut manifest = PackageManifest::from_file(&manifest_path)?;
        
        manifest.dependencies.insert(
            name.to_string(),
            Dependency::Simple(source.to_string())
        );
        
        manifest.save(&manifest_path)?;
        
        println!("{} Added dependency: {} = {}", "✅".green(), name, source);
        
        // Installa subito
        self.install_dependency(name, &Dependency::Simple(source.to_string()))?;
        
        Ok(())
    }
    
    /// Lista le dipendenze installate
    pub fn list_installed(&self) -> Result<Vec<String>, String> {
        let mut installed = Vec::new();
        
        if self.deps_dir.exists() {
            for entry in fs::read_dir(&self.deps_dir)
                .map_err(|e| format!("Cannot read deps directory: {}", e))? {
                let entry = entry.map_err(|e| format!("Entry error: {}", e))?;
                if entry.path().is_dir() {
                    installed.push(entry.file_name().to_string_lossy().to_string());
                }
            }
        }
        
        Ok(installed)
    }
}

/// Genera un lock file con le versioni esatte
pub fn generate_lockfile(project_root: &Path) -> Result<(), String> {
    let manifest_path = project_root.join("velora.toml");
    let manifest = PackageManifest::from_file(&manifest_path)?;
    
    let mut lock = HashMap::new();
    lock.insert("version".to_string(), "1".to_string());
    
    let mut packages = Vec::new();
    
    // Aggiungi il pacchetto corrente
    packages.push(HashMap::from([
        ("name".to_string(), manifest.package.name.clone()),
        ("version".to_string(), manifest.package.version.clone()),
    ]));
    
    // Aggiungi dipendenze
    for (name, dep) in &manifest.dependencies {
        let source = match dep {
            Dependency::Simple(s) => s.clone(),
            Dependency::Detailed { git, path, version, .. } => {
                git.clone().or(path.clone()).or(version.clone())
                    .unwrap_or_else(|| "unknown".to_string())
            }
        };
        
        let mut pkg = HashMap::new();
        pkg.insert("name".to_string(), name.clone());
        pkg.insert("source".to_string(), source);
        packages.push(pkg);
    }
    
    lock.insert("packages".to_string(), format!("{:?}", packages));
    
    let lock_path = project_root.join("velora.lock");
    let content = toml::to_string_pretty(&lock)
        .map_err(|e| format!("Cannot serialize lockfile: {}", e))?;
    
    fs::write(&lock_path, content)
        .map_err(|e| format!("Cannot write lockfile: {}", e))?;
    
    Ok(())
}
