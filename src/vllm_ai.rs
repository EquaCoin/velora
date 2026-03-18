/// Modulo AI che usa vLLM invece di API esterne
/// Ottimizzato per NVIDIA DGX Spark con TensorRT e CUDA

use serde::{Deserialize, Serialize};
use std::process::Command;
use std::env;

#[derive(Debug)]
pub enum AiError {
    VllmNotAvailable,
    RequestFailed(String),
    NoResponse,
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiError::VllmNotAvailable => write!(f, "vLLM not available"),
            AiError::RequestFailed(e) => write!(f, "Request failed: {}", e),
            AiError::NoResponse => write!(f, "No response from AI"),
        }
    }
}

impl std::error::Error for AiError {}

/// Usa vLLM per generare codice
pub fn generate_with_vllm(prompt: &str) -> Result<String, AiError> {
    let model = env::var("VLLM_MODEL").unwrap_or_else(|_| "nemotron-30b".to_string());
    
    // Costruisci il prompt per codice Velora
    let system_prompt = r#"You generate Velora programming language code.

CRITICAL RULES:
1. Output ONLY code, NO explanations, NO markdown
2. Use ONLY this syntax:
   - Variables: let x = 10
   - Print: print(x)
   - Functions: fn name(x) -> Int { return x * 2 }
3. NO comments, NO text, ONLY code

Generate code for:"#;

    let full_prompt = format!("{} {}\n\n", system_prompt, prompt);
    
    // Prova a usare vllm command line o Python
    let output = Command::new("python3")
        .args(&[
            "-c",
            &format!(
                r#"
import sys
try:
    from vllm import LLM, SamplingParams
    
    # Inizializza il modello (usa quello già scaricato)
    llm = LLM(model="{}", tensor_parallel_size=1, gpu_memory_utilization=0.9)
    
    sampling_params = SamplingParams(
        temperature=0.3,
        max_tokens=500,
        stop=["\n\n", "###", "<|endoftext|>"]
    )
    
    outputs = llm.generate("{}", sampling_params)
    print(outputs[0].outputs[0].text)
except Exception as e:
    print(f"Error: {{e}}", file=sys.stderr)
    sys.exit(1)
"#,
                model,
                full_prompt.replace("\"", "\\\"").replace("\n", "\\n")
            ),
        ])
        .output()
        .map_err(|e| AiError::VllmNotAvailable)?;
    
    if !output.status.success() {
        return Err(AiError::RequestFailed(
            String::from_utf8_lossy(&output.stderr).to_string()
        ));
    }
    
    let response = String::from_utf8_lossy(&output.stdout);
    Ok(extract_code(&response))
}

/// Estrae il codice dalla risposta
fn extract_code(text: &str) -> String {
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
    
    // Filtra righe valide
    let valid_starts = ["fn ", "main:", "let ", "print(", "if ", "while ", "return", "}", "{"];
    let mut result = Vec::new();
    
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        
        if valid_starts.iter().any(|&s| trimmed.starts_with(s)) && trimmed.len() < 100 {
            result.push(trimmed.to_string());
        }
    }
    
    result.join("\n")
}

/// Verifica se vLLM è disponibile
pub fn is_vllm_available() -> bool {
    Command::new("python3")
        .args(&["-c", "import vllm; print('ok')"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Ottieni informazioni sul sistema
pub fn get_system_info() -> String {
    let mut info = String::new();
    
    // GPU info
    if let Ok(output) = Command::new("nvidia-smi").args(&["--query-gpu=name,memory.total", "--format=csv,noheader"]).output() {
        info.push_str(&format!("GPU: {}", String::from_utf8_lossy(&output.stdout)));
    }
    
    // vLLM version
    if let Ok(output) = Command::new("python3").args(&["-c", "import vllm; print(vllm.__version__)"]).output() {
        info.push_str(&format!("vLLM: {}", String::from_utf8_lossy(&output.stdout)));
    }
    
    info
}
