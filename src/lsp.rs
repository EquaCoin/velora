//! Language Server Protocol (LSP) per Velora
//! Implementazione base per autocompletion e go-to-definition

use std::collections::HashMap;
use std::io::{self, BufRead, Read, Write};

/// Messaggio JSON-RPC
#[derive(Debug, Clone, serde::Deserialize)]
pub struct JsonRpcMessage {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: Option<String>,
    pub params: Option<serde_json::Value>,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<serde_json::Value>,
}

/// Server LSP semplice
pub struct LspServer {
    documents: HashMap<String, String>, // uri -> content
}

impl LspServer {
    pub fn new() -> Self {
        LspServer {
            documents: HashMap::new(),
        }
    }
    
    /// Avvia il server LSP (bloccante)
    pub fn run(&mut self) -> io::Result<()> {
        let stdin = io::stdin();
        let mut stdout = io::stdout();
        let mut stdin_lock = stdin.lock();
        
        loop {
            // Leggi Content-Length
            let mut header = String::new();
            let mut len: usize = 0;
            
            loop {
                header.clear();
                if stdin_lock.read_line(&mut header)? == 0 {
                    return Ok(()); // EOF
                }
                
                if header.starts_with("Content-Length: ") {
                    len = header[16..].trim().parse().unwrap_or(0);
                } else if header.trim().is_empty() {
                    break; // Fine headers
                }
            }
            
            if len == 0 {
                continue;
            }
            
            // Leggi il contenuto
            let mut content = vec![0u8; len];
            stdin_lock.read_exact(&mut content)?;
            
            let request = String::from_utf8_lossy(&content);
            
            // Processa la richiesta
            if let Some(response) = self.handle_request(&request) {
                self.send_response(&mut stdout, &response)?;
            }
        }
    }
    
    fn handle_request(&mut self, request: &str) -> Option<String> {
        let msg: JsonRpcMessage = match serde_json::from_str(request) {
            Ok(m) => m,
            Err(_) => return None,
        };
        
        let method = msg.method.as_deref()?;
        
        match method {
            "initialize" => Some(self.handle_initialize(msg.id)),
            "textDocument/didOpen" => {
                self.handle_did_open(msg.params);
                None
            }
            "textDocument/didChange" => {
                self.handle_did_change(msg.params);
                None
            }
            "textDocument/completion" => {
                Some(self.handle_completion(msg.id, msg.params))
            }
            "shutdown" => Some(self.handle_shutdown(msg.id)),
            "exit" => None,
            _ => None,
        }
    }
    
    fn handle_initialize(&self, id: Option<serde_json::Value>) -> String {
        let result = serde_json::json!({
            "capabilities": {
                "textDocumentSync": 1,
                "completionProvider": {
                    "triggerCharacters": [".", ":", "("],
                    "resolveProvider": false
                },
                "definitionProvider": true,
                "hoverProvider": true
            },
            "serverInfo": {
                "name": "velora-lsp",
                "version": "3.0.0"
            }
        });
        
        self.make_response(id, result)
    }
    
    fn handle_shutdown(&self, id: Option<serde_json::Value>) -> String {
        self.make_response(id, serde_json::Value::Null)
    }
    
    fn handle_did_open(&mut self, params: Option<serde_json::Value>) {
        if let Some(p) = params {
            if let (Some(uri), Some(text)) = (
                p["textDocument"]["uri"].as_str(),
                p["textDocument"]["text"].as_str()
            ) {
                self.documents.insert(uri.to_string(), text.to_string());
            }
        }
    }
    
    fn handle_did_change(&mut self, params: Option<serde_json::Value>) {
        if let Some(p) = params {
            if let (Some(uri), Some(changes)) = (
                p["textDocument"]["uri"].as_str(),
                p["contentChanges"].as_array()
            ) {
                // Semplificazione: usa l'ultima versione completa
                if let Some(last) = changes.last() {
                    if let Some(text) = last["text"].as_str() {
                        self.documents.insert(uri.to_string(), text.to_string());
                    }
                }
            }
        }
    }
    
    fn handle_completion(&self, id: Option<serde_json::Value>, params: Option<serde_json::Value>) -> String {
        let mut items = Vec::new();
        
        // Keyword
        let keywords = vec![
            "fn", "let", "if", "else", "while", "return", 
            "main", "import", "test", "true", "false",
            "Int", "Float", "String", "Bool", "Void", "Array", "Map", "Option", "Result"
        ];
        
        for kw in keywords {
            items.push(serde_json::json!({
                "label": kw,
                "kind": 14, // Keyword
                "detail": format!("Keyword: {}", kw)
            }));
        }
        
        // Funzioni built-in
        let builtins = vec![
            ("print", "print(expr) -> Void", "Stampa un valore"),
            ("fibonacci", "fibonacci(n: Int) -> Int", "Calcola fibonacci"),
            ("sqrt", "sqrt(x: Float) -> Float", "Radice quadrata"),
            ("abs", "abs(x: Int|Float) -> Int|Float", "Valore assoluto"),
            ("pow", "pow(base: Float, exp: Float) -> Float", "Potenza"),
            ("len", "len(x: Array|String) -> Int", "Lunghezza"),
            ("push", "push(arr: Array, item: T) -> Array", "Aggiunge elemento"),
            ("contains", "contains(x: Array|String, y: T) -> Bool", "Contiene elemento"),
            ("split", "split(s: String, delim: String) -> Array", "Divide stringa"),
            ("trim", "trim(s: String) -> String", "Rimuove spazi"),
            ("read_file", "read_file(path: String) -> String", "Legge file"),
            ("write_file", "write_file(path: String, content: String) -> Bool", "Scrive file"),
            ("map_get", "map_get(m: Map, k: String) -> Option", "Ottiene da mappa"),
            ("map_set", "map_set(m: Map, k: String, v: T) -> Map", "Imposta in mappa"),
            ("Some", "Some(value: T) -> Option", "Crea Some"),
            ("None", "None() -> Option", "Crea None"),
            ("Ok", "Ok(value: T) -> Result", "Crea Ok"),
            ("Err", "Err(error: String) -> Result", "Crea Err"),
            ("unwrap", "unwrap(opt: Option|Result) -> T", "Estrae valore"),
            ("assert_eq", "assert_eq(a: T, b: T) -> Bool", "Assert equality"),
        ];
        
        for (name, detail, doc) in builtins {
            items.push(serde_json::json!({
                "label": name,
                "kind": 3, // Function
                "detail": detail,
                "documentation": doc
            }));
        }
        
        // Se c'è un documento aperto, aggiungi le funzioni definite
        if let Some(p) = params {
            if let Some(uri) = p["textDocument"]["uri"].as_str() {
                if let Some(content) = self.documents.get(uri) {
                    // Estrai funzioni definite (semplificato)
                    for line in content.lines() {
                        if line.trim().starts_with("fn ") {
                            let name = line[3..].split('(').next().unwrap_or("").trim();
                            if !name.is_empty() {
                                items.push(serde_json::json!({
                                    "label": name,
                                    "kind": 3,
                                    "detail": format!("fn {}", name)
                                }));
                            }
                        }
                    }
                }
            }
        }
        
        let result = serde_json::json!({ "items": items });
        self.make_response(id, result)
    }
    
    fn make_response(&self, id: Option<serde_json::Value>, result: serde_json::Value) -> String {
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        });
        
        response.to_string()
    }
    
    fn send_response(&self, stdout: &mut dyn Write, content: &str) -> io::Result<()> {
        let len = content.len();
        write!(stdout, "Content-Length: {}\r\n\r\n{}", len, content)?;
        stdout.flush()
    }
}

/// Avvia il server LSP
pub fn start_lsp_server() -> io::Result<()> {
    eprintln!("Velora LSP server starting...");
    let mut server = LspServer::new();
    server.run()
}
