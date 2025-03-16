use std::collections::HashMap;
use std::time::Duration as StdDuration;

#[derive(Debug, Clone, PartialEq)]
pub enum LogSeverity {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

impl LogSeverity {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_uppercase().as_str() {
            "TRACE" => Ok(LogSeverity::Trace),
            "DEBUG" => Ok(LogSeverity::Debug),
            "INFO" => Ok(LogSeverity::Info),
            "WARN" => Ok(LogSeverity::Warn),
            "ERROR" => Ok(LogSeverity::Error),
            "FATAL" => Ok(LogSeverity::Fatal),
            _ => Err(format!("Unknown severity level: {}", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MusterDuration {
    Indefinite,
    Fixed(StdDuration),
}

impl MusterDuration {
    pub fn from_str(s: &str) -> Result<Self, String> {
        if s == "indefinite" {
            return Ok(MusterDuration::Indefinite);
        }

        // Parse duration in the format 5m, 1h, etc.
        let last_char = s.chars().last().ok_or("Empty duration string")?;
        let value = s[0..s.len() - 1]
            .parse::<u64>()
            .map_err(|e| e.to_string())?;

        match last_char {
            's' => Ok(MusterDuration::Fixed(StdDuration::from_secs(value))),
            'm' => Ok(MusterDuration::Fixed(StdDuration::from_secs(value * 60))),
            'h' => Ok(MusterDuration::Fixed(StdDuration::from_secs(value * 3600))),
            'd' => Ok(MusterDuration::Fixed(StdDuration::from_secs(value * 86400))),
            _ => Err(format!("Unknown duration unit: {}", last_char)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(String),
    Number(f64),
    Boolean(bool),
    Reference(String), // Reference to a variable like $user.id
    FakeGenerator { generator: String, args: Vec<Value> },
    Object(HashMap<String, Value>),
    Array(Vec<Value>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Template {
    pub severity: LogSeverity,
    pub message: String,
    pub weight: Option<u32>,
    pub condition: Option<String>, // Condition like "$transaction.status == 'declined'"
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlowStep {
    pub severity: LogSeverity,
    pub message: String,
    pub delay: Option<MusterDuration>,
    pub probability: Option<f64>,
    pub condition: Option<String>,
    pub data: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Flow {
    pub name: String,
    pub steps: Vec<FlowStep>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimePattern {
    pub schedule: String,
    pub multiplier: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OutputFormat {
    Json,
    PlainText,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OutputDestination {
    Stdout,
    File(String),
    Http(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutputConfig {
    pub format: OutputFormat,
    pub destination: OutputDestination,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Context {
    pub fields: HashMap<String, Value>,
    pub persistent: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LogsBlock {
    pub application_name: String,
    pub duration: MusterDuration,
    pub frequency: u64, // in milliseconds
    pub templates: Vec<Template>,
    pub flows: Option<Vec<Flow>>,
    pub data: HashMap<String, Value>,
    pub context: Option<Context>,
    pub patterns: Option<HashMap<String, TimePattern>>,
    pub output: Option<OutputConfig>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Muster {
    pub logs_blocks: Vec<LogsBlock>,
}

pub mod parser {
    #![allow(unused_assignments)]
    use super::*;
    use nom::{
        branch::alt,
        bytes::complete::{tag, take_until, take_while1},
        character::complete::{alpha1, alphanumeric1, char, digit1, multispace0},
        combinator::{map, map_res, opt, recognize, value},
        multi::{many0, separated_list0, separated_list1},
        sequence::{delimited, pair, preceded, separated_pair, tuple},
        IResult,
    };
    use std::str::FromStr;

    // Helper functions for parsing

    fn ws<'a, F, O>(inner: F) -> impl FnMut(&'a str) -> IResult<&'a str, O>
    where
        F: FnMut(&'a str) -> IResult<&'a str, O>,
    {
        delimited(multispace0, inner, multispace0)
    }

    fn identifier(input: &str) -> IResult<&str, &str> {
        recognize(pair(
            alt((alpha1, tag("_"))),
            many0(alt((alphanumeric1, tag("_")))),
        ))(input)
    }

    fn string_literal(input: &str) -> IResult<&str, String> {
        delimited(
            char('"'),
            map(take_until("\""), |s: &str| s.to_string()),
            char('"'),
        )(input)
    }

    fn number_literal(input: &str) -> IResult<&str, f64> {
        map_res(
            recognize(tuple((
                opt(char('-')),
                digit1,
                opt(pair(char('.'), digit1)),
            ))),
            |s: &str| f64::from_str(s),
        )(input)
    }

    fn boolean_literal(input: &str) -> IResult<&str, bool> {
        alt((value(true, tag("true")), value(false, tag("false"))))(input)
    }

    fn reference(input: &str) -> IResult<&str, String> {
        preceded(
            char('$'),
            map(
                recognize(pair(identifier, many0(preceded(char('.'), identifier)))),
                |s: &str| s.to_string(),
            ),
        )(input)
    }

    fn parse_value(input: &str) -> IResult<&str, Value> {
        alt((
            map(string_literal, Value::String),
            map(number_literal, Value::Number),
            map(boolean_literal, Value::Boolean),
            map(reference, Value::Reference),
            map(parse_fake_generator, |(generator, args)| {
                Value::FakeGenerator { generator, args }
            }),
            map(parse_object, |obj| Value::Object(obj)),
            map(parse_array, |arr| Value::Array(arr)),
        ))(input)
    }

    fn parse_fake_generator(input: &str) -> IResult<&str, (String, Vec<Value>)> {
        // First, check if the input starts with "fake::"
        if !input.starts_with("fake::") {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )));
        }

        // Find the opening parenthesis
        let Some(paren_pos) = input.find('(') else {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )));
        };

        // Extract the generator name
        let generator = input[..paren_pos].trim().to_string();

        // Find the matching closing parenthesis
        let mut paren_count = 0;
        let mut in_string = false;
        let mut escape_next = false;
        let mut close_paren_pos = 0;

        for (i, c) in input[paren_pos..].char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match c {
                '\\' if in_string => escape_next = true,
                '"' => in_string = !in_string,
                '(' if !in_string => paren_count += 1,
                ')' if !in_string => {
                    paren_count -= 1;
                    if paren_count == 0 {
                        close_paren_pos = paren_pos + i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }

        if close_paren_pos == 0 {
            return Err(nom::Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )));
        }

        // Extract the arguments string
        let args_str = &input[paren_pos + 1..close_paren_pos - 1];

        // Parse the arguments as a simple string value for now
        let args = if args_str.trim().is_empty() {
            vec![]
        } else {
            vec![Value::String(args_str.trim().to_string())]
        };

        Ok((&input[close_paren_pos..], (generator, args)))
    }

    fn parse_object(input: &str) -> IResult<&str, HashMap<String, Value>> {
        delimited(
            ws(char('{')),
            map(
                separated_list0(
                    ws(char(',')),
                    separated_pair(
                        ws(map(identifier, |s: &str| s.to_string())),
                        ws(char(':')),
                        ws(parse_value),
                    ),
                ),
                |pairs| {
                    let mut map = HashMap::new();
                    for (key, value) in pairs {
                        map.insert(key, value);
                    }
                    map
                },
            ),
            ws(char('}')),
        )(input)
    }

    fn parse_array(input: &str) -> IResult<&str, Vec<Value>> {
        delimited(
            ws(char('[')),
            separated_list0(ws(char(',')), ws(parse_value)),
            ws(char(']')),
        )(input)
    }

    fn parse_severity(input: &str) -> IResult<&str, LogSeverity> {
        map_res(
            delimited(
                char('"'),
                take_while1(|c: char| c.is_alphabetic()),
                char('"'),
            ),
            |s: &str| LogSeverity::from_str(s),
        )(input)
    }

    pub fn parse_template(input: &str) -> IResult<&str, Template> {
        delimited(
            ws(char('{')),
            map(
                tuple((
                    ws(preceded(ws(tag("severity:")), ws(parse_severity))),
                    opt(ws(char(','))),
                    ws(preceded(ws(tag("message:")), ws(string_literal))),
                    opt(ws(char(','))),
                    opt(ws(preceded(
                        ws(tag("weight:")),
                        ws(map_res(digit1, |s: &str| s.parse::<u32>())),
                    ))),
                    opt(ws(char(','))),
                    opt(ws(preceded(ws(tag("when:")), ws(string_literal)))),
                )),
                |(severity, _, message, _, weight, _, condition)| Template {
                    severity,
                    message,
                    weight,
                    condition,
                },
            ),
            ws(char('}')),
        )(input)
    }

    fn parse_flow_step(input: &str) -> IResult<&str, FlowStep> {
        delimited(
            ws(char('{')),
            map(
                tuple((
                    ws(preceded(ws(tag("severity:")), ws(parse_severity))),
                    opt(ws(char(','))),
                    ws(preceded(ws(tag("message:")), ws(string_literal))),
                    opt(ws(char(','))),
                    opt(ws(preceded(ws(tag("delay:")), ws(string_literal)))),
                    opt(ws(char(','))),
                    opt(ws(preceded(ws(tag("probability:")), ws(number_literal)))),
                    opt(ws(char(','))),
                    opt(ws(preceded(ws(tag("when:")), ws(string_literal)))),
                    opt(ws(char(','))),
                    opt(ws(preceded(ws(tag("data:")), ws(parse_object)))),
                )),
                |(severity, _, message, _, delay_str, _, probability, _, condition, _, data)| {
                    let delay = delay_str.map(|s| {
                        MusterDuration::from_str(&s)
                            .unwrap_or(MusterDuration::Fixed(StdDuration::from_millis(0)))
                    });

                    FlowStep {
                        severity,
                        message,
                        delay,
                        probability,
                        condition,
                        data,
                    }
                },
            ),
            ws(char('}')),
        )(input)
    }

    fn parse_flow(input: &str) -> IResult<&str, Flow> {
        let (input, name) = ws(identifier)(input)?;
        let (input, _) = ws(char(':'))(input)?;
        let (input, _) = ws(char('['))(input)?;

        // Parse flow steps
        let (input, steps) = separated_list1(ws(char(',')), ws(parse_flow_step))(input)?;

        // Parse closing bracket
        let (input, _) = ws(char(']'))(input)?;

        Ok((
            input,
            Flow {
                name: name.to_string(),
                steps,
            },
        ))
    }

    fn parse_time_pattern(input: &str) -> IResult<&str, (String, TimePattern)> {
        let (input, name) = ws(map(identifier, |s: &str| s.to_string()))(input)?;
        let (input, _) = ws(char(':'))(input)?;
        let (input, _) = ws(char('{'))(input)?;

        // Parse schedule
        let (input, _) = ws(tag("schedule:"))(input)?;
        let (input, schedule) = ws(string_literal)(input)?;
        let (input, _) = opt(ws(char(',')))(input)?;

        // Parse multiplier
        let (input, _) = ws(tag("multiplier:"))(input)?;
        let (input, multiplier) = ws(number_literal)(input)?;
        let (input, _) = opt(ws(char(',')))(input)?;

        // Parse closing brace
        let (input, _) = ws(char('}'))(input)?;

        Ok((
            input,
            (
                name,
                TimePattern {
                    schedule,
                    multiplier,
                },
            ),
        ))
    }

    fn parse_output_config(input: &str) -> IResult<&str, OutputConfig> {
        let (input, _) = ws(char('{'))(input)?;

        let mut format = OutputFormat::Json;
        let mut destination = OutputDestination::Stdout;
        let mut file_path = None;
        let mut endpoint = None;
        let mut remaining_input = input;

        // Parse fields
        while let Ok((i, (key, _, value, comma))) = tuple((
            ws(map(identifier, |s: &str| s.to_string())),
            ws(char(':')),
            ws(alt((
                string_literal,
                map(identifier, |s: &str| s.to_string()),
            ))),
            opt(ws(char(','))),
        ))(remaining_input)
        {
            remaining_input = i;

            match key.as_str() {
                "format" => {
                    format = match value.to_lowercase().as_str() {
                        "json" => OutputFormat::Json,
                        "text" => OutputFormat::PlainText,
                        "plaintext" => OutputFormat::PlainText,
                        _ => OutputFormat::Json,
                    };
                }
                "destination" => {
                    destination = match value.to_lowercase().as_str() {
                        "stdout" => OutputDestination::Stdout,
                        "file" => {
                            if let Some(path) = file_path.clone() {
                                OutputDestination::File(path)
                            } else {
                                OutputDestination::File(String::new())
                            }
                        }
                        "http" => {
                            if let Some(url) = endpoint.clone() {
                                OutputDestination::Http(url)
                            } else {
                                OutputDestination::Http(String::new())
                            }
                        }
                        _ => OutputDestination::Stdout,
                    };
                }
                "file_path" => {
                    file_path = Some(value.clone());
                    if let OutputDestination::File(_) = destination {
                        destination = OutputDestination::File(value);
                    }
                }
                "endpoint" => {
                    endpoint = Some(value.clone());
                    if let OutputDestination::Http(_) = destination {
                        destination = OutputDestination::Http(value);
                    }
                }
                _ => {}
            }

            // If no comma, we're done
            if comma.is_none() {
                break;
            }
        }

        let (input, _) = ws(char('}'))(remaining_input)?;

        Ok((
            input,
            OutputConfig {
                format,
                destination,
            },
        ))
    }

    fn parse_context(input: &str) -> IResult<&str, Context> {
        let (input, _) = ws(char('{'))(input)?;

        let mut fields = HashMap::new();
        let mut persistent = false;
        let mut remaining_input = input;

        // Parse fields
        while let Ok((i, (key, _, value, comma))) = tuple((
            ws(map(identifier, |s: &str| s.to_string())),
            ws(char(':')),
            ws(alt((
                map(tag("true"), |_| Value::Boolean(true)),
                map(tag("false"), |_| Value::Boolean(false)),
                parse_value,
            ))),
            opt(ws(char(','))),
        ))(remaining_input)
        {
            remaining_input = i;

            // Handle persistent field specially
            if key == "persistent" {
                if let Value::Boolean(p) = value {
                    persistent = p;
                }
            } else {
                fields.insert(key, value);
            }

            // If no comma, we're done
            if comma.is_none() {
                break;
            }
        }

        let (input, _) = ws(char('}'))(remaining_input)?;

        Ok((input, Context { fields, persistent }))
    }

    fn parse_logs_block(input: &str) -> IResult<&str, LogsBlock> {
        let (input, _) = ws(tag("logs"))(input)?;
        let (input, _) = ws(char('{'))(input)?;

        // Parse application_name
        let (input, _) = ws(tag("application_name:"))(input)?;
        let (input, app_name) = ws(string_literal)(input)?;
        let (input, _) = opt(ws(char(',')))(input)?;

        // Parse duration
        let (input, _) = ws(tag("duration:"))(input)?;
        let (input, duration_str) =
            ws(take_while1(|c: char| c.is_alphanumeric() || c == '_'))(input)?;
        let duration = MusterDuration::from_str(duration_str).map_err(|_| {
            nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))
        })?;
        let (input, _) = opt(ws(char(',')))(input)?;

        // Parse frequency
        let (input, _) = ws(tag("frequency:"))(input)?;
        let (input, frequency_str) = ws(digit1)(input)?;
        let frequency = frequency_str.parse::<u64>().map_err(|_| {
            nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::MapRes))
        })?;
        let (input, _) = opt(ws(char(',')))(input)?;

        // Parse templates
        let (input, _) = ws(tag("templates:"))(input)?;
        let (input, _) = ws(char('('))(input)?;
        let (input, templates) = separated_list1(ws(char(',')), ws(parse_template))(input)?;
        let (input, _) = ws(char(')'))(input)?;
        let (input, _) = opt(ws(char(',')))(input)?;

        // Parse optional flows
        let (mut input, mut flows) = (input, None);
        if let Ok((i, _)) = tag::<_, _, nom::error::Error<_>>("flows:")(input.trim()) {
            input = i;
            let (i, _) = ws(char('('))(input)?;
            let (i, f) = separated_list1(ws(char(',')), ws(parse_flow))(i)?;
            let (i, _) = ws(char(')'))(i)?;
            let (i, _) = opt(ws(char(',')))(i)?;
            input = i;
            flows = Some(f);
        }

        // Parse data
        let (input, _) = ws(tag("data:"))(input)?;
        let (input, _) = ws(char('('))(input)?;

        let mut data = HashMap::new();
        let mut remaining_input = input;

        // Parse data fields
        while let Ok((i, (key, _, value, comma))) = tuple((
            ws(map(identifier, |s: &str| s.to_string())),
            ws(char(':')),
            ws(parse_value),
            opt(ws(char(','))),
        ))(remaining_input)
        {
            remaining_input = i;
            data.insert(key, value);

            // If no comma, we're done
            if comma.is_none() {
                break;
            }
        }

        let (remaining_input, _) = ws(char(')'))(remaining_input)?;
        let (remaining_input, _) = opt(ws(char(',')))(remaining_input)?;

        // Parse optional context
        let (mut remaining_input, mut context) = (remaining_input, None);
        if let Ok((i, _)) = tag::<_, _, nom::error::Error<_>>("context:")(remaining_input.trim()) {
            remaining_input = i;
            let (i, c) = ws(parse_context)(i)?;
            let (i, _) = opt(ws(char(',')))(i)?;
            remaining_input = i;
            context = Some(c);
        }

        // Parse optional patterns
        let (mut remaining_input, mut patterns) = (remaining_input, None);
        if let Ok((i, _)) = tag::<_, _, nom::error::Error<_>>("patterns:")(remaining_input.trim()) {
            remaining_input = i;
            let (_i, _) = ws(char('{'))(remaining_input)?;

            let mut pattern_map = HashMap::new();
            let mut remaining = _i;

            // Parse multiple patterns
            while let Ok((i, pattern)) = ws(parse_time_pattern)(remaining) {
                let (name, pattern_value) = pattern;
                pattern_map.insert(name, pattern_value);

                // Check for comma
                if let Ok((i, _)) = ws(char(','))(i) {
                    remaining = i;
                } else {
                    remaining = i;
                    break;
                }
            }

            let (i, _) = ws(char('}'))(remaining)?;
            let (i, _) = opt(ws(char(',')))(i)?;
            remaining_input = i;
            patterns = Some(pattern_map);
        }

        // Parse optional output
        let (mut remaining_input, mut output) = (remaining_input, None);
        if let Ok((i, _)) = tag::<_, _, nom::error::Error<_>>("output:")(remaining_input.trim()) {
            remaining_input = i;
            let (i, o) = ws(parse_output_config)(i)?;
            let (i, _) = opt(ws(char(',')))(i)?;
            remaining_input = i;
            output = Some(o);
        }

        // Parse closing brace
        let (remaining_input, _) = ws(char('}'))(remaining_input)?;

        Ok((
            remaining_input,
            LogsBlock {
                application_name: app_name,
                duration,
                frequency,
                templates,
                flows,
                data,
                context,
                patterns,
                output,
            },
        ))
    }

    pub fn parse_muster(input: &str) -> Result<Muster, String> {
        println!("Parsing muster: {:?}", input);

        // Trim whitespace
        let input = input.trim();

        // Try to parse multiple logs blocks
        let mut logs_blocks = Vec::new();
        let mut remaining = input;

        while !remaining.trim().is_empty() {
            // Look for "logs {" pattern
            if let Some(logs_start) = remaining.find("logs {") {
                // Skip to the start of the logs block
                remaining = &remaining[logs_start..];

                // Find the matching closing brace
                let mut brace_count = 0;
                let mut logs_end = 0;
                let mut in_string = false;
                let mut escape_next = false;

                for (i, c) in remaining.char_indices() {
                    if escape_next {
                        escape_next = false;
                        continue;
                    }

                    match c {
                        '\\' if in_string => escape_next = true,
                        '"' => in_string = !in_string,
                        '{' if !in_string => brace_count += 1,
                        '}' if !in_string => {
                            brace_count -= 1;
                            if brace_count == 0 {
                                logs_end = i + 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                }

                if logs_end > 0 {
                    // Extract the logs block
                    let logs_block_str = &remaining[..logs_end];

                    // Parse the logs block
                    match parse_logs_block(logs_block_str) {
                        Ok((_, logs_block)) => {
                            logs_blocks.push(logs_block);
                            remaining = &remaining[logs_end..];
                        }
                        Err(e) => {
                            println!("Parser error: {:?}", e);
                            return Err(format!("Failed to parse Muster: {:?}", e));
                        }
                    }
                } else {
                    // No matching closing brace found
                    println!("Parser error: No matching closing brace found");
                    return Err(
                        "Failed to parse Muster: No matching closing brace found".to_string()
                    );
                }
            } else {
                // No more logs blocks found
                break;
            }
        }

        if logs_blocks.is_empty() {
            println!("Parser error: No logs blocks found");
            return Err("Failed to parse Muster: No logs blocks found".to_string());
        }

        Ok(Muster { logs_blocks })
    }
}

// Include tests from separate file
#[cfg(test)]
mod tests;
