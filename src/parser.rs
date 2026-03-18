use pest::Parser;
use pest::iterators::Pairs;
use pest::pratt_parser::{PrattParser, Assoc, Op};

use crate::ast::*;
use crate::error::{ParseError, ErrorSuggester};

#[derive(pest_derive::Parser)]
#[grammar = "src/velora.pest"]
struct VeloraParser;

lazy_static::lazy_static! {
    static ref PRATT: PrattParser<Rule> = {
        PrattParser::new()
            // Or e And
            .op(Op::infix(Rule::or, Assoc::Left) | Op::infix(Rule::and, Assoc::Left))
            // Confronti
            .op(Op::infix(Rule::eq, Assoc::Left) 
                | Op::infix(Rule::neq, Assoc::Left)
                | Op::infix(Rule::lte, Assoc::Left)
                | Op::infix(Rule::gte, Assoc::Left)
                | Op::infix(Rule::lt, Assoc::Left)
                | Op::infix(Rule::gt, Assoc::Left))
            // Addizione e sottrazione
            .op(Op::infix(Rule::plus, Assoc::Left) | Op::infix(Rule::minus, Assoc::Left))
            // Moltiplicazione, divisione, modulo
            .op(Op::infix(Rule::mul, Assoc::Left) 
                | Op::infix(Rule::div, Assoc::Left)
                | Op::infix(Rule::modulo, Assoc::Left))
            // Operatori unari (usano regole separate: neg per prefix -, not_op per prefix !)
            .op(Op::prefix(Rule::neg) | Op::prefix(Rule::not_op))
    };
}

/// Parse input string into a Program
pub fn parse(input: &str) -> Result<Program, ParseError> {
    // Velora AI-First - preprocessing con AI e validazione
    let processed_input = preprocess_ai_directives(input)?;
    
    let pairs = VeloraParser::parse(Rule::program, &processed_input)
        .map_err(|e| convert_pest_error(&processed_input, e))?;
    
    parse_program(pairs, &processed_input)
}

/// Converte un errore Pest in ParseError con posizione
fn convert_pest_error(input: &str, error: pest::error::Error<Rule>) -> ParseError {
    let (line, col) = match &error.location {
        pest::error::InputLocation::Pos(pos) => line_col_from_pos(input, *pos),
        pest::error::InputLocation::Span((start, _)) => line_col_from_pos(input, *start),
    };
    
    let message = format!("{}", error.variant.message());
    let mut err = ParseError::new(message, line, col);
    
    // Aggiungi snippet
    let lines: Vec<&str> = input.lines().collect();
    if line > 0 && line <= lines.len() {
        err = err.with_snippet(lines[line - 1]);
    }
    
    // Aggiungi suggestion se disponibile
    if let Some(sugg) = ErrorSuggester::suggest_parse_fix(&error.variant.to_string()) {
        err = err.with_suggestion(sugg);
    }
    
    err
}

/// Calcola linea e colonna da una posizione nel testo
fn line_col_from_pos(input: &str, pos: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    
    for (i, ch) in input.char_indices() {
        if i >= pos {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    
    (line, col)
}

/// Crea un ParseError da una pair Pest con contesto
fn make_error_from_pair(pair: &pest::iterators::Pair<Rule>, message: impl Into<String>) -> ParseError {
    let (line, col) = pair.as_span().start_pos().line_col();
    let snippet = pair.as_str().to_string();
    
    ParseError::new(message, line, col)
        .with_snippet(snippet)
}

/// Crea un ParseError semplice
fn make_error(line: usize, col: usize, message: impl Into<String>) -> ParseError {
    ParseError::new(message, line, col)
}

/// Preprocessa le direttive AI con validazione e retry
fn preprocess_ai_directives(input: &str) -> Result<String, ParseError> {
    let mut functions_section = String::new();
    let mut generated_functions = String::new();
    let mut generated_statements = String::new();
    let mut in_main_block = false;
    let mut main_content = String::new();
    
    for (line_num, line) in input.lines().enumerate() {
        let trimmed = line.trim();
        
        if trimmed == "main:" {
            in_main_block = true;
            continue;
        }
        
        if trimmed.starts_with("# AI:") {
            let request = trimmed[5..].trim();
            
            println!("🤖 Auto-evoluzione: {}", request);
            
            // Se la richiesta inizia con "self", usa il self-hosting
            let (functions, statements) = if request.starts_with("self ") {
                let self_request = &request[5..];
                match crate::ai::self_host(self_request) {
                    Ok(code) => {
                        // Il codice generato include già main:, lo mettiamo in generated_functions
                        // dopo aver rimosso il main: wrapper
                        let code_without_main = code.replace("main:\n    print(auto_evolved_feature())", "");
                        (code_without_main.trim().to_string(), "print(auto_evolved_feature())\n".to_string())
                    }
                    Err(e) => {
                        eprintln!("   ⚠️ Self-hosting error: {}", e);
                        (String::new(), String::new())
                    }
                }
            } else {
                // Usa sistema smart: template -> cache -> GROK -> fallback
                crate::ai::generate_code_smart(request).unwrap_or((String::new(), String::new()))
            };
            
            if !functions.is_empty() {
                generated_functions.push_str(&functions);
                generated_functions.push('\n');
            }
            if !statements.is_empty() {
                generated_statements.push_str(&statements);
                generated_statements.push('\n');
            }
        } else if in_main_block {
            // Accumula contenuto del main originale
            main_content.push_str(line);
            main_content.push('\n');
        } else {
            // Accumula funzioni esterne al main
            functions_section.push_str(line);
            functions_section.push('\n');
        }
    }
    
    // Assembla il risultato finale:
    let mut result = String::new();
    
    // Sezione funzioni
    result.push_str(&functions_section);
    result.push_str(&generated_functions);
    
    // Main block
    result.push_str("main:\n");
    result.push_str(&main_content);
    result.push_str(&generated_statements);
    
    Ok(result)
}

fn parse_program(mut pairs: Pairs<Rule>, source: &str) -> Result<Program, ParseError> {
    let mut program = Program::new();
    
    for pair in pairs.next().unwrap().into_inner() {
        match pair.as_rule() {
            Rule::import_stmt => {
                let mut inner = pair.into_inner();
                let path_pair = inner.next().unwrap();
                let path = path_pair.as_str();
                let path = path[1..path.len()-1].to_string(); // rimuovi virgolette
                let alias = inner.next().map(|a| a.as_str().to_string());
                program.imports.push((path, alias));
            }
            Rule::main_block => {
                program.main = parse_main_block(pair, source)?;
            }
            Rule::function_def => {
                program.functions.push(parse_function_def(pair, source)?);
            }
            Rule::test_block => {
                program.tests.push(parse_test_block(pair, source)?);
            }
            Rule::EOI => break,
            _ => {}
        }
    }
    
    Ok(program)
}

fn parse_test_block(pair: pest::iterators::Pair<Rule>, source: &str) -> Result<Test, ParseError> {
    let mut inner = pair.into_inner();
    let name_pair = inner.next().unwrap();
    let name = name_pair.as_str();
    let name = name[1..name.len()-1].to_string(); // rimuovi virgolette
    
    let mut body = Vec::new();
    for stmt_pair in inner {
        if stmt_pair.as_rule() == Rule::statement {
            body.push(parse_statement(stmt_pair, source)?);
        }
    }
    
    Ok(Test { name, body })
}

fn parse_main_block(pair: pest::iterators::Pair<Rule>, source: &str) -> Result<Vec<Stmt>, ParseError> {
    let mut stmts = Vec::new();
    
    for inner in pair.into_inner() {
        if inner.as_rule() == Rule::statement {
            stmts.push(parse_statement(inner, source)?);
        }
    }
    
    Ok(stmts)
}

fn parse_block(pair: pest::iterators::Pair<Rule>, source: &str) -> Result<Vec<Stmt>, ParseError> {
    let mut stmts = Vec::new();
    
    match pair.as_rule() {
        Rule::block | Rule::block_stmt => {
            for inner in pair.into_inner() {
                match inner.as_rule() {
                    Rule::statement => stmts.push(parse_statement(inner, source)?),
                    Rule::block_stmt => stmts.push(Stmt::Block(parse_block(inner, source)?)),
                    _ => {}
                }
            }
        }
        Rule::statement => {
            stmts.push(parse_statement(pair, source)?);
        }
        _ => {}
    }
    
    Ok(stmts)
}

fn parse_function_def(pair: pest::iterators::Pair<Rule>, source: &str) -> Result<Function, ParseError> {
    let span = pair.as_span();
    let (line, col) = span.start_pos().line_col();
    let mut inner = pair.into_inner();
    
    let name = inner.next()
        .ok_or_else(|| make_error(line, col, "Missing function name"))?
        .as_str().to_string();
    
    // Parse params opzionali con type annotations
    let params_item = inner.next()
        .ok_or_else(|| make_error(line, col, "Missing function parameters or return type"))?;
    
    let params = if params_item.as_rule() == Rule::params {
        let mut p = Vec::new();
        for param in params_item.into_inner() {
            // Ogni param può essere: identifier OPPURE identifier ~ ":" ~ type_name
            let mut param_inner = param.into_inner();
            let param_name = param_inner.next()
                .ok_or_else(|| make_error(line, col, "Invalid parameter"))?
                .as_str().to_string();
            let param_type = param_inner.next().map(|t| t.as_str().to_string());
            p.push((param_name, param_type));
        }
        p
    } else {
        // Se non c'è params, params_item è il type_name, lo skippiamo dopo
        Vec::new()
    };
    
    // Trova il body (block)
    let mut body = Vec::new();
    for item in inner {
        if item.as_rule() == Rule::block || item.as_rule() == Rule::block_stmt {
            body = parse_block(item, source)?;
            break;
        }
    }
    
    if body.is_empty() {
        return Err(make_error(line, col, format!("Function '{}' has no body", name)));
    }
    
    Ok(Function { name, params, body })
}

fn parse_statement(pair: pest::iterators::Pair<Rule>, source: &str) -> Result<Stmt, ParseError> {
    let mut inner_pairs = pair.into_inner();
    let inner = inner_pairs.next()
        .ok_or_else(|| make_error(0, 0, "Empty statement"))?;
    
    match inner.as_rule() {
        Rule::print_stmt => {
            let expr_pair = inner.into_inner().next()
                .ok_or_else(|| make_error(0, 0, "Empty print statement"))?;
            Ok(Stmt::Print(parse_expr(expr_pair, source)?))
        }
        Rule::let_stmt => {
            let mut inner = inner.into_inner();
            let name = inner.next()
                .ok_or_else(|| make_error(0, 0, "Missing variable name in let"))?
                .as_str().to_string();
            let value_pair = inner.next()
                .ok_or_else(|| make_error(0, 0, "Missing value in let statement"))?;
            let value = parse_expr(value_pair, source)?;
            Ok(Stmt::Let { name, value })
        }
        Rule::assignment_stmt => {
            let mut inner = inner.into_inner();
            let name = inner.next()
                .ok_or_else(|| make_error(0, 0, "Missing variable name in assignment"))?
                .as_str().to_string();
            let value_pair = inner.next()
                .ok_or_else(|| make_error(0, 0, "Missing value in assignment"))?;
            let value = parse_expr(value_pair, source)?;
            Ok(Stmt::Assign { name, value })
        }
        Rule::return_stmt => {
            let mut expr = None;
            for item in inner.into_inner() {
                expr = Some(parse_expr(item, source)?);
            }
            Ok(Stmt::Return(expr))
        }
        Rule::expr_stmt => {
            let expr_pair = inner.into_inner().next()
                .ok_or_else(|| make_error(0, 0, "Empty expression statement"))?;
            Ok(Stmt::Expr(parse_expr(expr_pair, source)?))
        }
        Rule::if_stmt => {
            let mut inner = inner.into_inner();
            let condition = parse_expr(inner.next()
                .ok_or_else(|| make_error(0, 0, "Missing if condition"))?, source)?;
            
            let then_block = inner.next()
                .ok_or_else(|| make_error(0, 0, "Missing if then block"))?;
            let then_branch = parse_block(then_block, source)?;
            
            let else_branch = if let Some(else_block) = inner.next() {
                Some(parse_block(else_block, source)?)
            } else {
                None
            };
            
            Ok(Stmt::If { condition, then_branch, else_branch })
        }
        Rule::while_stmt => {
            let mut inner = inner.into_inner();
            let condition = parse_expr(inner.next()
                .ok_or_else(|| make_error(0, 0, "Missing while condition"))?, source)?;
            
            let body_block = inner.next()
                .ok_or_else(|| make_error(0, 0, "Missing while body"))?;
            let body = parse_block(body_block, source)?;
            
            Ok(Stmt::While { condition, body })
        }
        Rule::block_stmt => {
            let stmts: Result<Vec<_>, _> = inner.into_inner()
                .filter(|p| p.as_rule() == Rule::statement)
                .map(|p| parse_statement(p, source))
                .collect();
            Ok(Stmt::Block(stmts?))
        }
        _ => Err(make_error_from_pair(&inner, format!("Unexpected statement: {:?}", inner.as_rule()))),
    }
}

fn parse_expr(pair: pest::iterators::Pair<Rule>, source: &str) -> Result<Expr, ParseError> {
    match pair.as_rule() {
        Rule::expr => parse_pratt_expr(pair, source),
        Rule::primary => parse_primary(pair, source),
        Rule::number_lit => {
            let s = pair.as_str();
            // Prova a parsare come float se contiene '.'
            if s.contains('.') || s.contains('e') || s.contains('E') {
                let num = s.parse::<f64>()
                    .map_err(|e| make_error_from_pair(&pair, e.to_string()))?;
                Ok(Expr::Float(num))
            } else {
                let num = s.parse::<i64>()
                    .map_err(|e| make_error_from_pair(&pair, e.to_string()))?;
                Ok(Expr::Number(num))
            }
        }
        Rule::string_lit => {
            let s = pair.as_str();
            Ok(Expr::String(s[1..s.len()-1].to_string()))
        }
        Rule::bool_lit => {
            let s = pair.as_str();
            Ok(Expr::Bool(s == "true"))
        }
        Rule::identifier => {
            let s = pair.as_str();
            Ok(Expr::Variable(s.to_string()))
        }
        Rule::function_call => {
            parse_function_call(pair, source)
        }
        Rule::array_lit => {
            let mut elements = Vec::new();
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::expr {
                    elements.push(parse_expr(inner, source)?);
                }
            }
            Ok(Expr::Array(elements))
        }
        Rule::map_lit => {
            let mut entries = Vec::new();
            for inner in pair.into_inner() {
                if inner.as_rule() == Rule::map_entry {
                    let mut entry_inner = inner.into_inner();
                    let key_pair = entry_inner.next().unwrap();
                    let key = if key_pair.as_rule() == Rule::string_lit {
                        let s = key_pair.as_str();
                        s[1..s.len()-1].to_string()
                    } else {
                        key_pair.as_str().to_string()
                    };
                    let value_pair = entry_inner.next().unwrap();
                    let value = parse_expr(value_pair, source)?;
                    entries.push((key, value));
                }
            }
            Ok(Expr::Map(entries))
        }
        Rule::index_expr => {
            let mut inner = pair.into_inner();
            let name = inner.next()
                .ok_or_else(|| make_error(0, 0, "Missing array name in index"))?
                .as_str().to_string();
            let index_pair = inner.next()
                .ok_or_else(|| make_error(0, 0, "Missing index expression"))?;
            let index = parse_expr(index_pair, source)?;
            Ok(Expr::Index { name, index: Box::new(index) })
        }
        _ => Err(make_error_from_pair(&pair, format!("Unexpected expression rule: {:?}", pair.as_rule()))),
    }
}

fn parse_pratt_expr(pair: pest::iterators::Pair<Rule>, source: &str) -> Result<Expr, ParseError> {
    PRATT.map_primary(|primary| parse_expr(primary, source))
        .map_infix(|left, op, right| {
            let op = match op.as_rule() {
                Rule::plus => BinaryOp::Add,
                Rule::minus => BinaryOp::Sub,
                Rule::mul => BinaryOp::Mul,
                Rule::div => BinaryOp::Div,
                Rule::modulo => BinaryOp::Mod,
                Rule::eq => BinaryOp::Eq,
                Rule::neq => BinaryOp::Neq,
                Rule::lt => BinaryOp::Lt,
                Rule::gt => BinaryOp::Gt,
                Rule::lte => BinaryOp::Lte,
                Rule::gte => BinaryOp::Gte,
                Rule::and => BinaryOp::And,
                Rule::or => BinaryOp::Or,
                _ => panic!("Unknown binary operator: {:?}", op.as_rule()),
            };
            Ok(Expr::Binary {
                left: Box::new(left?),
                op,
                right: Box::new(right?),
            })
        })
        .map_prefix(|op, expr| {
            let op = match op.as_rule() {
                Rule::neg => UnaryOp::Neg,
                Rule::not_op => UnaryOp::Not,
                _ => panic!("Unknown unary operator: {:?}", op.as_rule()),
            };
            Ok(Expr::Unary {
                op,
                expr: Box::new(expr?),
            })
        })
        .parse(pair.into_inner())
}

fn parse_function_call(pair: pest::iterators::Pair<Rule>, source: &str) -> Result<Expr, ParseError> {
    let mut inner = pair.into_inner();
    let name_pair = inner.next()
        .ok_or_else(|| make_error(0, 0, "Missing function name in call"))?;
    
    // Il nome può essere un identifier singolo o qualificato (mod.func)
    let name = if name_pair.as_rule() == Rule::func_name {
        name_pair.as_str().to_string()
    } else {
        name_pair.as_str().to_string()
    };
    
    let mut args = Vec::new();
    
    for arg in inner {
        if arg.as_rule() == Rule::call_args {
            for a in arg.into_inner() {
                args.push(parse_expr(a, source)?);
            }
        }
    }
    
    Ok(Expr::Call { name, args })
}

fn parse_primary(pair: pest::iterators::Pair<Rule>, source: &str) -> Result<Expr, ParseError> {
    let mut inner_pairs = pair.into_inner();
    let inner = inner_pairs.next()
        .ok_or_else(|| make_error(0, 0, "Empty primary expression"))?;
    
    match inner.as_rule() {
        Rule::number_lit => {
            let s = inner.as_str();
            if s.contains('.') || s.contains('e') || s.contains('E') {
                let num = s.parse::<f64>()
                    .map_err(|e| make_error_from_pair(&inner, e.to_string()))?;
                Ok(Expr::Float(num))
            } else {
                let num = s.parse::<i64>()
                    .map_err(|e| make_error_from_pair(&inner, e.to_string()))?;
                Ok(Expr::Number(num))
            }
        }
        Rule::string_lit => {
            let s = inner.as_str();
            Ok(Expr::String(s[1..s.len()-1].to_string()))
        }
        Rule::bool_lit => {
            let s = inner.as_str();
            Ok(Expr::Bool(s == "true"))
        }
        Rule::function_call => {
            parse_function_call(inner, source)
        }
        Rule::array_lit => {
            let mut elements = Vec::new();
            for item in inner.into_inner() {
                if item.as_rule() == Rule::expr {
                    elements.push(parse_expr(item, source)?);
                }
            }
            Ok(Expr::Array(elements))
        }
        Rule::map_lit => {
            let mut entries = Vec::new();
            for item in inner.into_inner() {
                if item.as_rule() == Rule::map_entry {
                    let mut entry_inner = item.into_inner();
                    let key_pair = entry_inner.next().unwrap();
                    let key = if key_pair.as_rule() == Rule::string_lit {
                        let s = key_pair.as_str();
                        s[1..s.len()-1].to_string()
                    } else {
                        key_pair.as_str().to_string()
                    };
                    let value_pair = entry_inner.next().unwrap();
                    let value = parse_expr(value_pair, source)?;
                    entries.push((key, value));
                }
            }
            Ok(Expr::Map(entries))
        }
        Rule::index_expr => {
            let mut inner_pairs = inner.into_inner();
            let name = inner_pairs.next()
                .ok_or_else(|| make_error(0, 0, "Missing array name in index"))?
                .as_str().to_string();
            let index_pair = inner_pairs.next()
                .ok_or_else(|| make_error(0, 0, "Missing index expression"))?;
            let index = parse_expr(index_pair, source)?;
            Ok(Expr::Index { name, index: Box::new(index) })
        }
        Rule::identifier => {
            let s = inner.as_str();
            Ok(Expr::Variable(s.to_string()))
        }
        Rule::expr => parse_expr(inner, source),
        _ => Err(make_error_from_pair(&inner, format!("Unexpected primary: {:?}", inner.as_rule()))),
    }
}
