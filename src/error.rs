//! Error handling per Velora con pretty printing e suggestions

use colored::Colorize;

/// Errore di parsing con posizione e contesto
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub snippet: String,
    pub suggestion: Option<String>,
    pub filename: Option<String>,
}

impl ParseError {
    /// Crea un nuovo errore di parsing
    pub fn new(message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            message: message.into(),
            line,
            column,
            snippet: String::new(),
            suggestion: None,
            filename: None,
        }
    }

    /// Aggiunge lo snippet di codice
    pub fn with_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.snippet = snippet.into();
        self
    }

    /// Aggiunge un suggestion
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Aggiunge il nome del file
    pub fn with_filename(mut self, filename: impl Into<String>) -> Self {
        self.filename = Some(filename.into());
        self
    }

    /// Formatta l'errore in stile Rust (colorato)
    pub fn format_pretty(&self, source: &str) -> String {
        let mut output = String::new();

        // Header errore
        output.push_str(&format!("{}: {}", "error".red().bold(), self.message.bold()));
        output.push('\n');

        // Posizione
        let location = match &self.filename {
            Some(f) => format!("{}:{}:{}", f, self.line, self.column),
            None => format!("<input>:{}:{}", self.line, self.column),
        };
        output.push_str(&format!(" {} {}\n", "-->".blue().bold(), location));

        // Snippet
        if !self.snippet.is_empty() {
            output.push_str(&self.format_snippet());
        } else if !source.is_empty() {
            output.push_str(&self.format_snippet_from_source(source));
        }

        // Suggestion
        if let Some(sugg) = &self.suggestion {
            output.push_str(&format!("\n{}: {}\n", "help".green().bold(), sugg));
        }

        output
    }

    fn format_snippet(&self) -> String {
        let mut output = String::new();
        let line_num_str = self.line.to_string();
        let padding = " ".repeat(line_num_str.len());

        // Linea vuota con bordo
        output.push_str(&format!("{} {}\n", padding, "|".blue().bold()));

        // Numero linea e codice
        output.push_str(&format!("{} {} {}\n", 
            line_num_str.blue().bold(), 
            "|".blue().bold(),
            self.snippet
        ));

        // Freccia indicante l'errore
        let arrow = "^".repeat(1).red().bold();
        let spaces = " ".repeat(self.column.saturating_sub(1));
        output.push_str(&format!("{} {} {}{} {}\n",
            padding,
            "|".blue().bold(),
            spaces,
            arrow,
            self.message.red()
        ));

        output
    }

    fn format_snippet_from_source(&self, source: &str) -> String {
        let lines: Vec<&str> = source.lines().collect();
        if self.line == 0 || self.line > lines.len() {
            return String::new();
        }

        let line_content = lines[self.line - 1];
        self.clone()
            .with_snippet(line_content)
            .format_snippet()
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parse error at {}:{}: {}", self.line, self.column, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Errore runtime con stack trace
#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub message: String,
    pub stack: Vec<StackFrame>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StackFrame {
    pub function: String,
    pub line: Option<usize>,
}

impl RuntimeError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            stack: Vec::new(),
            suggestion: None,
        }
    }

    pub fn with_frame(mut self, function: impl Into<String>, line: Option<usize>) -> Self {
        self.stack.push(StackFrame {
            function: function.into(),
            line,
        });
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn format_pretty(&self) -> String {
        let mut output = String::new();

        // Header errore
        output.push_str(&format!("{}: {}", "runtime error".red().bold(), self.message.bold()));
        output.push('\n');

        // Stack trace
        if !self.stack.is_empty() {
            output.push_str("\n");
            output.push_str(&format!("{}\n", "Stack trace:".yellow().bold()));
            for (i, frame) in self.stack.iter().rev().enumerate() {
                let location = match frame.line {
                    Some(l) => format!(":{}", l),
                    None => String::new(),
                };
                output.push_str(&format!("  {} at {}{}\n", 
                    i.to_string().blue(),
                    frame.function.yellow(),
                    location
                ));
            }
        }

        // Suggestion
        if let Some(sugg) = &self.suggestion {
            output.push_str(&format!("\n{}: {}\n", "help".green().bold(), sugg));
        }

        output
    }
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Runtime error: {}", self.message)
    }
}

impl std::error::Error for RuntimeError {}

/// Calcola la distanza di Levenshtein tra due stringhe
/// Usata per suggerire correzioni typo
pub fn levenshtein_distance(a: &str, b: &str) -> usize {
    let len_a = a.chars().count();
    let len_b = b.chars().count();
    
    if len_a == 0 { return len_b; }
    if len_b == 0 { return len_a; }
    
    let mut matrix = vec![vec![0; len_b + 1]; len_a + 1];
    
    for i in 0..=len_a {
        matrix[i][0] = i;
    }
    for j in 0..=len_b {
        matrix[0][j] = j;
    }
    
    for (i, ca) in a.chars().enumerate() {
        for (j, cb) in b.chars().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            matrix[i + 1][j + 1] = [
                matrix[i][j + 1] + 1,      // deletion
                matrix[i + 1][j] + 1,      // insertion
                matrix[i][j] + cost,       // substitution
            ].into_iter().min().unwrap();
        }
    }
    
    matrix[len_a][len_b]
}

/// Trova il candidato più simile usando Levenshtein
pub fn find_similar<'a>(target: &str, candidates: &[&'a str], max_distance: usize) -> Option<&'a str> {
    candidates
        .iter()
        .map(|&c| (c, levenshtein_distance(target, c)))
        .filter(|(_, d)| *d <= max_distance)
        .min_by_key(|(_, d)| *d)
        .map(|(c, _)| c)
}

/// Suggerimenti per errori comuni
pub struct ErrorSuggester;

impl ErrorSuggester {
    /// Suggerisce una correzione per una funzione non trovata
    pub fn suggest_function(name: &str, available: &[&str]) -> Option<String> {
        if let Some(similar) = find_similar(name, available, 3) {
            return Some(format!("Forse intendevi '{}'?", similar));
        }
        
        // Pattern matching per errori comuni
        let suggestions: Vec<(&str, &str)> = vec![
            ("print", "Usa 'print(expr)' per stampare"),
            ("len", "Usa 'len(string)' o 'len(array)'"),
            ("sqrt", "Usa 'sqrt(number)' per radice quadrata"),
        ];
        
        for (pattern, help) in suggestions {
            if levenshtein_distance(name, pattern) <= 2 {
                return Some(help.to_string());
            }
        }
        
        None
    }

    /// Suggerisce correzione per variabile non trovata
    pub fn suggest_variable(name: &str, available: &[&str]) -> Option<String> {
        find_similar(name, available, 2)
            .map(|s| format!("Forse intendevi '{}'?", s))
    }

    /// Suggerimento per errore di parsing generico
    pub fn suggest_parse_fix(error: &str) -> Option<String> {
        if error.contains("expected") && error.contains("fn") {
            return Some("Ricorda: le funzioni devono avere il tipo di ritorno: fn nome() -> Tipo { ... }".to_string());
        }
        if error.contains("return") && error.contains("outside") {
            return Some("'return' può essere usato solo dentro una funzione".to_string());
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_levenshtein() {
        assert_eq!(levenshtein_distance("kitten", "sitting"), 3);
        assert_eq!(levenshtein_distance("", ""), 0);
        assert_eq!(levenshtein_distance("a", ""), 1);
        assert_eq!(levenshtein_distance("", "a"), 1);
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
    }

    #[test]
    fn test_find_similar() {
        let candidates = vec!["print", "len", "sqrt", "abs"];
        assert_eq!(find_similar("prnit", &candidates, 2), Some("print"));
        assert_eq!(find_similar("unknown", &candidates, 1), None);
    }
}
