/// Integrazione GROK API (xAI) per Velora
/// Genera template dinamici quando i predefiniti non bastano

use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug)]
pub enum GrokError {
    NoApiKey,
    RequestFailed(String),
    ParseFailed(String),
    InvalidResponse,
}

impl std::fmt::Display for GrokError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrokError::NoApiKey => write!(f, "GROK_API_KEY non impostata"),
            GrokError::RequestFailed(e) => write!(f, "Richiesta fallita: {}", e),
            GrokError::ParseFailed(e) => write!(f, "Parse fallito: {}", e),
            GrokError::InvalidResponse => write!(f, "Risposta invalida da Grok"),
        }
    }
}

impl std::error::Error for GrokError {}

#[derive(Serialize)]
struct GrokRequest {
    model: String,
    messages: Vec<GrokMessage>,
    temperature: f32,
    max_tokens: i32,
}

#[derive(Serialize, Deserialize)]
struct GrokMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct GrokResponse {
    choices: Vec<GrokChoice>,
}

#[derive(Deserialize)]
struct GrokChoice {
    message: GrokMessage,
}

/// Genera un template Velora usando GROK
pub fn generate_template_with_grok(description: &str) -> Result<String, GrokError> {
    let api_key = env::var("GROK_API_KEY")
        .map_err(|_| GrokError::NoApiKey)?;
    
    let client = reqwest::blocking::Client::new();
    
    // Prompt ottimizzato per Grok
    let system_prompt = r#"You are a Velora programming language code generator.

Generate ONLY valid Velora code. No explanations, no markdown.

Velora syntax rules:
- Variables: let x = 10 (NO mut keyword)
- Print: print(x)
- Functions: fn name(x) -> Int { return x * 2 }
- Types: Int, String
- If: if x > 5 { print("yes") } else { print("no") }
- While: while x > 0 { print(x); let x = x - 1 }
- Operators: +, -, *, /, %, ==, !=, <, >, <=, >=
- NO comments starting with #
- NO := operator
- Use + for string concatenation

IMPORTANT: Return ONLY the function and a test call.
Format:
fn function_name(params) -> ReturnType {
    // code
    return result
}
print(function_name(test_value))"#;

    let request = GrokRequest {
        model: "grok-code-fast-1".to_string(),
        messages: vec![
            GrokMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            GrokMessage {
                role: "user".to_string(),
                content: format!("Generate Velora code for: {}", description),
            },
        ],
        temperature: 0.2,  // Basso per codice deterministico
        max_tokens: 500,
    };

    let response = client
        .post("https://api.x.ai/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .map_err(|e| GrokError::RequestFailed(e.to_string()))?;

    if !response.status().is_success() {
        let error_text = response.text().unwrap_or_default();
        if error_text.contains("Bad credentials") {
            return Err(GrokError::RequestFailed(
                "API Key non valida. Verifica che GROK_API_KEY sia corretta.".to_string()
            ));
        }
        return Err(GrokError::RequestFailed(error_text));
    }

    let grok_response: GrokResponse = response
        .json()
        .map_err(|e| GrokError::ParseFailed(e.to_string()))?;

    grok_response
        .choices
        .into_iter()
        .next()
        .map(|c| extract_code(&c.message.content))
        .ok_or(GrokError::InvalidResponse)
}

/// Estrae e fixa il codice dalla risposta di Grok
fn extract_code(text: &str) -> String {
    let text = text.trim();
    
    // Fix comuni di sintassi
    let text = text.replace("fn ", "fn ");
    // Rimuovi spazi prima di ->
    let text = text.replace(" ) -> ", ") -> ");
    let text = text.replace(" ) ->", ") ->");
    
    let text = text.trim();
    
    // Se c'è un code block, estrailo
    if let Some(start) = text.find("```") {
        let after_start = &text[start + 3..];
        let code_start = after_start.find('\n').unwrap_or(0);
        let after_lang = &after_start[code_start..];
        
        if let Some(end) = after_lang.find("```") {
            return after_lang[..end].trim().to_string();
        }
    }
    
    // Filtra righe che sembrano codice Velora
    let valid_prefixes = ["fn ", "let ", "print(", "if ", "while ", "return"];
    let mut result = Vec::new();
    
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Ignora righe con testo spiegativo
        let lower = trimmed.to_lowercase();
        if lower.contains("here is") || 
           lower.contains("this code") ||
           lower.contains("the function") {
            continue;
        }
        if valid_prefixes.iter().any(|&p| trimmed.starts_with(p)) {
            result.push(trimmed.to_string());
        }
    }
    
    result.join("\n")
}

/// Verifica se GROK è configurato e valido
pub fn is_grok_configured() -> bool {
    match env::var("GROK_API_KEY") {
        Ok(key) if !key.is_empty() && key.starts_with("xai-") => true,
        _ => false,
    }
}

/// Info su GROK
pub fn get_grok_info() -> Option<String> {
    if is_grok_configured() {
        Some("GROK (xAI)".to_string())
    } else {
        None
    }
}
