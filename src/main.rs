use std::env;
use std::fs;
use std::io::{self, Write};
use colored::Colorize;

mod ast;
mod parser;
mod interpreter;
mod ai;
mod error;
mod package;
mod lsp;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        run_repl();
        return;
    }

    if args[1] == "repl" {
        run_repl();
        return;
    }

    if args[1] == "test" {
        let filename = if args.len() > 2 { &args[2] } else { 
            eprintln!("Usage: velora test <file.vel>");
            std::process::exit(1);
        };
        run_tests(filename);
        return;
    }

    if args[1] == "init" {
        let name = if args.len() > 2 { args[2].clone() } else {
            std::env::current_dir()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                .unwrap_or_else(|| "my_project".to_string())
        };
        let pm = package::PackageManager::new(std::env::current_dir().unwrap());
        pm.init(&name).unwrap_or_else(|e| {
            eprintln!("{} {}", "❌ Error:".red(), e);
            std::process::exit(1);
        });
        return;
    }

    if args[1] == "install" || args[1] == "add" {
        let pm = package::PackageManager::new(std::env::current_dir().unwrap());
        if args[1] == "install" {
            pm.install().unwrap_or_else(|e| {
                eprintln!("{} {}", "❌ Error:".red(), e);
                std::process::exit(1);
            });
        } else if args[1] == "add" {
            if args.len() < 4 {
                eprintln!("Usage: velora add <name> <source>");
                std::process::exit(1);
            }
            pm.add_dependency(&args[2], &args[3]).unwrap_or_else(|e| {
                eprintln!("{} {}", "❌ Error:".red(), e);
                std::process::exit(1);
            });
        }
        return;
    }

    if args[1] == "lsp" {
        lsp::start_lsp_server().unwrap_or_else(|e| {
            eprintln!("{} LSP error: {}", "❌".red(), e);
            std::process::exit(1);
        });
        return;
    }

    let filename = if args.len() > 2 && args[1] == "run" {
        &args[2]
    } else {
        &args[1]
    };

    run_file(filename);
}

fn run_tests(filename: &str) {
    println!("🧪 Velora v3 - Running tests...");
    println!("📄 File: {}", filename);
    println!();

    let content = match fs::read_to_string(filename) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{} {}", "❌ Error reading file:".red(), e);
            std::process::exit(1);
        }
    };

    let program = match parser::parse(&content) {
        Ok(p) => p,
        Err(e) => {
            let error_msg = e.format_pretty(&content);
            eprintln!("\n{}", error_msg);
            std::process::exit(1);
        }
    };

    let mut interpreter = interpreter::Interpreter::new();
    
    match interpreter.run_tests(&program) {
        Ok((passed, failed)) => {
            println!();
            println!("Test Results: {} passed, {} failed", passed, failed);
            if failed > 0 {
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("\n{}", e.format_pretty());
            std::process::exit(1);
        }
    }
}

fn run_file(filename: &str) {
    println!("🔨 Velora v3 - Esecuzione in corso...");
    println!("📄 File: {}", filename);
    println!();

    // Leggi il file
    let content = match fs::read_to_string(filename) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{} {}", "❌ Errore lettura file:".red(), e);
            std::process::exit(1);
        }
    };

    // Parsing
    println!("📝 Parsing...");
    let program = match parser::parse(&content) {
        Ok(p) => {
            println!("✅ Parsing completato!");
            p
        }
        Err(e) => {
            let error_msg = e.format_pretty(&content);
            eprintln!("\n{}", error_msg);
            std::process::exit(1);
        }
    };

    // Debug: mostra AST
    if env::var("DEBUG_AST").is_ok() {
        println!("\n🔍 AST:");
        println!("{:#?}", program);
        println!();
    }

    // Esecuzione
    println!("🚀 Esecuzione...\n");
    match interpreter::run_program(&program) {
        Ok(output) => {
            for line in output {
                println!("{}", line);
            }
            println!("\n✅ Esecuzione completata!");
        }
        Err(e) => {
            let error_msg = e.format_pretty();
            eprintln!("\n{}", error_msg);
            std::process::exit(1);
        }
    }
}

fn run_repl() {
    println!("🚀 Velora v3 - REPL");
    println!("Digita 'exit' o 'quit' per uscire");
    println!("Digita 'help' per l'aiuto");
    println!("Per funzioni multi-linea, usa 'fn nome() {{ ... }}' su una riga\n");

    let stdin = io::stdin();
    let mut interpreter = interpreter::Interpreter::new();

    loop {
        print!("velora> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        match stdin.read_line(&mut input) {
            Ok(_) => {
                let input = input.trim();
                
                if input.is_empty() {
                    continue;
                }

                match input {
                    "exit" | "quit" => {
                        println!("👋 Ciao!");
                        break;
                    }
                    "help" => {
                        print_help();
                        continue;
                    }
                    "clear" => {
                        print!("\x1B[2J\x1B[1;1H");
                        continue;
                    }
                    _ => {}
                }

                // Per il REPL, wrappa l'input in un main block se non è già una funzione
                let code = if input.starts_with("fn ") {
                    input.to_string()
                } else if input.starts_with("main:") {
                    input.to_string()
                } else {
                    format!("main:\n    {}", input)
                };

                match parser::parse(&code) {
                    Ok(program) => {
                        // Aggiungi le funzioni definite all'interprete
                        for func in &program.functions {
                            interpreter.add_function(
                                func.name.clone(),
                                func.params.clone(),
                                func.body.clone(),
                            );
                            println!("✅ Funzione '{}' registrata", func.name);
                        }
                        
                        // Esegui il main
                        for stmt in &program.main {
                            match interpreter.execute_single(stmt) {
                                Ok(_) => {
                                    // Stampa l'output accumulato
                                    let output = interpreter.take_output();
                                    for line in output {
                                        println!("{}", line);
                                    }
                                }
                                Err(e) => {
                                    let error_msg = e.format_pretty();
                                    eprintln!("\n{}", error_msg);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let error_msg = e.format_pretty(&code);
                        eprintln!("\n{}", error_msg);
                    }
                }
            }
            Err(e) => {
                eprintln!("❌ Errore di lettura: {}", e);
                break;
            }
        }
    }
}

fn print_help() {
    println!("Comandi CLI:");
    println!("  velora <file.vel>     - Esegue un file");
    println!("  velora repl           - Avvia la REPL");
    println!("  velora test <file>    - Esegue i test");
    println!("  velora init [name]    - Inizializza un nuovo progetto");
    println!("  velora install        - Installa le dipendenze");
    println!("  velora add <name> <source> - Aggiunge una dipendenza");
    println!("  velora lsp            - Avvia il Language Server");
    println!();
    println!("Comandi REPL:");
    println!("  help     - Mostra questo aiuto");
    println!("  clear    - Pulisce lo schermo");
    println!("  exit     - Esce dal REPL");
    println!();
    println!("Esempi di codice:");
    println!("  let x = 10");
    println!("  print(x + 5)");
    println!("  if x > 5 {{ print(\"grande\") }} else {{ print(\"piccolo\") }}");
    println!("  while x > 0 {{ print(x); let x = x - 1 }}");
    println!("  fn double(x) -> Int {{ return x * 2 }}");
    println!("  print(double(21))");
    println!();
    println!("Funzioni built-in:");
    println!("  Numeriche: fibonacci(n), abs(n), sqrt(n), pow(b, e), len(x)");
    println!("  Arrotondamento: floor(n), ceil(n), round(n)");
    println!("  Array/String: push(arr, item), contains(x, y), split(s, d)");
    println!("  Stringhe: trim(s), starts_with(s, p), ends_with(s, s)");
    println!("            to_upper(s), to_lower(s), substring(s, a, b), replace(s, f, t)");
    println!("  File: read_file(path), write_file(path, content), append_file(path, content)");
    println!("        file_exists(path)");
}
