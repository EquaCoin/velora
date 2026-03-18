/// Sistema AI avanzato con validazione e retry per Velora AI-First
use crate::ai::{generate_velora_code, is_ai_configured};
use crate::parser::parse;

const MAX_RETRIES: u32 = 3;

/// Genera codice con validazione e retry automatico
pub fn generate_velora_code_with_retry(
    request: &str,
    max_attempts: u32
) -> Result<(String, String), String> {
    if !is_ai_configured() {
        return Err("Nessun provider AI configurato".to_string());
    }

    let mut last_error = String::new();
    
    for attempt in 1..=max_attempts {
        println!("   Tentativo {}/{}...", attempt, max_attempts);
        
        // Costruisci prompt con feedback dagli errori precedenti
        let prompt = build_prompt(request, &last_error);
        
        match generate_velora_code(&prompt) {
            Ok(code) => {
                // Valida il codice generato
                match validate_and_split_code(&code) {
                    Ok((functions, statements)) => {
                        return Ok((functions, statements));
                    }
                    Err(e) => {
                        last_error = format!(
                            "Tentativo {} fallito: {}. Codice problematico:\n{}",
                            attempt, e, code
                        );
                        println!("   ⚠️  Validazione fallita: {}", e);
                    }
                }
            }
            Err(e) => {
                last_error = format!("Tentativo {} - Generazione fallita: {}", attempt, e);
                println!("   ❌ Generazione fallita: {}", e);
            }
        }
    }
    
    // Tutti i tentativi falliti, usa fallback
    println!("   🔄 Usando fallback...");
    generate_fallback_code(request)
}

/// Costruisce un prompt ottimizzato con eventuale feedback errore
fn build_prompt(request: &str, previous_error: &str) -> String {
    let base_prompt = format!(
        "Generate VALID Velora code for: {}\n\n\
         CRITICAL RULES:\n\
         1. Output ONLY valid code - NO explanations, NO markdown\n\
         2. Syntax:\n\
            - let x = 10\n\
            - print(x)\n\
            - fn name(x) -> Int {{ return x * 2 }}\n\
            - if x > 5 {{ print(\"yes\") }}\n\
            - while x > 0 {{ print(x); let x = x - 1 }}\n\
         3. NO comments starting with #\n\
         4. Use semicolons ONLY to separate statements on same line\n\
         5. All functions must end with }}\n",
        request
    );
    
    if !previous_error.is_empty() {
        format!(
            "{}\n\n\
             PREVIOUS ATTEMPT FAILED:\n\
             {}\n\n\
             Fix the errors and generate correct code:\n",
            base_prompt, previous_error
        )
    } else {
        base_prompt
    }
}

/// Valida il codice e lo divide in funzioni e statement
fn validate_and_split_code(code: &str) -> Result<(String, String), String> {
    // Estrai solo le righe che sembrano codice valido
    let valid_lines: Vec<&str> = code
        .lines()
        .filter(|l| {
            let trimmed = l.trim();
            !trimmed.is_empty() &&
            !trimmed.starts_with("//") &&
            !trimmed.starts_with("#") &&
            !is_explanation_line(trimmed)
        })
        .collect();
    
    let cleaned_code = valid_lines.join("\n");
    
    // Prova a fare il parsing
    let test_code = format!("main:\n{}", cleaned_code);
    
    match parse(&test_code) {
        Ok(_) => {
            // Dividi in funzioni e statement
            let mut functions = Vec::new();
            let mut statements = Vec::new();
            let mut in_function = false;
            let mut current_function = Vec::new();
            let mut brace_count = 0;
            
            for line in valid_lines {
                let trimmed = line.trim();
                
                if trimmed.starts_with("fn ") {
                    in_function = true;
                    brace_count = 0;
                    current_function.clear();
                    current_function.push(line);
                    brace_count += count_braces(trimmed);
                } else if in_function {
                    current_function.push(line);
                    brace_count += count_braces(trimmed);
                    
                    if brace_count > 0 && brace_count == count_closing_braces(&current_function.join("\n")) {
                        // Funzione completata
                        functions.push(current_function.join("\n"));
                        in_function = false;
                        brace_count = 0;
                    }
                } else {
                    statements.push(line);
                }
            }
            
            Ok((
                functions.join("\n"),
                statements.join("\n")
            ))
        }
        Err(e) => {
            Err(format!("Parse error: {}", e))
        }
    }
}

/// Conta le graffe aperte in una riga
fn count_braces(line: &str) -> i32 {
    line.chars().filter(|&c| c == '{').count() as i32
}

/// Conta le graffe chiuse nel codice
fn count_closing_braces(code: &str) -> i32 {
    code.chars().filter(|&c| c == '}').count() as i32
}

/// Verifica se una riga è chiaramente spiegazione
fn is_explanation_line(line: &str) -> bool {
    let lower = line.to_lowercase();
    let explanation_patterns = [
        "here is", "this is", "that is", "note:", "example:",
        "the code", "this function", "this will", "you can",
        "make sure", "ensure", "remember", "following"
    ];
    
    explanation_patterns.iter().any(|&p| lower.contains(p))
}

/// Genera codice di fallback quando l'AI fallisce
fn generate_fallback_code(request: &str) -> Result<(String, String), String> {
    let lower = request.to_lowercase();
    
    // Pattern matching per richieste comuni
    if lower.contains("somma") || lower.contains("sum") {
        Ok((
            "fn sum_to_n(n) -> Int {\n    let total = 0\n    while n > 0 {\n        total = total + n\n        let n = n - 1\n    }\n    return total\n}".to_string(),
            "print(sum_to_n(10))".to_string()
        ))
    } else if lower.contains("fattoriale") || lower.contains("factorial") {
        Ok((
            "fn factorial(n) -> Int {\n    if n <= 1 {\n        return 1\n    }\n    return n * factorial(n - 1)\n}".to_string(),
            "print(factorial(5))".to_string()
        ))
    } else if lower.contains("primo") || lower.contains("prime") {
        Ok((
            "fn is_prime(n) -> Bool {\n    if n <= 1 {\n        return false\n    }\n    if n == 2 {\n        return true\n    }\n    let i = 2\n    while i * i <= n {\n        if n % i == 0 {\n            return false\n        }\n        let i = i + 1\n    }\n    return true\n}".to_string(),
            "print(is_prime(17))".to_string()
        ))
    } else if lower.contains("fibonacci") {
        Ok((
            "fn fib(n) -> Int {\n    if n <= 0 {\n        return 0\n    }\n    if n == 1 {\n        return 1\n    }\n    return fib(n - 1) + fib(n - 2)\n}".to_string(),
            "print(fib(10))".to_string()
        ))
    } else if lower.contains("doppio") || lower.contains("double") {
        Ok((
            "fn double(x) -> Int {\n    return x * 2\n}".to_string(),
            "print(double(21))".to_string()
        ))
    } else {
        // Fallback generico
        Ok((
            format!("fn ai_generated() -> Int {{\n    print(\"AI code for: {}\")\n    return 42\n}}", request).to_string(),
            "print(ai_generated())".to_string()
        ))
    }
}
