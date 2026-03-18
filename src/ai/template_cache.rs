/// Sistema cache per template generati dinamicamente
/// Salva i template GROK validati per riutilizzo futuro

use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Struttura per un template in cache
#[derive(Clone, Debug)]
pub struct CachedTemplate {
    pub name: String,
    pub description: String,
    pub pattern: String,
    pub code: String,
    pub use_count: u32,
}

/// Manager della cache
pub struct TemplateCache {
    templates: HashMap<String, CachedTemplate>,
    cache_file: String,
    modified: bool,
}

impl TemplateCache {
    /// Crea nuova cache, carica da file se esiste
    pub fn new(cache_path: &str) -> Self {
        let mut cache = TemplateCache {
            templates: HashMap::new(),
            cache_file: cache_path.to_string(),
            modified: false,
        };
        
        // Carica cache esistente
        if Path::new(cache_path).exists() {
            if let Ok(content) = fs::read_to_string(cache_path) {
                cache.load_from_json(&content);
            }
        }
        
        cache
    }
    
    /// Cerca un template in cache per pattern
    pub fn find(&self, request: &str) -> Option<&CachedTemplate> {
        let lower_request = request.to_lowercase();
        
        for (_, template) in &self.templates {
            let patterns: Vec<&str> = template.pattern.split('|').collect();
            for pattern in patterns {
                if lower_request.contains(pattern) {
                    return Some(template);
                }
            }
        }
        
        None
    }
    
    /// Aggiunge un nuovo template alla cache
    pub fn add(&mut self, request: &str, code: &str) -> CachedTemplate {
        // Genera nome dal request
        let name = sanitize_name(request);
        let description = format!("Auto-generated for: {}", request);
        let pattern = extract_keywords(request);
        
        let template = CachedTemplate {
            name: name.clone(),
            description,
            pattern,
            code: code.to_string(),
            use_count: 1,
        };
        
        self.templates.insert(name, template.clone());
        self.modified = true;
        
        template
    }
    
    /// Incrementa contatore uso
    pub fn increment_use(&mut self, name: &str) {
        if let Some(template) = self.templates.get_mut(name) {
            template.use_count += 1;
            self.modified = true;
        }
    }
    
    /// Salva cache su file
    pub fn save(&mut self) -> Result<(), String> {
        if !self.modified {
            return Ok(());
        }
        
        let json = self.to_json();
        fs::write(&self.cache_file, json)
            .map_err(|e| format!("Failed to save cache: {}", e))?;
        
        self.modified = false;
        Ok(())
    }
    
    /// Lista tutti i template in cache
    pub fn list(&self) -> Vec<&CachedTemplate> {
        self.templates.values().collect()
    }
    
    /// Numero di template in cache
    pub fn len(&self) -> usize {
        self.templates.len()
    }
    
    /// Carica da JSON
    fn load_from_json(&mut self, json: &str) {
        // Formato semplice: una riga per template
        // name|pattern|use_count|code
        for line in json.lines() {
            let parts: Vec<&str> = line.splitn(4, '|').collect();
            if parts.len() == 4 {
                if let Ok(use_count) = parts[2].parse() {
                    let template = CachedTemplate {
                        name: parts[0].to_string(),
                        description: format!("Cached: {}", parts[0]),
                        pattern: parts[1].to_string(),
                        code: parts[3].to_string(),
                        use_count,
                    };
                    self.templates.insert(parts[0].to_string(), template);
                }
            }
        }
    }
    
    /// Converte in JSON
    fn to_json(&self) -> String {
        let mut lines = Vec::new();
        for (name, template) in &self.templates {
            lines.push(format!(
                "{}|{}|{}|{}",
                name,
                template.pattern,
                template.use_count,
                template.code.replace('\n', "\\n")
            ));
        }
        lines.join("\n")
    }
}

/// Sanitizza un nome per usarlo come identificatore
fn sanitize_name(request: &str) -> String {
    let lower = request.to_lowercase();
    let cleaned: String = lower
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect();
    
    let words: Vec<&str> = cleaned.split_whitespace().take(3).collect();
    words.join("_")
}

/// Estrae keyword dal request per pattern matching
fn extract_keywords(request: &str) -> String {
    let lower = request.to_lowercase();
    let words: Vec<&str> = lower
        .split_whitespace()
        .filter(|w| w.len() > 3)  // Solo parole significative
        .filter(|w| !is_stop_word(w))
        .take(3)
        .collect();
    
    words.join("|")
}

/// Verifica se è una stop word
fn is_stop_word(word: &str) -> bool {
    let stop_words = [
        "crea", "create", "funzione", "function", "calcola", "calculate",
        "per", "for", "che", "that", "the", "and", "una", "a", "un",
    ];
    stop_words.contains(&word)
}

impl CachedTemplate {
    /// Divide in funzioni e statements
    pub fn split_code(&self) -> (String, String) {
        let lines: Vec<&str> = self.code.lines().collect();
        let mut functions = Vec::new();
        let mut statements = Vec::new();
        let mut in_function = false;
        let mut current_function = Vec::new();
        
        for line in lines {
            if line.starts_with("fn ") {
                in_function = true;
                current_function.clear();
                current_function.push(line);
            } else if in_function {
                current_function.push(line);
                if line == "}" {
                    in_function = false;
                    functions.push(current_function.join("\n"));
                }
            } else {
                statements.push(line);
            }
        }
        
        (
            functions.join("\n"),
            statements.join("\n")
        )
    }
}
