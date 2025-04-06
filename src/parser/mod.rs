use pest::iterators::{Pair, Pairs};
use pest::Parser;
use pest_derive::Parser;
use std::time::Duration;

#[derive(Parser)]
#[grammar = "parser/grammar.pest"]
pub struct MustermannParser;

// AST structures for the program elements
#[derive(Debug, Clone)]
pub struct Program {
    pub services: Vec<Service>,
}

#[derive(Debug, Clone)]
pub struct Service {
    pub name: String,
    pub methods: Vec<Method>,
    pub loops: Vec<Loop>,
}

#[derive(Debug, Clone)]
pub struct Method {
    pub name: String,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct Loop {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Stdout {
        message: String,
        args: Option<Vec<String>>,
    },
    Stderr {
        message: String,
        args: Option<Vec<String>>,
    },
    Sleep {
        duration: Duration,
    },
    Call {
        service: Option<String>,
        method: String,
    },
}

impl std::fmt::Display for Statement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Statement::Stdout { message, args } => {
                write!(f, "Print({})", message)?;
                if let Some(args) = args {
                    write!(f, "({:?})", args)?;
                }
                Ok(())
            }
            Statement::Sleep { duration } => write!(f, "Sleep({:?})", duration),
            Statement::Call { service, method } => {
                write!(
                    f,
                    "Call({}.{})",
                    service.clone().unwrap_or_default(),
                    method
                )
            }
            Statement::Stderr { message, args } => {
                write!(f, "Stderr({})", message)?;
                if let Some(args) = args {
                    write!(f, "({:?})", args)?;
                }
                Ok(())
            }
        }
    }
}
#[derive(Debug)]
pub enum ParseError {
    PestError(Box<pest::error::Error<Rule>>),
    InvalidInput(String),
}

impl From<pest::error::Error<Rule>> for ParseError {
    fn from(error: pest::error::Error<Rule>) -> Self {
        ParseError::PestError(Box::new(error))
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::PestError(e) => write!(f, "Parser error: {}", e),
            ParseError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
        }
    }
}

impl std::error::Error for ParseError {}

// Main parsing function
pub fn parse(input: &str) -> Result<Program, ParseError> {
    let mut pairs = MustermannParser::parse(Rule::program, input)?;
    parse_program(pairs.next().unwrap().into_inner())
}

// Parse the entire program
fn parse_program(pairs: Pairs<Rule>) -> Result<Program, ParseError> {
    let mut services = Vec::new();

    for pair in pairs {
        match pair.as_rule() {
            Rule::service_def => {
                services.push(parse_service(pair)?);
            }
            Rule::EOI => {}
            _ => {
                return Err(ParseError::InvalidInput(format!(
                    "Unexpected rule: {:?}",
                    pair.as_rule()
                )))
            }
        }
    }

    Ok(Program { services })
}

// Parse a service definition
fn parse_service(pair: Pair<Rule>) -> Result<Service, ParseError> {
    let mut inner_pairs = pair.into_inner();

    // Get the service name
    let name = inner_pairs
        .next()
        .and_then(|p| {
            if p.as_rule() == Rule::identifier {
                Some(p.as_str().to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| ParseError::InvalidInput("Expected service name".to_string()))?;

    let mut methods = Vec::new();
    let mut loops = Vec::new();

    // Parse method and loop definitions
    for pair in inner_pairs {
        match pair.as_rule() {
            Rule::method_def => {
                methods.push(parse_method(pair)?);
            }
            Rule::loop_def => {
                loops.push(parse_loop(pair)?);
            }
            _ => {}
        }
    }

    Ok(Service {
        name,
        methods,
        loops,
    })
}

// Parse a method definition
fn parse_method(pair: Pair<Rule>) -> Result<Method, ParseError> {
    let mut inner_pairs = pair.into_inner();

    // Get the method name
    let name = inner_pairs
        .next()
        .and_then(|p| {
            if p.as_rule() == Rule::identifier {
                Some(p.as_str().to_string())
            } else {
                None
            }
        })
        .ok_or_else(|| ParseError::InvalidInput("Expected method name".to_string()))?;

    let mut statements = Vec::new();

    // Parse statements
    for pair in inner_pairs {
        if pair.as_rule() == Rule::statement {
            statements.push(parse_statement(pair)?);
        }
    }

    Ok(Method { name, statements })
}

// Parse a loop definition
fn parse_loop(pair: Pair<Rule>) -> Result<Loop, ParseError> {
    let mut statements = Vec::new();

    // Parse statements in the loop
    for pair in pair.into_inner() {
        if pair.as_rule() == Rule::statement {
            statements.push(parse_statement(pair)?);
        }
    }

    Ok(Loop { statements })
}

// Parse a statement
fn parse_statement(pair: Pair<Rule>) -> Result<Statement, ParseError> {
    let inner = pair
        .into_inner()
        .next()
        .ok_or_else(|| ParseError::InvalidInput("Empty statement".to_string()))?;

    match inner.as_rule() {
        Rule::print_stmt => parse_print_statement(inner),
        Rule::sleep_stmt => parse_sleep_statement(inner),
        Rule::call_stmt => parse_call_statement(inner),
        _ => Err(ParseError::InvalidInput(format!(
            "Unexpected statement type: {:?}",
            inner.as_rule()
        ))),
    }
}

// Parse a print statement
fn parse_print_statement(pair: Pair<Rule>) -> Result<Statement, ParseError> {
    let mut inner_pairs = pair.into_inner();

    // Get the print channel (print or stderr)
    let channel_pair = inner_pairs.next().ok_or_else(|| {
        ParseError::InvalidInput("Expected print channel in print statement".to_string())
    })?;

    let is_stderr = channel_pair.as_str() == "stderr";

    // Get the message string
    let message_pair = inner_pairs.next().ok_or_else(|| {
        ParseError::InvalidInput("Expected string literal in print statement".to_string())
    })?;

    let message = if message_pair.as_rule() == Rule::string_literal {
        // Remove quotes from the string literal
        let raw_str = message_pair.as_str();
        raw_str[1..raw_str.len() - 1].to_string()
    } else {
        return Err(ParseError::InvalidInput(
            "Expected string literal in print statement".to_string(),
        ));
    };

    // Parse optional array literal for arguments
    let args = if let Some(array_pair) = inner_pairs.find(|p| p.as_rule() == Rule::array_literal) {
        let mut args = Vec::new();

        for str_pair in array_pair.into_inner() {
            if str_pair.as_rule() == Rule::string_literal {
                let raw_str = str_pair.as_str();
                args.push(raw_str[1..raw_str.len() - 1].to_string());
            }
        }

        Some(args)
    } else {
        None
    };

    if is_stderr {
        Ok(Statement::Stderr { message, args })
    } else {
        Ok(Statement::Stdout { message, args })
    }
}

// Parse a sleep statement
fn parse_sleep_statement(pair: Pair<Rule>) -> Result<Statement, ParseError> {
    let time_value_pair = pair.into_inner().next().ok_or_else(|| {
        ParseError::InvalidInput("Expected time value in sleep statement".to_string())
    })?;

    if time_value_pair.as_rule() != Rule::time_value {
        return Err(ParseError::InvalidInput(
            "Expected time value in sleep statement".to_string(),
        ));
    }

    let mut inner_pairs = time_value_pair.into_inner();

    let number_str = inner_pairs
        .next()
        .and_then(|p| {
            if p.as_rule() == Rule::number {
                Some(p.as_str())
            } else {
                None
            }
        })
        .ok_or_else(|| ParseError::InvalidInput("Expected number in time value".to_string()))?;

    let number: u64 = number_str
        .parse()
        .map_err(|_| ParseError::InvalidInput(format!("Invalid number: {}", number_str)))?;

    let unit = inner_pairs
        .next()
        .and_then(|p| {
            if p.as_rule() == Rule::time_unit {
                Some(p.as_str())
            } else {
                None
            }
        })
        .ok_or_else(|| ParseError::InvalidInput("Expected time unit in time value".to_string()))?;

    let duration = match unit {
        "ms" => Duration::from_millis(number),
        "s" => Duration::from_secs(number),
        _ => {
            return Err(ParseError::InvalidInput(format!(
                "Invalid time unit: {}",
                unit
            )))
        }
    };

    Ok(Statement::Sleep { duration })
}

// Parse a call statement
fn parse_call_statement(pair: Pair<Rule>) -> Result<Statement, ParseError> {
    let mut inner_pairs = pair.into_inner();

    let mut service_name = None;
    let mut method_name = None;

    // Process the pairs to extract service and method names
    let mut pairs_vec: Vec<Pair<Rule>> = inner_pairs.collect();

    if pairs_vec.len() == 1 {
        // Only method name is present
        if pairs_vec[0].as_rule() == Rule::identifier {
            method_name = Some(pairs_vec[0].as_str().to_string());
        }
    } else if pairs_vec.len() == 2 {
        // Both service and method names are present
        if pairs_vec[0].as_rule() == Rule::identifier && pairs_vec[1].as_rule() == Rule::identifier
        {
            service_name = Some(pairs_vec[0].as_str().to_string());
            method_name = Some(pairs_vec[1].as_str().to_string());
        }
    }

    // Ensure we have at least a method name
    let method = method_name.ok_or_else(|| {
        ParseError::InvalidInput("Expected method name in call statement".to_string())
    })?;

    Ok(Statement::Call {
        service: service_name,
        method,
    })
}

// Helper trait for rule enum
pub trait RuleTrait {
    fn as_str(&self) -> &'static str;
}

impl RuleTrait for Rule {
    fn as_str(&self) -> &'static str {
        match self {
            Rule::program => "program",
            Rule::service_def => "service_def",
            Rule::method_def => "method_def",
            Rule::loop_def => "loop_def",
            Rule::statement => "statement",
            Rule::print_stmt => "print_stmt",
            Rule::sleep_stmt => "sleep_stmt",
            Rule::call_stmt => "call_stmt",
            Rule::time_value => "time_value",
            Rule::time_unit => "time_unit",
            Rule::array_literal => "array_literal",
            Rule::string_literal => "string_literal",
            Rule::identifier => "identifier",
            Rule::number => "number",
            Rule::WHITESPACE => "WHITESPACE",
            Rule::COMMENT => "COMMENT",
            Rule::EOI => "EOI",
            Rule::print_channel => "print_channel",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_service() {
        let service = "
        service products {
            method get_products {
                print \"Fetching product orders %s\" with []
            }
        }
        ";
        let ast = parse(service).unwrap();

        assert_eq!(ast.services.len(), 1);
        assert_eq!(ast.services[0].name, "products");
        assert_eq!(ast.services[0].methods.len(), 1);
        assert_eq!(ast.services[0].methods[0].name, "get_products");
    }

    #[test]
    fn test_parse_service_with_empty_var_list() {
        let service = "
        service products {
            method get_products {
                print \"Fetching product orders %s\" with []
            }
        }
        ";
        let ast = parse(service).unwrap();

        assert_eq!(ast.services.len(), 1);
        assert_eq!(ast.services[0].name, "products");
        assert_eq!(ast.services[0].methods.len(), 1);
        assert_eq!(ast.services[0].methods[0].name, "get_products");
        assert_eq!(ast.services[0].methods[0].statements.len(), 1);
        assert_eq!(
            ast.services[0].methods[0].statements[0],
            Statement::Stdout {
                message: "Fetching product orders %s".to_string(),
                args: Some(vec![]),
            }
        );
    }

    #[test]
    fn test_parse_service_with_empty_var_list_and_sleep() {
        let service = "
        service products {
            method get_products {
                print \"Fetching product orders %s\" with []
                sleep 1s
            }
        }
        ";
        let ast = parse(service).unwrap();

        assert_eq!(ast.services.len(), 1);
        assert_eq!(ast.services[0].name, "products");
        assert_eq!(ast.services[0].methods.len(), 1);
        assert_eq!(ast.services[0].methods[0].name, "get_products");
        assert_eq!(ast.services[0].methods[0].statements.len(), 2);
        assert_eq!(
            ast.services[0].methods[0].statements[0],
            Statement::Stdout {
                message: "Fetching product orders %s".to_string(),
                args: Some(vec![]),
            }
        );
        assert_eq!(
            ast.services[0].methods[0].statements[1],
            Statement::Sleep {
                duration: Duration::from_secs(1),
            }
        );
    }

    #[test]
    fn test_parse_service_with_stderr() {
        let service = "
        service products {
            method get_products {
                print \"Fetching product orders %s\" with []
                stderr \"Error fetching product orders\"
            }
        }
        ";
        let ast = parse(service).unwrap();

        assert_eq!(ast.services.len(), 1);
        assert_eq!(ast.services[0].name, "products");
        assert_eq!(ast.services[0].methods.len(), 1);
        assert_eq!(ast.services[0].methods[0].name, "get_products");
        assert_eq!(ast.services[0].methods[0].statements.len(), 2);
        assert_eq!(
            ast.services[0].methods[0].statements[0],
            Statement::Stdout {
                message: "Fetching product orders %s".to_string(),
                args: Some(vec![]),
            }
        );
        assert_eq!(
            ast.services[0].methods[0].statements[1],
            Statement::Stderr {
                message: "Error fetching product orders".to_string(),
                args: None,
            }
        );
    }
}
