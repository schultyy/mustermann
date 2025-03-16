# Mustermann Parser

This module contains a parser for the Mustermann domain-specific language (DSL) for generating test data for OpenTelemetry pipelines. The parser transforms Mustermann DSL text into an Abstract Syntax Tree (AST) for further processing.

## Structure

The parser module uses [nom](https://github.com/Geal/nom), a parser combinator library for Rust, to parse the Mustermann DSL.

### Main Components

- `mod.rs`: Contains the AST type definitions and the parser module
- `tests.rs`: Contains comprehensive tests for the parser

## AST Structure

The AST is structured to represent all the language constructs of the Mustermann DSL:

- `Muster`: The top-level container that holds multiple `LogsBlock` instances
- `LogsBlock`: Represents a single `logs` block with configuration, templates, data generators, etc.
- `Template`: Represents a log message template with severity, message text, and optional conditions
- `Flow`: Represents a sequence of log messages that simulate a process flow
- `FlowStep`: Represents a single step in a flow with its own severity, message, probability, etc.
- `TimePattern`: Represents a time-based pattern for log generation
- `OutputConfig`: Specifies how and where logs should be output
- `Context`: Provides context variables for log correlation

## Value Types

The `Value` enum represents different types of values that can be used in the DSL:

- `String`: A simple string value
- `Number`: A numeric value
- `Boolean`: A boolean value (true/false)
- `Reference`: A reference to another value (starts with `$`)
- `FakeGenerator`: A reference to a function in the fake-rs library
- `Object`: A collection of named values
- `Array`: A sequence of values

## Parser

The `parser` submodule contains the parsing logic using nom combinators:

- Helper functions for parsing whitespace, identifiers, literals, etc.
- Specific parsers for each language construct
- The main `parse_muster` function that parses an entire Mustermann file

## Usage

```rust
use mustermann::parser::parser::parse_muster;

fn main() {
    let input = r#"
    logs {
        application_name: "checkout-service"
        duration: indefinite
        frequency: 100

        templates: (
            { severity: "INFO", message: "Performed checkout for $user.id" },
            { severity: "ERROR", message: "Checkout failed. Reason: $reason" }
        )

        data: (
            user: {
                name: fake::name::Name(),
                id: fake::uuid::UUIDv4()
            },
            reason: fake::lorem::Sentence(1..2)
        )
    }
    "#;

    match parse_muster(input) {
        Ok(muster) => {
            println!("Successfully parsed muster with {} logs block(s)", muster.logs_blocks.len());
            // Process the AST...
        },
        Err(e) => {
            eprintln!("Error parsing muster: {}", e);
        }
    }
}
```

## Examples

Example Mustermann DSL files can be found in the `examples/` directory:

- `basic.muster`: A simple example with basic features
- `advanced.muster`: A complex example with flows, patterns, and context
- `specialized.muster`: Examples showcasing localization and different output formats

## Future Work

Current implementation only generates the AST. Future work will include:

1. An evaluator/interpreter to process the AST
2. Integration with fake-rs to generate the data
3. Output generation according to the specified format and destination
