/// Modulo AI ottimizzato per Ollama su NVIDIA DGX Spark
/// Usa modelli Nemotron nativi NVIDIA per migliori performance

use serde::{Deserialize, Serialize};
use std::env;
use std::collections::HashMap;

// Usa il tipo AiError del modulo principale
pub use super::AiError;

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

/// Genera codice usando Ollama con ottimizzazioni DGX Spark
pub fn generate_velora_code(prompt: &str) -> Result<String, AiError> {
    let model = select_best_model()?;
    
    let system_prompt = r#"You are a Velora code generator.
Output ONLY valid Velora code. NO explanations. NO markdown. NO comments.

Velora syntax rules:
- let x = 10 (variables, NO mut, NO type annotations)
- print(x) (NO println)
- fn name(x) -> Int { return x * 2 } (types: Int, String, Bool)
- if x > 5 { print("yes") } else { print("no") }
- while x > 0 { print(x); let x = x - 1 }
- NO := operator, NO // comments, NO /* */ comments

Example valid code:
fn factorial(n) -> Int {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}
print(factorial(5))

Generate ONLY code for:"#;

    let request = OllamaRequest {
        model: model.clone(),
        prompt: format!("{} {}", system_prompt, prompt),
        stream: false,
        options: {
            let mut opts = HashMap::new();
            opts.insert("temperature".to_string(), serde_json::json!(0.2));
            opts.insert("num_predict".to_string(), serde_json::json!(300));
            // Ottimizzazioni DGX Spark
            opts.insert("num_gpu".to_string(), serde_json::json!(99)); // Usa tutti i layer GPU
            opts.insert("num_thread".to_string(), serde_json::json!(8));
            opts
        },
    };

    let client = reqwest::blocking::Client::new();
    let ollama_host = env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string());
    
    let response = client
        .post(format!("{}/api/generate", ollama_host))
        .json(&request)
        .send()
        .map_err(|e| AiError::RequestFailed(format!(
            "Failed to connect to Ollama: {}. Is it running?", e
        )))?;

    if !response.status().is_success() {
        let error_text = response.text().unwrap_or_default();
        return Err(AiError::RequestFailed(error_text));
    }

    let ai_response: OllamaResponse = response
        .json()
        .map_err(|e| AiError::RequestFailed(e.to_string()))?;

    Ok(extract_code(&ai_response.response))
}

/// Seleziona il miglior modello disponibile
fn select_best_model() -> Result<String, AiError> {
    let preferred = vec![
        "nemotron-mini-4b",      // Leggero, veloce, nativo NVIDIA
        "nemotron-30b",          // Potente ma pesante
        "deepseek-coder",        // Buono per codice
        "qwen2.5-7b",            // Bilanciato
        "codellama-7b",          // Ottimo per codice
    ];
    
    // Ottieni lista modelli disponibili
    let ollama_host = env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string());
    
    if let Ok(response) = reqwest::blocking::get(format!("{}/api/tags", ollama_host)) {
        if let Ok(models) = response.json::<serde_json::Value>() {
            let available: Vec<String> = models["models"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            
            // Trova il primo modello preferito disponibile
            for pref in &preferred {
                for avail in &available {
                    if avail.starts_with(pref) {
                        return Ok(avail.clone());
                    }
                }
            }
            
            // Se nessuno dei preferiti, usa il primo disponibile
            if let Some(first) = available.first() {
                return Ok(first.clone());
            }
        }
    }
    
    // Fallback al modello di default
    Ok("nemotron-mini-4b:latest".to_string())
}

/// Estrae codice valido dalla risposta
fn extract_code(text: &str) -> String {
    let text = text.trim();
    
    // Se c'è un code block
    if let Some(start) = text.find("```") {
        let after_start = &text[start + 3..];
        let code_start = after_start.find('\n').unwrap_or(0);
        let after_lang = &after_start[code_start..];
        
        if let Some(end) = after_lang.find("```") {
            return filter_valid_lines(after_lang[..end].trim());
        }
    }
    
    filter_valid_lines(text)
}

/// Filtra solo righe valide di Velora
fn filter_valid_lines(text: &str) -> String {
    let valid_starts = ["fn ", "main:", "let ", "print(", "if ", "while ", "return", "}", "{"];
    let explanation_words = [
        "here", "this", "that", "code", "function", "creates", "example",
        "explanation", "note", "following", "above", "below"
    ];
    
    let mut result = Vec::new();
    
    for line in text.lines() {
        let trimmed = line.trim();
        
        if trimmed.is_empty() {
            continue;
        }
        
        // Ignora righe con parole spiegative
        let lower = trimmed.to_lowercase();
        if explanation_words.iter().any(|&w| lower.contains(w)) {
            continue;
        }
        
        // Accetta solo righe che sembrano codice
        if valid_starts.iter().any(|&s| trimmed.starts_with(s)) && trimmed.len() < 80 {
            result.push(trimmed.to_string());
        }
    }
    
    result.join("\n")
}

/// Verifica se Ollama è configurato
pub fn is_ai_configured() -> bool {
    let ollama_host = env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string());
    
    if let Ok(response) = reqwest::blocking::get(format!("{}/api/tags", ollama_host)) {
        response.status().is_success()
    } else {
        false
    }
}

/// Info sul sistema
pub fn get_configured_provider() -> Option<String> {
    if is_ai_configured() {
        if let Ok(model) = select_best_model() {
            return Some(format!("Ollama + {} (DGX Spark)", model));
        }
        Some("Ollama (DGX Spark)".to_string())
    } else {
        None
    }
}
