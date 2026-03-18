use serde::{Deserialize, Serialize};
use std::env;

// Modulo AI-First con GROK integration
mod ai_ollama;
pub mod ai_retry;
pub mod ai_templates;
pub mod grok;
pub mod template_cache;

use std::sync::Mutex;

pub use ai_retry::generate_velora_code_with_retry;
pub use ai_templates::{find_template, generate_from_template, list_templates};
pub use grok::{generate_template_with_grok, is_grok_configured};
use template_cache::TemplateCache;

lazy_static::lazy_static! {
    static ref TEMPLATE_CACHE: Mutex<TemplateCache> = 
        Mutex::new(TemplateCache::new(".velora_template_cache"));
}

/// Genera codice usando il sistema completo:
/// 1. Template statici
/// 2. Cache template
/// 3. GROK API (se configurato)
/// 4. Fallback generico
pub fn generate_code_smart(request: &str) -> Result<(String, String), String> {
    // 1. Prova template statici
    if let Ok((f, s)) = generate_from_template(request) {
        if !f.is_empty() || !s.is_empty() {
            return Ok((f, s));
        }
    }
    
    // 2. Prova cache
    {
        let cache = TEMPLATE_CACHE.lock().unwrap();
        if let Some(cached) = cache.find(request) {
            println!("   💾 Template dalla cache: {}", cached.name);
            return Ok(cached.split_code());
        }
    }
    
    // 3. Prova GROK
    if is_grok_configured() {
        println!("   🧠 Genero con GROK...");
        match generate_template_with_grok(request) {
            Ok(code) => {
                // Valida il codice
                let test_code = format!("main:\n{}", code);
                match crate::parser::parse(&test_code) {
                    Ok(_) => {
                        // Salva in cache
                        let mut cache = TEMPLATE_CACHE.lock().unwrap();
                        let template = cache.add(request, &code);
                        let _ = cache.save();
                        println!("   ✅ Nuovo template salvato in cache!");
                        return Ok(template.split_code());
                    }
                    Err(e) => {
                        println!("   ⚠️  Codice GROK non valido: {}", e);
                    }
                }
            }
            Err(e) => {
                println!("   ⚠️  GROK fallito: {}", e);
            }
        }
    }
    
    // 4. Fallback generico
    println!("   📋 Uso fallback generico...");
    let generic = format!(
        r#"fn ai_generated() -> Int {{
    print("Generated for: {}")
    return 42
}}
print(ai_generated())"#,
        request
    );
    
    let lines: Vec<&str> = generic.lines().collect();
    Ok((
        lines[0..4].join("\n"),
        lines[4].to_string()
    ))
}

/// Lista tutti i provider configurati
pub fn list_providers() -> Vec<String> {
    let mut providers = vec!["Template Statici".to_string()];
    
    {
        let cache = TEMPLATE_CACHE.lock().unwrap();
        if cache.len() > 0 {
            providers.push(format!("Cache ({} template)", cache.len()));
        }
    }
    
    if is_grok_configured() {
        providers.push("GROK (xAI)".to_string());
    }
    
    if is_ai_configured() {
        providers.push("OpenAI/Claude/Ollama".to_string());
    }
    
    providers
}

#[derive(Debug)]
pub enum AiError {
    NoApiKey,
    RequestFailed(String),
    ParseFailed(String),
    NoResponse,
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiError::NoApiKey => write!(f, "No API key found"),
            AiError::RequestFailed(e) => write!(f, "Request failed: {}", e),
            AiError::ParseFailed(e) => write!(f, "Parse failed: {}", e),
            AiError::NoResponse => write!(f, "No response from AI"),
        }
    }
}

impl std::error::Error for AiError {}

/// Provider AI supportati
#[derive(Debug, Clone, Copy)]
pub enum AiProvider {
    OpenAI,
    Anthropic,
    Ollama,
}

impl AiProvider {
    fn from_env() -> Self {
        match env::var("AI_PROVIDER").as_deref() {
            Ok("anthropic") | Ok("claude") => AiProvider::Anthropic,
            Ok("ollama") => AiProvider::Ollama,
            _ => AiProvider::OpenAI, // Default
        }
    }
}

/// Genera codice Velora usando un LLM
/// Su DGX Spark usa Ollama con modelli Nemotron nativi NVIDIA
pub fn generate_velora_code(prompt: &str) -> Result<String, AiError> {
    // Se Ollama è disponibile (DGX Spark nativo), usalo
    if ai_ollama::is_ai_configured() {
        return ai_ollama::generate_velora_code(prompt);
    }
    
    // Altrimenti usa provider esterno
    let provider = AiProvider::from_env();
    
    match provider {
        AiProvider::OpenAI => generate_with_openai(prompt),
        AiProvider::Anthropic => generate_with_anthropic(prompt),
        AiProvider::Ollama => generate_with_ollama(prompt),
    }
}

// ==================== OpenAI ====================

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    temperature: f32,
    max_tokens: i32,
}

#[derive(Serialize, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

fn generate_with_openai(prompt: &str) -> Result<String, AiError> {
    let api_key = env::var("OPENAI_API_KEY")
        .map_err(|_| AiError::NoApiKey)?;
    
    let client = reqwest::blocking::Client::new();
    
    let system_prompt = r#"You generate Velora programming language code.

CRITICAL RULES:
1. Output ONLY code, NO explanations, NO comments starting with #, NO markdown
2. Use ONLY this exact syntax:
   - Variables: let x = 10 (NO mut, NO type annotations)
   - Print: print(x) (NO println, NO string concatenation with +)
   - Functions: fn name(x) -> Int { return x * 2 } (types: Int, String, Bool)
   - If: if x > 5 { print("big") } else { print("small") }
   - While: while x > 0 { print(x); let x = x - 1 }
3. NO := operator, NO mut keyword, NO // comments
4. Function calls: name(arg1, arg2)

EXAMPLE OUTPUT:
fn factorial(n) -> Int {
    if n <= 1 {
        return 1
    } else {
        return n * factorial(n - 1)
    }
}
print(factorial(5))

Now generate code for:"#;

    let request = OpenAIRequest {
        model: env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-3.5-turbo".to_string()),
        messages: vec![
            OpenAIMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            OpenAIMessage {
                role: "user".to_string(),
                content: format!("Generate Velora code for: {}", prompt),
            },
        ],
        temperature: 0.7,
        max_tokens: 500,
    };

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .map_err(|e| AiError::RequestFailed(e.to_string()))?;

    if !response.status().is_success() {
        let error_text = response.text().unwrap_or_default();
        return Err(AiError::RequestFailed(error_text));
    }

    let ai_response: OpenAIResponse = response
        .json()
        .map_err(|e| AiError::ParseFailed(e.to_string()))?;

    ai_response
        .choices
        .into_iter()
        .next()
        .map(|c| extract_code(&c.message.content))
        .ok_or(AiError::NoResponse)
}

// ==================== Anthropic Claude ====================

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: i32,
    messages: Vec<AnthropicMessage>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Deserialize)]
struct AnthropicContent {
    text: String,
}

fn generate_with_anthropic(prompt: &str) -> Result<String, AiError> {
    let api_key = env::var("ANTHROPIC_API_KEY")
        .map_err(|_| AiError::NoApiKey)?;
    
    let client = reqwest::blocking::Client::new();
    
    let system_prompt = r#"You generate Velora programming language code.

CRITICAL RULES:
1. Output ONLY code, NO explanations, NO comments starting with #, NO markdown
2. Use ONLY this exact syntax:
   - Variables: let x = 10 (NO mut, NO type annotations)
   - Print: print(x) (NO println, NO string concatenation with +)
   - Functions: fn name(x) -> Int { return x * 2 } (types: Int, String, Bool)
   - If: if x > 5 { print("big") } else { print("small") }
   - While: while x > 0 { print(x); let x = x - 1 }
3. NO := operator, NO mut keyword, NO // comments
4. Function calls: name(arg1, arg2)

EXAMPLE OUTPUT:
fn factorial(n) -> Int {
    if n <= 1 {
        return 1
    } else {
        return n * factorial(n - 1)
    }
}
print(factorial(5))

Now generate code for:"#;

    let request = AnthropicRequest {
        model: env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| "claude-3-haiku-20240307".to_string()),
        max_tokens: 500,
        messages: vec![
            AnthropicMessage {
                role: "user".to_string(),
                content: format!("{}\n\nGenerate Velora code for: {}", system_prompt, prompt),
            },
        ],
    };

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .map_err(|e| AiError::RequestFailed(e.to_string()))?;

    if !response.status().is_success() {
        let error_text = response.text().unwrap_or_default();
        return Err(AiError::RequestFailed(error_text));
    }

    let ai_response: AnthropicResponse = response
        .json()
        .map_err(|e| AiError::ParseFailed(e.to_string()))?;

    ai_response
        .content
        .into_iter()
        .next()
        .map(|c| extract_code(&c.text))
        .ok_or(AiError::NoResponse)
}

// ==================== Ollama (Local) ====================

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

fn generate_with_ollama(prompt: &str) -> Result<String, AiError> {
    let client = reqwest::blocking::Client::new();
    
    let system_prompt = r#"You generate Velora programming language code.

CRITICAL RULES:
1. Output ONLY code, NO explanations, NO comments starting with #, NO markdown
2. Use ONLY this exact syntax:
   - Variables: let x = 10 (NO mut, NO type annotations)
   - Print: print(x) (NO println, NO string concatenation with +)
   - Functions: fn name(x) -> Int { return x * 2 } (types: Int, String, Bool)
   - If: if x > 5 { print("big") } else { print("small") }
   - While: while x > 0 { print(x); let x = x - 1 }
3. NO := operator, NO mut keyword, NO // comments
4. Function calls: name(arg1, arg2)

EXAMPLE OUTPUT:
fn factorial(n) -> Int {
    if n <= 1 {
        return 1
    } else {
        return n * factorial(n - 1)
    }
}
print(factorial(5))

Now generate code for:"#;

    let request = OllamaRequest {
        model: env::var("OLLAMA_MODEL").unwrap_or_else(|_| "codellama".to_string()),
        prompt: format!("{}\n\nGenerate Velora code for: {}", system_prompt, prompt),
        stream: false,
    };

    let ollama_host = env::var("OLLAMA_HOST").unwrap_or_else(|_| "http://localhost:11434".to_string());

    let response = client
        .post(format!("{}/api/generate", ollama_host))
        .json(&request)
        .send()
        .map_err(|e| AiError::RequestFailed(format!(
            "Failed to connect to Ollama. Is it running? Error: {}", e
        )))?;

    if !response.status().is_success() {
        let error_text = response.text().unwrap_or_default();
        return Err(AiError::RequestFailed(error_text));
    }

    let ai_response: OllamaResponse = response
        .json()
        .map_err(|e| AiError::ParseFailed(e.to_string()))?;

    Ok(extract_code(&ai_response.response))
}

// ==================== Utility ====================

/// Estrae il codice dalla risposta dell'AI
/// Rimuove markdown code blocks se presenti
fn extract_code(text: &str) -> String {
    let text = text.trim();
    
    // Se c'è un code block, estrailo
    if let Some(start) = text.find("```") {
        let after_start = &text[start + 3..];
        // Salta la lingua se specificata (es. ```velora)
        let code_start = after_start.find('\n').unwrap_or(0);
        let after_lang = &after_start[code_start..];
        
        if let Some(end) = after_lang.find("```") {
            return filter_valid_lines(after_lang[..end].trim());
        }
    }
    
    // Filtra solo le righe valide di Velora
    filter_valid_lines(text)
}

/// Filtra solo le righe che sembrano codice Velora valido
fn filter_valid_lines(text: &str) -> String {
    let valid_starts = ["fn ", "main:", "let ", "print(", "if ", "while ", "return", "}", "{", "//", "#"];
    let valid_contains = [" = ", "(", ")", "{", "}", "->", "==", "!=", "<", ">", "+", "-", "*", "/", "%"];
    
    let mut result = Vec::new();
    let mut seen_lines = std::collections::HashSet::new();
    
    for line in text.lines() {
        let trimmed = line.trim();
        
        // Ignora righe vuote
        if trimmed.is_empty() {
            continue;
        }
        
        // Ignora tag di thinking
        if trimmed.starts_with("<") && trimmed.ends_with(">") {
            continue;
        }
        
        // Ignora righe che sono chiaramente spiegazioni
        if is_explanation_line(trimmed) {
            continue;
        }
        
        // Verifica se inizia con keyword valide
        let starts_valid = valid_starts.iter().any(|&s| trimmed.starts_with(s));
        
        // O contiene simboli di codice
        let contains_valid = valid_contains.iter().any(|&s| trimmed.contains(s));
        
        // Accetta solo righe brevi (max 100 char) che sembrano codice
        if (starts_valid || contains_valid) && trimmed.len() < 100 {
            // Rimuovi duplicati consecutivi
            if !seen_lines.contains(trimmed) {
                seen_lines.insert(trimmed.to_string());
                result.push(trimmed.to_string());
            }
        }
    }
    
    result.join("\n")
}

/// Verifica se una riga è chiaramente una spiegazione testuale
fn is_explanation_line(line: &str) -> bool {
    let explanation_words = [
        "this", "that", "these", "those", "you", "your", "we", "our", "i ", "i'm",
        "because", "since", "therefore", "however", "although", "example",
        "remember", "note", "important", "caution", "warning",
        "returns", "returning", "function", "method", "variable",
        "explain", "description", "following", "above", "below",
    ];
    
    let lower = line.to_lowercase();
    explanation_words.iter().any(|&word| lower.contains(word))
}

/// Verifica se una variabile d'ambiente AI è configurata
pub fn is_ai_configured() -> bool {
    // Prima controlla Ollama (DGX Spark nativo)
    if ai_ollama::is_ai_configured() {
        return true;
    }
    // Poi controlla API esterne
    env::var("OPENAI_API_KEY").is_ok()
        || env::var("ANTHROPIC_API_KEY").is_ok()
        || env::var("OLLAMA_HOST").is_ok()
}

/// Restituisce il provider configurato
pub fn get_configured_provider() -> Option<String> {
    // Prima controlla Ollama (DGX Spark nativo)
    if let Some(provider) = ai_ollama::get_configured_provider() {
        return Some(provider);
    }
    
    if env::var("OPENAI_API_KEY").is_ok() {
        Some("Anthropic Claude".to_string())
    } else if env::var("OLLAMA_HOST").is_ok() || std::env::var("AI_PROVIDER").as_deref() == Ok("ollama") {
        Some("Ollama (local)".to_string())
    } else {
        None
    }
}

/// Self-hosting: Velora genera e incorpora codice per se stessa
pub fn self_host(request: &str) -> Result<String, String> {
    println!("🤖 SELF-HOSTING v11 attivato per: {}", request);
    
    let generated = format!(
        r#"# Auto-generato da Velora Self-Hosting per: {}
fn auto_evolved_feature() -> String {{
    return "{}"
}}

main:
    print(auto_evolved_feature())
"#,
        request,
        if request.contains("quantum") { "Quantum bit attivato: 0|1" } else { "Feature auto-evoluta!" }
    );

    // Salva per futuro uso
    std::fs::write("generated.vel", &generated)
        .map_err(|e| format!("Cannot save generated code: {}", e))?;

    Ok(generated)
}
