use std::collections::HashMap;

use crate::ast::*;
use crate::error::{RuntimeError, ErrorSuggester};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Array(Vec<Value>),
    Map(std::collections::HashMap<String, Value>),
    Option(Option<Box<Value>>),
    Result(Result<Box<Value>, Box<Value>>),
    Void,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Array(arr) => {
                let elements: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                write!(f, "[{}]", elements.join(", "))
            }
            Value::Map(map) => {
                let entries: Vec<String> = map.iter()
                    .map(|(k, v)| format!("{}: {}", k, v))
                    .collect();
                write!(f, "{{{}}}", entries.join(", "))
            }
            Value::Option(opt) => match opt {
                Some(v) => write!(f, "Some({})", v),
                None => write!(f, "None"),
            },
            Value::Result(res) => match res {
                Ok(v) => write!(f, "Ok({})", v),
                Err(e) => write!(f, "Err({})", e),
            },
            Value::Void => write!(f, "void"),
        }
    }
}

// Rappresenta una funzione utente definita nel codice
#[derive(Clone)]
struct UserFunction {
    params: Vec<(String, Option<String>)>,  // (name, optional_type)
    body: Vec<Stmt>,
}

pub struct Interpreter {
    // Stack di scope per variabili (ogni scope è un hashmap)
    scopes: Vec<HashMap<String, Value>>,
    // Funzioni built-in e utente
    functions: HashMap<String, UserFunction>,
    output: Vec<String>,
    // Per gestire il return dalle funzioni
    return_value: Option<Value>,
    // Stack trace per errori
    call_stack: Vec<String>,
    // Moduli già importati (per evitare cicli)
    imported_modules: HashMap<String, Vec<String>>, // nome modulo -> funzioni esportate
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Interpreter {
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
            output: Vec::new(),
            return_value: None,
            call_stack: Vec::new(),
            imported_modules: HashMap::new(),
        };
        
        // Inizializza scope globale
        interpreter.scopes.push(HashMap::new());
        
        interpreter
    }

    pub fn run(&mut self, program: &Program) -> Result<Vec<String>, RuntimeError> {
        // Importa i moduli prima di tutto
        for (path, alias) in &program.imports {
            self.import_module(path, alias.as_deref())?;
        }
        
        // Registra tutte le funzioni utente
        for func in &program.functions {
            self.functions.insert(
                func.name.clone(),
                UserFunction {
                    params: func.params.clone(),
                    body: func.body.clone(),
                },
            );
        }
        
        // Esegui main
        for stmt in &program.main {
            self.execute_stmt(stmt)?;
        }
        
        Ok(self.output.clone())
    }

    // Metodi pubblici per REPL
    pub fn add_function(&mut self, name: String, params: Vec<(String, Option<String>)>, body: Vec<Stmt>) {
        self.functions.insert(
            name,
            UserFunction { params, body },
        );
    }

    pub fn execute_single(&mut self, stmt: &Stmt) -> Result<Value, RuntimeError> {
        self.execute_stmt(stmt)
    }

    pub fn take_output(&mut self) -> Vec<String> {
        std::mem::take(&mut self.output)
    }

    fn current_scope(&mut self) -> &mut HashMap<String, Value> {
        self.scopes.last_mut().unwrap()
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn get_variable(&self, name: &str) -> Option<Value> {
        // Cerca dallo scope più interno verso l'esterno
        for scope in self.scopes.iter().rev() {
            if let Some(val) = scope.get(name) {
                return Some(val.clone());
            }
        }
        None
    }

    fn set_variable(&mut self, name: &str, value: Value) {
        self.current_scope().insert(name.to_string(), value);
    }

    fn execute_stmt(&mut self, stmt: &Stmt) -> Result<Value, RuntimeError> {
        // Se c'è già un return value, non eseguire altri statement
        if self.return_value.is_some() {
            return Ok(Value::Void);
        }

        match stmt {
            Stmt::Print(expr) => {
                let value = self.evaluate_expr(expr)?;
                self.output.push(value.to_string());
                Ok(Value::Void)
            }
            Stmt::Let { name, value } => {
                let val = self.evaluate_expr(value)?;
                self.set_variable(name, val);
                Ok(Value::Void)
            }
            Stmt::Assign { name, value } => {
                if self.get_variable(name).is_none() {
                    let available: Vec<&str> = self.scopes.iter()
                        .flat_map(|s| s.keys().map(|k| k.as_str()))
                        .collect();
                    let mut err = RuntimeError::new(format!("Variable '{}' not defined", name))
                        .with_frame("main", None);
                    if let Some(sugg) = ErrorSuggester::suggest_variable(name, &available) {
                        err = err.with_suggestion(sugg);
                    }
                    return Err(err);
                }
                let val = self.evaluate_expr(value)?;
                self.set_variable(name, val);
                Ok(Value::Void)
            }
            Stmt::Return(expr) => {
                let val = match expr {
                    Some(e) => self.evaluate_expr(e)?,
                    None => Value::Void,
                };
                self.return_value = Some(val);
                Ok(Value::Void)
            }
            Stmt::Expr(expr) => {
                self.evaluate_expr(expr)?;
                Ok(Value::Void)
            }
            Stmt::If { condition, then_branch, else_branch } => {
                let cond_val = self.evaluate_expr(condition)?;
                match cond_val {
                    Value::Bool(true) => {
                        self.push_scope();
                        for stmt in then_branch {
                            self.execute_stmt(stmt)?;
                            if self.return_value.is_some() {
                                break;
                            }
                        }
                        self.pop_scope();
                    }
                    Value::Bool(false) => {
                        if let Some(else_stmts) = else_branch {
                            self.push_scope();
                            for stmt in else_stmts {
                                self.execute_stmt(stmt)?;
                                if self.return_value.is_some() {
                                    break;
                                }
                            }
                            self.pop_scope();
                        }
                    }
                    _ => return Err(RuntimeError::new("If condition must be boolean")
                        .with_frame("main", None)),
                }
                Ok(Value::Void)
            }
            Stmt::While { condition, body } => {
                loop {
                    let cond_val = self.evaluate_expr(condition)?;
                    match cond_val {
                        Value::Bool(true) => {
                            self.push_scope();
                            for stmt in body {
                                self.execute_stmt(stmt)?;
                                if self.return_value.is_some() {
                                    self.pop_scope();
                                    return Ok(Value::Void);
                                }
                            }
                            self.pop_scope();
                        }
                        Value::Bool(false) => break,
                        _ => return Err(RuntimeError::new("While condition must be boolean")
                            .with_frame("main", None)),
                    }
                }
                Ok(Value::Void)
            }
            Stmt::Block(stmts) => {
                self.push_scope();
                for stmt in stmts {
                    self.execute_stmt(stmt)?;
                    if self.return_value.is_some() {
                        break;
                    }
                }
                self.pop_scope();
                Ok(Value::Void)
            }
        }
    }

    fn evaluate_expr(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Number(n) => Ok(Value::Number(*n)),
            Expr::Float(n) => Ok(Value::Float(*n)),
            Expr::String(s) => Ok(Value::String(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Array(elements) => {
                let values: Result<Vec<Value>, RuntimeError> = elements
                    .iter()
                    .map(|e| self.evaluate_expr(e))
                    .collect();
                Ok(Value::Array(values?))
            }
            Expr::Map(entries) => {
                let mut map = std::collections::HashMap::new();
                for (key, expr) in entries {
                    let value = self.evaluate_expr(expr)?;
                    map.insert(key.clone(), value);
                }
                Ok(Value::Map(map))
            }
            Expr::Index { name, index } => {
                let arr = self.get_variable(name)
                    .ok_or_else(|| {
                        RuntimeError::new(format!("Variable '{}' not found", name))
                            .with_frame("main", None)
                    })?;
                let idx_val = self.evaluate_expr(index)?;
                let idx = match idx_val {
                    Value::Number(n) => n as usize,
                    _ => return Err(RuntimeError::new("Array index must be an integer")
                        .with_frame("main", None)),
                };
                match arr {
                    Value::Array(arr) => {
                        if idx >= arr.len() {
                            return Err(RuntimeError::new(format!("Index {} out of bounds (array length: {})", idx, arr.len()))
                                .with_frame("main", None));
                        }
                        Ok(arr[idx].clone())
                    }
                    Value::String(s) => {
                        if idx >= s.len() {
                            return Err(RuntimeError::new(format!("Index {} out of bounds (string length: {})", idx, s.len()))
                                .with_frame("main", None));
                        }
                        Ok(Value::String(s.chars().nth(idx).unwrap().to_string()))
                    }
                    _ => Err(RuntimeError::new(format!("'{}' is not indexable", name))
                        .with_frame("main", None)),
                }
            }
            Expr::Variable(name) => {
                self.get_variable(name)
                    .ok_or_else(|| {
                        let available: Vec<&str> = self.scopes.iter()
                            .flat_map(|s| s.keys().map(|k| k.as_str()))
                            .collect();
                        let mut err = RuntimeError::new(format!("Variable '{}' not found", name))
                            .with_frame("main", None);
                        if let Some(sugg) = ErrorSuggester::suggest_variable(name, &available) {
                            err = err.with_suggestion(sugg);
                        }
                        err
                    })
            }
            Expr::Binary { left, op, right } => {
                let left_val = self.evaluate_expr(left)?;
                let right_val = self.evaluate_expr(right)?;
                self.eval_binary_op(left_val, *op, right_val)
            }
            Expr::Unary { op, expr } => {
                let val = self.evaluate_expr(expr)?;
                self.eval_unary_op(*op, val)
            }
            Expr::Call { name, args } => {
                let arg_values: Result<Vec<Value>, RuntimeError> = args
                    .iter()
                    .map(|a| self.evaluate_expr(a))
                    .collect();
                self.call_function(name, arg_values?)
            }
        }
    }

    fn eval_binary_op(&self, left: Value, op: BinaryOp, right: Value) -> Result<Value, RuntimeError> {
        match (left, op, right) {
            // Aritmetica Int
            (Value::Number(l), BinaryOp::Add, Value::Number(r)) => Ok(Value::Number(l + r)),
            (Value::Number(l), BinaryOp::Sub, Value::Number(r)) => Ok(Value::Number(l - r)),
            (Value::Number(l), BinaryOp::Mul, Value::Number(r)) => Ok(Value::Number(l * r)),
            (Value::Number(l), BinaryOp::Div, Value::Number(r)) => {
                if r == 0 {
                    Err(RuntimeError::new("Division by zero").with_frame("main", None))
                } else {
                    Ok(Value::Number(l / r))
                }
            }
            (Value::Number(l), BinaryOp::Mod, Value::Number(r)) => {
                if r == 0 {
                    Err(RuntimeError::new("Modulo by zero").with_frame("main", None))
                } else {
                    Ok(Value::Number(l % r))
                }
            }
            
            // Aritmetica Float (e mixed Int/Float)
            (Value::Float(l), BinaryOp::Add, Value::Float(r)) => Ok(Value::Float(l + r)),
            (Value::Float(l), BinaryOp::Sub, Value::Float(r)) => Ok(Value::Float(l - r)),
            (Value::Float(l), BinaryOp::Mul, Value::Float(r)) => Ok(Value::Float(l * r)),
            (Value::Float(l), BinaryOp::Div, Value::Float(r)) => Ok(Value::Float(l / r)),
            (Value::Float(l), BinaryOp::Mod, Value::Float(r)) => Ok(Value::Float(l % r)),
            
            // Mixed Int/Float -> promuovi a Float
            (Value::Number(l), BinaryOp::Add, Value::Float(r)) => Ok(Value::Float(l as f64 + r)),
            (Value::Float(l), BinaryOp::Add, Value::Number(r)) => Ok(Value::Float(l + r as f64)),
            (Value::Number(l), BinaryOp::Sub, Value::Float(r)) => Ok(Value::Float(l as f64 - r)),
            (Value::Float(l), BinaryOp::Sub, Value::Number(r)) => Ok(Value::Float(l - r as f64)),
            (Value::Number(l), BinaryOp::Mul, Value::Float(r)) => Ok(Value::Float(l as f64 * r)),
            (Value::Float(l), BinaryOp::Mul, Value::Number(r)) => Ok(Value::Float(l * r as f64)),
            (Value::Number(l), BinaryOp::Div, Value::Float(r)) => Ok(Value::Float(l as f64 / r)),
            (Value::Float(l), BinaryOp::Div, Value::Number(r)) => {
                if r == 0 {
                    Err(RuntimeError::new("Division by zero").with_frame("main", None))
                } else {
                    Ok(Value::Float(l / r as f64))
                }
            }
            
            // Confronti Int
            (Value::Number(l), BinaryOp::Eq, Value::Number(r)) => Ok(Value::Bool(l == r)),
            (Value::Number(l), BinaryOp::Neq, Value::Number(r)) => Ok(Value::Bool(l != r)),
            (Value::Number(l), BinaryOp::Lt, Value::Number(r)) => Ok(Value::Bool(l < r)),
            (Value::Number(l), BinaryOp::Gt, Value::Number(r)) => Ok(Value::Bool(l > r)),
            (Value::Number(l), BinaryOp::Lte, Value::Number(r)) => Ok(Value::Bool(l <= r)),
            (Value::Number(l), BinaryOp::Gte, Value::Number(r)) => Ok(Value::Bool(l >= r)),
            
            // Confronti Float
            (Value::Float(l), BinaryOp::Eq, Value::Float(r)) => Ok(Value::Bool(l == r)),
            (Value::Float(l), BinaryOp::Neq, Value::Float(r)) => Ok(Value::Bool(l != r)),
            (Value::Float(l), BinaryOp::Lt, Value::Float(r)) => Ok(Value::Bool(l < r)),
            (Value::Float(l), BinaryOp::Gt, Value::Float(r)) => Ok(Value::Bool(l > r)),
            (Value::Float(l), BinaryOp::Lte, Value::Float(r)) => Ok(Value::Bool(l <= r)),
            (Value::Float(l), BinaryOp::Gte, Value::Float(r)) => Ok(Value::Bool(l >= r)),
            
            // Mixed Int/Float confronti
            (Value::Number(l), BinaryOp::Eq, Value::Float(r)) => Ok(Value::Bool(l as f64 == r)),
            (Value::Float(l), BinaryOp::Eq, Value::Number(r)) => Ok(Value::Bool(l == r as f64)),
            (Value::Number(l), BinaryOp::Lt, Value::Float(r)) => Ok(Value::Bool((l as f64) < r)),
            (Value::Float(l), BinaryOp::Lt, Value::Number(r)) => Ok(Value::Bool(l < r as f64)),
            
            // Stringhe
            (Value::String(l), BinaryOp::Add, Value::String(r)) => Ok(Value::String(l + &r)),
            (Value::String(l), BinaryOp::Eq, Value::String(r)) => Ok(Value::Bool(l == r)),
            (Value::String(l), BinaryOp::Neq, Value::String(r)) => Ok(Value::Bool(l != r)),
            
            // Bool
            (Value::Bool(l), BinaryOp::And, Value::Bool(r)) => Ok(Value::Bool(l && r)),
            (Value::Bool(l), BinaryOp::Or, Value::Bool(r)) => Ok(Value::Bool(l || r)),
            (Value::Bool(l), BinaryOp::Eq, Value::Bool(r)) => Ok(Value::Bool(l == r)),
            (Value::Bool(l), BinaryOp::Neq, Value::Bool(r)) => Ok(Value::Bool(l != r)),
            
            _ => Err(RuntimeError::new("Type mismatch in binary operation")
                .with_frame("main", None)
                .with_suggestion("Controlla che entrambi gli operandi siano dello stesso tipo")),
        }
    }

    fn eval_unary_op(&self, op: UnaryOp, val: Value) -> Result<Value, RuntimeError> {
        match (op, val) {
            (UnaryOp::Neg, Value::Number(n)) => Ok(Value::Number(-n)),
            (UnaryOp::Neg, Value::Float(n)) => Ok(Value::Float(-n)),
            (UnaryOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
            _ => Err(RuntimeError::new("Type mismatch in unary operation")
                .with_frame("main", None)),
        }
    }

    fn call_function(&mut self, name: &str, args: Vec<Value>) -> Result<Value, RuntimeError> {
        // Prima cerca funzioni built-in
        match name {
            "fibonacci" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("fibonacci expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                match &args[0] {
                    Value::Number(n) => {
                        let result = Self::fibonacci(*n);
                        Ok(Value::Number(result))
                    }
                    Value::Float(n) => {
                        let result = Self::fibonacci(*n as i64);
                        Ok(Value::Number(result))
                    }
                    _ => Err(RuntimeError::new("fibonacci expects a number")
                        .with_frame("main", None)),
                }
            }
            "sqrt" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("sqrt expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                match &args[0] {
                    Value::Number(n) => Ok(Value::Float((*n as f64).sqrt())),
                    Value::Float(n) => Ok(Value::Float(n.sqrt())),
                    _ => Err(RuntimeError::new("sqrt expects a number")
                        .with_frame("main", None)),
                }
            }
            "abs" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("abs expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                match &args[0] {
                    Value::Number(n) => Ok(Value::Number(n.abs())),
                    Value::Float(n) => Ok(Value::Float(n.abs())),
                    _ => Err(RuntimeError::new("abs expects a number")
                        .with_frame("main", None)),
                }
            }
            "pow" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(format!("pow expects 2 arguments, got {}", args.len()))
                        .with_frame("main", None));
                }
                let base = match &args[0] {
                    Value::Number(n) => *n as f64,
                    Value::Float(n) => *n,
                    _ => return Err(RuntimeError::new("pow expects numbers")
                        .with_frame("main", None)),
                };
                let exp = match &args[1] {
                    Value::Number(n) => *n as f64,
                    Value::Float(n) => *n,
                    _ => return Err(RuntimeError::new("pow expects numbers")
                        .with_frame("main", None)),
                };
                Ok(Value::Float(base.powf(exp)))
            }
            "floor" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("floor expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                match &args[0] {
                    Value::Number(n) => Ok(Value::Number(*n)),
                    Value::Float(n) => Ok(Value::Number(n.floor() as i64)),
                    _ => Err(RuntimeError::new("floor expects a number")
                        .with_frame("main", None)),
                }
            }
            "ceil" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("ceil expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                match &args[0] {
                    Value::Number(n) => Ok(Value::Number(*n)),
                    Value::Float(n) => Ok(Value::Number(n.ceil() as i64)),
                    _ => Err(RuntimeError::new("ceil expects a number")
                        .with_frame("main", None)),
                }
            }
            "round" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("round expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                match &args[0] {
                    Value::Number(n) => Ok(Value::Number(*n)),
                    Value::Float(n) => Ok(Value::Number(n.round() as i64)),
                    _ => Err(RuntimeError::new("round expects a number")
                        .with_frame("main", None)),
                }
            }
            "len" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("len expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                match &args[0] {
                    Value::String(s) => Ok(Value::Number(s.len() as i64)),
                    Value::Array(arr) => Ok(Value::Number(arr.len() as i64)),
                    _ => Err(RuntimeError::new("len expects a string or array")
                        .with_frame("main", None)
                        .with_suggestion("Esempi: len(\"hello\"), len([1, 2, 3])")),
                }
            }
            "push" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(format!("push expects 2 arguments, got {}", args.len()))
                        .with_frame("main", None));
                }
                match &args[0] {
                    Value::Array(arr) => {
                        let mut new_arr = arr.clone();
                        new_arr.push(args[1].clone());
                        Ok(Value::Array(new_arr))
                    }
                    _ => Err(RuntimeError::new("push expects an array as first argument")
                        .with_frame("main", None)),
                }
            }
            "contains" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(format!("contains expects 2 arguments, got {}", args.len()))
                        .with_frame("main", None));
                }
                match (&args[0], &args[1]) {
                    (Value::Array(arr), item) => {
                        Ok(Value::Bool(arr.contains(item)))
                    }
                    (Value::String(s), Value::String(sub)) => {
                        Ok(Value::Bool(s.contains(sub)))
                    }
                    _ => Err(RuntimeError::new("contains expects array+item or string+substring")
                        .with_frame("main", None)),
                }
            }
            "split" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(format!("split expects 2 arguments, got {}", args.len()))
                        .with_frame("main", None));
                }
                match (&args[0], &args[1]) {
                    (Value::String(s), Value::String(delim)) => {
                        let parts: Vec<Value> = s.split(delim)
                            .map(|p| Value::String(p.to_string()))
                            .collect();
                        Ok(Value::Array(parts))
                    }
                    _ => Err(RuntimeError::new("split expects string and delimiter")
                        .with_frame("main", None)),
                }
            }
            "trim" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("trim expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                match &args[0] {
                    Value::String(s) => Ok(Value::String(s.trim().to_string())),
                    _ => Err(RuntimeError::new("trim expects a string")
                        .with_frame("main", None)),
                }
            }
            "starts_with" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(format!("starts_with expects 2 arguments, got {}", args.len()))
                        .with_frame("main", None));
                }
                match (&args[0], &args[1]) {
                    (Value::String(s), Value::String(prefix)) => Ok(Value::Bool(s.starts_with(prefix))),
                    _ => Err(RuntimeError::new("starts_with expects two strings")
                        .with_frame("main", None)),
                }
            }
            "ends_with" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(format!("ends_with expects 2 arguments, got {}", args.len()))
                        .with_frame("main", None));
                }
                match (&args[0], &args[1]) {
                    (Value::String(s), Value::String(suffix)) => Ok(Value::Bool(s.ends_with(suffix))),
                    _ => Err(RuntimeError::new("ends_with expects two strings")
                        .with_frame("main", None)),
                }
            }
            "to_upper" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("to_upper expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                match &args[0] {
                    Value::String(s) => Ok(Value::String(s.to_uppercase())),
                    _ => Err(RuntimeError::new("to_upper expects a string")
                        .with_frame("main", None)),
                }
            }
            "to_lower" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("to_lower expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                match &args[0] {
                    Value::String(s) => Ok(Value::String(s.to_lowercase())),
                    _ => Err(RuntimeError::new("to_lower expects a string")
                        .with_frame("main", None)),
                }
            }
            "substring" => {
                if args.len() != 3 {
                    return Err(RuntimeError::new(format!("substring expects 3 arguments, got {}", args.len()))
                        .with_frame("main", None));
                }
                match (&args[0], &args[1], &args[2]) {
                    (Value::String(s), Value::Number(start), Value::Number(end)) => {
                        let start = *start as usize;
                        let end = *end as usize;
                        if start > s.len() || end > s.len() || start > end {
                            return Err(RuntimeError::new("Invalid substring indices")
                                .with_frame("main", None));
                        }
                        Ok(Value::String(s[start..end].to_string()))
                    }
                    _ => Err(RuntimeError::new("substring expects string, start, end")
                        .with_frame("main", None)),
                }
            }
            "replace" => {
                if args.len() != 3 {
                    return Err(RuntimeError::new(format!("replace expects 3 arguments, got {}", args.len()))
                        .with_frame("main", None));
                }
                match (&args[0], &args[1], &args[2]) {
                    (Value::String(s), Value::String(from), Value::String(to)) => {
                        Ok(Value::String(s.replace(from, to)))
                    }
                    _ => Err(RuntimeError::new("replace expects string, from, to")
                        .with_frame("main", None)),
                }
            }
            // I/O File
            "read_file" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("read_file expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                match &args[0] {
                    Value::String(path) => {
                        match std::fs::read_to_string(path) {
                            Ok(content) => Ok(Value::String(content)),
                            Err(e) => Err(RuntimeError::new(format!("Failed to read file '{}': {}", path, e))
                                .with_frame("main", None)),
                        }
                    }
                    _ => Err(RuntimeError::new("read_file expects a file path (string)")
                        .with_frame("main", None)),
                }
            }
            "write_file" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(format!("write_file expects 2 arguments, got {}", args.len()))
                        .with_frame("main", None));
                }
                match (&args[0], &args[1]) {
                    (Value::String(path), Value::String(content)) => {
                        match std::fs::write(path, content) {
                            Ok(_) => Ok(Value::Bool(true)),
                            Err(e) => Err(RuntimeError::new(format!("Failed to write file '{}': {}", path, e))
                                .with_frame("main", None)),
                        }
                    }
                    _ => Err(RuntimeError::new("write_file expects path (string) and content (string)")
                        .with_frame("main", None)),
                }
            }
            "append_file" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(format!("append_file expects 2 arguments, got {}", args.len()))
                        .with_frame("main", None));
                }
                match (&args[0], &args[1]) {
                    (Value::String(path), Value::String(content)) => {
                        use std::io::Write;
                        match std::fs::OpenOptions::new().create(true).append(true).open(path) {
                            Ok(mut file) => {
                                match file.write_all(content.as_bytes()) {
                                    Ok(_) => Ok(Value::Bool(true)),
                                    Err(e) => Err(RuntimeError::new(format!("Failed to append to file '{}': {}", path, e))
                                        .with_frame("main", None)),
                                }
                            }
                            Err(e) => Err(RuntimeError::new(format!("Failed to open file '{}': {}", path, e))
                                .with_frame("main", None)),
                        }
                    }
                    _ => Err(RuntimeError::new("append_file expects path (string) and content (string)")
                        .with_frame("main", None)),
                }
            }
            "file_exists" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("file_exists expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                match &args[0] {
                    Value::String(path) => Ok(Value::Bool(std::path::Path::new(path).exists())),
                    _ => Err(RuntimeError::new("file_exists expects a file path (string)")
                        .with_frame("main", None)),
                }
            }
            // Option/Result
            "Some" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("Some expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                Ok(Value::Option(Some(Box::new(args[0].clone()))))
            }
            "None" => {
                if !args.is_empty() {
                    return Err(RuntimeError::new(format!("None expects 0 arguments, got {}", args.len()))
                        .with_frame("main", None));
                }
                Ok(Value::Option(None))
            }
            "Ok" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("Ok expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                Ok(Value::Result(Ok(Box::new(args[0].clone()))))
            }
            "Err" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("Err expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                Ok(Value::Result(Err(Box::new(args[0].clone()))))
            }
            "unwrap" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("unwrap expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                match &args[0] {
                    Value::Option(Some(v)) => Ok((**v).clone()),
                    Value::Option(None) => Err(RuntimeError::new("Called unwrap on None")
                        .with_frame("main", None)),
                    Value::Result(Ok(v)) => Ok((**v).clone()),
                    Value::Result(Err(e)) => Err(RuntimeError::new(format!("Called unwrap on Err: {}", e))
                        .with_frame("main", None)),
                    _ => Err(RuntimeError::new("unwrap expects Option or Result")
                        .with_frame("main", None)),
                }
            }
            "unwrap_or" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(format!("unwrap_or expects 2 arguments, got {}", args.len()))
                        .with_frame("main", None));
                }
                match &args[0] {
                    Value::Option(Some(v)) => Ok((**v).clone()),
                    Value::Option(None) => Ok(args[1].clone()),
                    Value::Result(Ok(v)) => Ok((**v).clone()),
                    Value::Result(Err(_)) => Ok(args[1].clone()),
                    _ => Err(RuntimeError::new("unwrap_or expects Option or Result as first argument")
                        .with_frame("main", None)),
                }
            }
            "is_ok" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("is_ok expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                match &args[0] {
                    Value::Result(Ok(_)) => Ok(Value::Bool(true)),
                    Value::Result(Err(_)) => Ok(Value::Bool(false)),
                    _ => Err(RuntimeError::new("is_ok expects Result")
                        .with_frame("main", None)),
                }
            }
            "is_err" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("is_err expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                match &args[0] {
                    Value::Result(Ok(_)) => Ok(Value::Bool(false)),
                    Value::Result(Err(_)) => Ok(Value::Bool(true)),
                    _ => Err(RuntimeError::new("is_err expects Result")
                        .with_frame("main", None)),
                }
            }
            "is_some" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("is_some expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                match &args[0] {
                    Value::Option(Some(_)) => Ok(Value::Bool(true)),
                    Value::Option(None) => Ok(Value::Bool(false)),
                    _ => Err(RuntimeError::new("is_some expects Option")
                        .with_frame("main", None)),
                }
            }
            "is_none" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("is_none expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                match &args[0] {
                    Value::Option(Some(_)) => Ok(Value::Bool(false)),
                    Value::Option(None) => Ok(Value::Bool(true)),
                    _ => Err(RuntimeError::new("is_none expects Option")
                        .with_frame("main", None)),
                }
            }
            // Map functions
            "map_get" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(format!("map_get expects 2 arguments, got {}", args.len()))
                        .with_frame("main", None));
                }
                match (&args[0], &args[1]) {
                    (Value::Map(map), Value::String(key)) => {
                        match map.get(key) {
                            Some(v) => Ok(Value::Option(Some(Box::new(v.clone())))),
                            None => Ok(Value::Option(None)),
                        }
                    }
                    _ => Err(RuntimeError::new("map_get expects Map and String key")
                        .with_frame("main", None)),
                }
            }
            "map_set" => {
                if args.len() != 3 {
                    return Err(RuntimeError::new(format!("map_set expects 3 arguments, got {}", args.len()))
                        .with_frame("main", None));
                }
                match (&args[0], &args[1]) {
                    (Value::Map(map), Value::String(key)) => {
                        let mut new_map = map.clone();
                        new_map.insert(key.clone(), args[2].clone());
                        Ok(Value::Map(new_map))
                    }
                    _ => Err(RuntimeError::new("map_set expects Map, String key, and value")
                        .with_frame("main", None)),
                }
            }
            "map_keys" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("map_keys expects 1 argument, got {}", args.len()))
                        .with_frame("main", None));
                }
                match &args[0] {
                    Value::Map(map) => {
                        let keys: Vec<Value> = map.keys().map(|k| Value::String(k.clone())).collect();
                        Ok(Value::Array(keys))
                    }
                    _ => Err(RuntimeError::new("map_keys expects Map")
                        .with_frame("main", None)),
                }
            }
            "map_has" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(format!("map_has expects 2 arguments, got {}", args.len()))
                        .with_frame("main", None));
                }
                match (&args[0], &args[1]) {
                    (Value::Map(map), Value::String(key)) => {
                        Ok(Value::Bool(map.contains_key(key)))
                    }
                    _ => Err(RuntimeError::new("map_has expects Map and String key")
                        .with_frame("main", None)),
                }
            }
            // Testing
            "assert_eq" => {
                if args.len() != 2 {
                    return Err(RuntimeError::new(format!("assert_eq expects 2 arguments, got {}", args.len()))
                        .with_frame("test", None));
                }
                if args[0] != args[1] {
                    return Err(RuntimeError::new(format!("Assertion failed: {:?} != {:?}", args[0], args[1]))
                        .with_frame("test", None));
                }
                Ok(Value::Bool(true))
            }
            "assert_true" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("assert_true expects 1 argument, got {}", args.len()))
                        .with_frame("test", None));
                }
                match &args[0] {
                    Value::Bool(true) => Ok(Value::Bool(true)),
                    _ => Err(RuntimeError::new(format!("Assertion failed: expected true, got {:?}", args[0]))
                        .with_frame("test", None)),
                }
            }
            "assert_false" => {
                if args.len() != 1 {
                    return Err(RuntimeError::new(format!("assert_false expects 1 argument, got {}", args.len()))
                        .with_frame("test", None));
                }
                match &args[0] {
                    Value::Bool(false) => Ok(Value::Bool(true)),
                    _ => Err(RuntimeError::new(format!("Assertion failed: expected false, got {:?}", args[0]))
                        .with_frame("test", None)),
                }
            }
            _ => {
                // Cerca funzione utente
                if let Some(func) = self.functions.get(name).cloned() {
                    self.call_user_function(name, &func, args)
                } else {
                    let builtins = vec![
                    "fibonacci", "sqrt", "abs", "len", "pow", "floor", "ceil", "round",
                    "push", "contains", "split", "trim", "starts_with", "ends_with",
                    "to_upper", "to_lower", "substring", "replace",
                    "read_file", "write_file", "append_file", "file_exists",
                    "Some", "None", "Ok", "Err", "unwrap", "unwrap_or",
                    "is_ok", "is_err", "is_some", "is_none",
                    "map_get", "map_set", "map_keys", "map_has",
                    "assert_eq", "assert_true", "assert_false"
                ];
                    let available: Vec<&str> = self.functions.keys()
                        .map(|s| s.as_str())
                        .chain(builtins)
                        .collect();
                    let mut err = RuntimeError::new(format!("Unknown function: {}", name))
                        .with_frame("main", None);
                    if let Some(sugg) = ErrorSuggester::suggest_function(name, &available) {
                        err = err.with_suggestion(sugg);
                    }
                    Err(err)
                }
            }
        }
    }

    fn call_user_function(&mut self, name: &str, func: &UserFunction, args: Vec<Value>) -> Result<Value, RuntimeError> {
        if args.len() != func.params.len() {
            return Err(RuntimeError::new(format!(
                "Function '{}' expected {} arguments, got {}",
                name, func.params.len(), args.len()
            )).with_frame(name, None));
        }

        // Push sullo stack
        self.call_stack.push(name.to_string());

        // Crea nuovo scope per la funzione
        self.push_scope();
        
        // Bind parametri (solo il nome, ignora il tipo)
        for ((param_name, _param_type), arg) in func.params.iter().zip(args.iter()) {
            self.set_variable(param_name, arg.clone());
        }

        // Esegui il corpo della funzione
        for stmt in &func.body {
            if let Err(mut e) = self.execute_stmt(stmt) {
                // Aggiungi frame al stack trace se non già presente
                if !e.stack.iter().any(|f| f.function == name) {
                    e = e.with_frame(name, None);
                }
                self.pop_scope();
                self.call_stack.pop();
                return Err(e);
            }
            if self.return_value.is_some() {
                break;
            }
        }

        // Ottieni il valore di ritorno
        let result = self.return_value.clone().unwrap_or(Value::Void);
        
        // Reset return value
        self.return_value = None;
        
        // Pop scope
        self.pop_scope();

        // Pop dallo stack
        self.call_stack.pop();

        Ok(result)
    }

    fn import_module(&mut self, path: &str, alias: Option<&str>) -> Result<(), RuntimeError> {
        // Verifica se già importato
        if self.imported_modules.contains_key(path) {
            return Ok(());
        }
        
        // Cerca il file in vari percorsi
        let paths_to_try = vec![
            path.to_string(),
            format!(".velora/deps/{}/{}", path, path),
            format!(".velora/deps/{}/{}.vel", path, path),
            format!(".velora/deps/{}/lib.vel", path),
            format!(".velora/deps/{}/main.vel", path),
            format!(".velora/deps/{}/helpers.vel", path),
        ];
        
        let mut content = None;
        let mut actual_path = path.to_string();
        
        for try_path in &paths_to_try {
            if let Ok(c) = std::fs::read_to_string(try_path) {
                content = Some(c);
                actual_path = try_path.clone();
                break;
            }
        }
        
        let content = match content {
            Some(c) => c,
            None => return Err(RuntimeError::new(format!("Cannot import '{}': file not found", path))
                .with_frame("main", None)
                .with_suggestion("Verifica che il file esista, il percorso sia corretto, o esegui 'velora install'")),
        };
        
        // Parsa il modulo
        let module_program = match crate::parser::parse(&content) {
            Ok(p) => p,
            Err(e) => return Err(RuntimeError::new(format!("Parse error in module '{}': {}", path, e))
                .with_frame("main", None)),
        };
        
        // Registra le funzioni del modulo
        let mut module_functions = Vec::new();
        for func in &module_program.functions {
            let func_name = if let Some(alias) = alias {
                format!("{}.{}", alias, func.name)
            } else {
                func.name.clone()
            };
            
            self.functions.insert(
                func_name.clone(),
                UserFunction {
                    params: func.params.clone(),
                    body: func.body.clone(),
                },
            );
            module_functions.push(func_name);
        }
        
        // Registra il modulo come importato
        self.imported_modules.insert(path.to_string(), module_functions);
        
        Ok(())
    }

    pub fn run_tests(&mut self, program: &Program) -> Result<(usize, usize), RuntimeError> {
        let mut passed = 0;
        let mut failed = 0;
        
        // Registra tutte le funzioni utente
        for func in &program.functions {
            self.functions.insert(
                func.name.clone(),
                UserFunction {
                    params: func.params.clone(),
                    body: func.body.clone(),
                },
            );
        }
        
        // Esegui i test
        for test in &program.tests {
            print!("Running test '{}'... ", test.name);
            self.push_scope();
            
            let mut test_passed = true;
            for stmt in &test.body {
                if let Err(e) = self.execute_stmt(stmt) {
                    println!("FAILED");
                    println!("  Error: {}", e.message);
                    test_passed = false;
                    break;
                }
            }
            
            self.pop_scope();
            
            if test_passed {
                println!("PASSED");
                passed += 1;
            } else {
                failed += 1;
            }
        }
        
        Ok((passed, failed))
    }

    fn fibonacci(n: i64) -> i64 {
        if n <= 0 {
            0
        } else if n == 1 {
            1
        } else {
            let mut a = 0;
            let mut b = 1;
            for _ in 2..=n {
                let temp = a + b;
                a = b;
                b = temp;
            }
            b
        }
    }
}

pub fn run_program(program: &Program) -> Result<Vec<String>, RuntimeError> {
    let mut interpreter = Interpreter::new();
    interpreter.run(program)
}
