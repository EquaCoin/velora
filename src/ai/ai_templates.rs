/// Template system per Velora AI-First
/// Fallback robusto quando l'AI esterna non è disponibile

use std::collections::HashMap;

/// Struttura template per template statici (&'static str)
pub struct CodeTemplate {
    pub name: &'static str,
    pub description: &'static str,
    pub pattern: &'static str,
    pub code: &'static str,
}

/// Struttura per template dinamici ( dalla cache)
pub struct DynamicTemplate {
    pub name: String,
    pub description: String,
    pub pattern: String,
    pub code: String,
}

/// Libreria di template predefiniti
pub static TEMPLATES: &[CodeTemplate] = &[
    CodeTemplate {
        name: "sum_to_n",
        description: "Somma numeri da 1 a N",
        pattern: "somma|sum|totale|1 a n",
        code: r#"fn sum_to_n(n) -> Int {
    return n * (n + 1) / 2
}
print(sum_to_n(10))"#,
    },
    CodeTemplate {
        name: "factorial",
        description: "Calcolo fattoriale",
        pattern: "fattoriale|factorial|n!",
        code: r#"fn factorial(n) -> Int {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}
print(factorial(5))"#,
    },
    CodeTemplate {
        name: "is_prime",
        description: "Verifica numero primo (1=primo, 0=non primo)",
        pattern: "primo|prime|divisibile",
        code: r#"fn is_prime(n) -> Int {
    if n <= 1 {
        return 0
    }
    if n == 2 {
        return 1
    }
    if n % 2 == 0 {
        return 0
    }
    return 1
}
print(is_prime(17))
print(is_prime(4))"#,
    },
    CodeTemplate {
        name: "fibonacci",
        description: "Sequenza Fibonacci",
        pattern: "fibonacci|fib|serie",
        code: r#"fn fib(n) -> Int {
    if n <= 0 {
        return 0
    }
    if n == 1 {
        return 1
    }
    return fib(n - 1) + fib(n - 2)
}
print(fib(10))"#,
    },
    CodeTemplate {
        name: "double",
        description: "Raddoppia un numero",
        pattern: "raddoppia|double|* 2",
        code: r#"fn double(x) -> Int {
    return x * 2
}
print(double(21))"#,
    },
    CodeTemplate {
        name: "cube",
        description: "Cubo di un numero",
        pattern: "cubo|cube|elevato a 3|^ 3",
        code: r#"fn cube(x) -> Int {
    return x * x * x
}
print(cube(3))"#,
    },
    CodeTemplate {
        name: "is_even",
        description: "Verifica se numero è pari",
        pattern: "pari|even|is even|divisibile per 2",
        code: r#"fn is_even(n) -> Int {
    if n % 2 == 0 {
        return 1
    }
    return 0
}
print(is_even(4))
print(is_even(7))"#,
    },
    CodeTemplate {
        name: "power",
        description: "Potenza di un numero",
        pattern: "potenza|power|^|elevato",
        code: r#"fn power(base, exp) -> Int {
    let result = 1
    let i = 0
    while i < exp {
        result = result * base
        let i = i + 1
    }
    return result
}
print(power(2, 10))"#,
    },
    CodeTemplate {
        name: "gcd",
        description: "Massimo Comun Divisore",
        pattern: "mcd|gcd|comun divisore",
        code: r#"fn gcd(a, b) -> Int {
    while b != 0 {
        let temp = b
        let b = a % b
        let a = temp
    }
    return a
}
print(gcd(48, 18))"#,
    },
    CodeTemplate {
        name: "array_sum",
        description: "Somma elementi array (simulato)",
        pattern: "somma array|array sum|lista",
        code: r#"fn sum_list() -> Int {
    let n1 = 10
    let n2 = 20
    let n3 = 30
    let n4 = 40
    let n5 = 50
    return n1 + n2 + n3 + n4 + n5
}
print(sum_list())"#,
    },
    CodeTemplate {
        name: "countdown",
        description: "Conto alla rovescia",
        pattern: "countdown|conto rovescia|timer",
        code: r#"fn countdown(from) -> Int {
    let n = from
    while n > 0 {
        print(n)
        let n = n - 1
    }
    return 0
}
countdown(5)"#,
    },
    CodeTemplate {
        name: "hello_function",
        description: "Funzione saluto",
        pattern: "saluta|hello|greeting|ciao",
        code: r#"fn greet(name) -> String {
    return "Ciao " + name + "!"
}
print(greet("Mondo"))"#,
    },
];

/// Cerca un template che matcha la richiesta
pub fn find_template(request: &str) -> Option<&'static CodeTemplate> {
    let lower_request = request.to_lowercase();
    
    for template in TEMPLATES {
        let patterns: Vec<&str> = template.pattern.split('|').collect();
        for pattern in patterns {
            if lower_request.contains(pattern) {
                return Some(template);
            }
        }
    }
    
    None
}

/// Genera codice dal template
pub fn generate_from_template(request: &str) -> Result<(String, String), String> {
    match find_template(request) {
        Some(template) => {
            println!("   📋 Template trovato: {}", template.name);
            
            // Dividi in funzioni e statements
            let lines: Vec<&str> = template.code.lines().collect();
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
                    // Chiudi funzione SOLO se } è l'unico carattere (no indentazione)
                    if line == "}" {
                        in_function = false;
                        functions.push(current_function.join("\n"));
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
        None => {
            // Nessun template trovato - lascia che il sistema provi con GROK
            Err("Nessun template statico trovato".to_string())
        }
    }
}

/// Lista tutti i template disponibili
pub fn list_templates() -> Vec<(&'static str, &'static str)> {
    TEMPLATES.iter()
        .map(|t| (t.name, t.description))
        .collect()
}
