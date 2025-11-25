// Polyglot Runtime Adapter
// Automatic language detection and WebAssembly interpreter injection

use std::path::{Path, PathBuf};
use anyhow::{Result, Context};
use tracing::{info, debug};

/// Detected programming language
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectedLanguage {
    PHP,
    Python,
    NodeJS,
    Ruby,
    StaticFiles,
    Unknown,
}

/// Runtime adapter
pub struct PolyglotAdapter {
    /// Path to runtime WebAssembly modules
    runtime_dir: PathBuf,
}

impl PolyglotAdapter {
    /// Create a new polyglot adapter
    pub fn new<P: AsRef<Path>>(runtime_dir: P) -> Self {
        let runtime_dir = runtime_dir.as_ref().to_path_buf();
        
        info!(runtime_dir = %runtime_dir.display(), "Polyglot adapter initialized");
        
        Self { runtime_dir }
    }

    /// Detect language from site directory
    pub fn detect_language<P: AsRef<Path>>(&self, site_path: P) -> Result<DetectedLanguage> {
        let site_path = site_path.as_ref();
        
        // Check for PHP
        if self.has_file(site_path, "index.php")
            || self.has_file(site_path, "composer.json")
            || self.has_extension(site_path, "php") {
            debug!("Detected PHP application");
            return Ok(DetectedLanguage::PHP);
        }

        // Check for Python
        if self.has_file(site_path, "requirements.txt")
            || self.has_file(site_path, "setup.py")
            || self.has_file(site_path, "pyproject.toml")
            || self.has_file(site_path, "app.py")
            || self.has_file(site_path, "main.py") {
            debug!("Detected Python application");
            return Ok(DetectedLanguage::Python);
        }

        // Check for Node.js  
        if self.has_file(site_path, "package.json")
            || self.has_file(site_path, "server.js")
            || self.has_file(site_path, "index.js") {
            debug!("Detected Node.js application");
            return Ok(DetectedLanguage::NodeJS);
        }

        // Check for Ruby
        if self.has_file(site_path, "Gemfile")
            || self.has_file(site_path, "config.ru") {
            debug!("Detected Ruby application");
            return Ok(DetectedLanguage::Ruby);
        }

        // Check for static files
        if self.has_file(site_path, "index.html")
            || self.has_extension(site_path, "html") {
            debug!("Detected static HTML site");
            return Ok(DetectedLanguage::StaticFiles);
        }

        debug!("Could not detect language");
        Ok(DetectedLanguage::Unknown)
    }

    /// Get runtime WebAssembly module path for language
    pub fn get_runtime_wasm(&self, language: &DetectedLanguage) -> Result<PathBuf> {
        let wasm_name = match language {
            DetectedLanguage::PHP => "php-cgi.wasm",
            DetectedLanguage::Python => "python3.11-wasi.wasm",
            DetectedLanguage::NodeJS => "node-wasi.wasm",
            DetectedLanguage::Ruby => "ruby-wasi.wasm",
            DetectedLanguage::StaticFiles => "static-server.wasm",
            DetectedLanguage::Unknown => {
                anyhow::bail!("Cannot get runtime for unknown language")
            }
        };

        let wasm_path = self.runtime_dir.join(wasm_name);
        
        if !wasm_path.exists() {
            anyhow::bail!(
                "Runtime WebAssembly module not found: {}. Please download or compile it.",
                wasm_path.display()
            );
        }

        Ok(wasm_path)
    }

    /// Get runtime configuration
    pub fn get_runtime_config(&self, language: &DetectedLanguage) -> RuntimeConfig {
        match language {
            DetectedLanguage::PHP => RuntimeConfig {
                entry_point: "index.php".to_string(),
                env_vars: vec![
                    ("SCRIPT_FILENAME".to_string(), "/var/www/index.php".to_string()),
                    ("REDIRECT_STATUS".to_string(), "200".to_string()),
                ],
                memory_limit_mb: 256,
            },
            DetectedLanguage::Python => RuntimeConfig {
                entry_point: "app.py".to_string(),
                env_vars: vec![
                    ("PYTHONPATH".to_string(), "/var/www".to_string()),
                ],
                memory_limit_mb: 512,
            },
            DetectedLanguage::NodeJS => RuntimeConfig {
                entry_point: "index.js".to_string(),
                env_vars: vec![
                    ("NODE_PATH".to_string(), "/var/www/node_modules".to_string()),
                ],
                memory_limit_mb: 512,
            },
            DetectedLanguage::Ruby => RuntimeConfig {
                entry_point: "config.ru".to_string(),
                env_vars: vec![],
                memory_limit_mb: 256,
            },
            DetectedLanguage::StaticFiles => RuntimeConfig {
                entry_point: "index.html".to_string(),
                env_vars: vec![],
                memory_limit_mb: 64,
            },
            DetectedLanguage::Unknown => RuntimeConfig {
                entry_point: "index.html".to_string(),
                env_vars: vec![],
                memory_limit_mb: 64,
            },
        }
    }

    /// Check if file exists in directory
    fn has_file<P: AsRef<Path>>(&self, dir: P, filename: &str) -> bool {
        dir.as_ref().join(filename).exists()
    }

    /// Check if directory has files with extension
    fn has_extension<P: AsRef<Path>>(&self, dir: P, ext: &str) -> bool {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Some(path_ext) = entry.path().extension() {
                    if path_ext == ext {
                        return true;
                    }
                }
            }
        }
        false
    }
}

impl Default for PolyglotAdapter {
    fn default() -> Self {
        Self::new("./assets/runtimes")
    }
}

/// Runtime configuration
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    pub entry_point: String,
    pub env_vars: Vec<(String, String)>,
    pub memory_limit_mb: usize,
}

/// Deployment advice for user
pub fn get_deployment_advice(language: &DetectedLanguage) -> String {
    match language {
        DetectedLanguage::PHP => {
            "PHP detected. Ensure composer dependencies are installed. \
             The php-cgi.wasm runtime will handle PHP execution."
        }
        DetectedLanguage::Python => {
            "Python detected. Install dependencies listed in requirements.txt. \
             The Python 3.11 WASI runtime will execute your application."
        }
        DetectedLanguage::NodeJS => {
            "Node.js detected. Run 'npm install' to install dependencies. \
             The Node.js WASI runtime will execute your application."
        }
        DetectedLanguage::Ruby => {
            "Ruby detected. Run 'bundle install' for dependencies. \
             The Ruby WASI runtime will execute via Rack."
        }
        DetectedLanguage::StaticFiles => {
            "Static HTML detected. A lightweight file server will serve your files."
        }
        DetectedLanguage::Unknown => {
            "Language could not be detected. Please specify runtime manually or \
             ensure your project has standard files (index.php, package.json, etc.)"
        }
    }.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_php_detection() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("index.php"), "<?php echo 'hello'; ?>").unwrap();
        
        let adapter = PolyglotAdapter::new("/tmp/runtimes");
        let lang = adapter.detect_language(temp.path()).unwrap();
        
        assert_eq!(lang, DetectedLanguage::PHP);
    }

    #[test]
    fn test_python_detection() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("requirements.txt"), "flask==2.0.0").unwrap();
        
        let adapter = PolyglotAdapter::new("/tmp/runtimes");
        let lang = adapter.detect_language(temp.path()).unwrap();
        
        assert_eq!(lang, DetectedLanguage::Python);
    }

    #[test]
    fn test_nodejs_detection() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("package.json"), r#"{"name": "app"}"#).unwrap();
        
        let adapter = PolyglotAdapter::new("/tmp/runtimes");
        let lang = adapter.detect_language(temp.path()).unwrap();
        
        assert_eq!(lang, DetectedLanguage::NodeJS);
    }

    #[test]
    fn test_static_detection() {
        let temp = TempDir::new().unwrap();
        fs::write(temp.path().join("index.html"), "<html></html>").unwrap();
        
        let adapter = PolyglotAdapter::new("/tmp/runtimes");
        let lang = adapter.detect_language(temp.path()).unwrap();
        
        assert_eq!(lang, DetectedLanguage::StaticFiles);
    }

    #[test]
    fn test_runtime_config() {
        let adapter = PolyglotAdapter::new("/tmp/runtimes");
        let config = adapter.get_runtime_config(&DetectedLanguage::PHP);
        
        assert_eq!(config.entry_point, "index.php");
        assert_eq!(config.memory_limit_mb, 256);
    }
}
