use super::parser::parse_muster;
use super::*;

#[test]
fn test_parse_basic_muster() {
    let input = r#"
    logs {
        application_name: "checkout-service",
        duration: indefinite,
        frequency: 100,
        
        templates: (
            { severity: "INFO", message: "Performed checkout for $user.id" },
            { severity: "ERROR", message: "Checkout failed. Reason: $reason" }
        ),
        
        data: (
            user: {
                name: fake::name::Name(),
                id: fake::uuid::UUIDv4()
            },
            reason: fake::lorem::Sentence(1..2)
        )
    }
    "#;

    let result = parse_muster(input);
    assert!(
        result.is_ok(),
        "Failed to parse basic muster: {:?}",
        result.err()
    );

    let muster = result.unwrap();
    assert_eq!(muster.logs_blocks.len(), 1);

    let logs_block = &muster.logs_blocks[0];
    assert_eq!(logs_block.application_name, "checkout-service");
    assert_eq!(logs_block.duration, MusterDuration::Indefinite);
    assert_eq!(logs_block.frequency, 100);

    assert_eq!(logs_block.templates.len(), 2);
    assert_eq!(logs_block.templates[0].severity, LogSeverity::Info);
    assert_eq!(
        logs_block.templates[0].message,
        "Performed checkout for $user.id"
    );

    assert_eq!(logs_block.templates[1].severity, LogSeverity::Error);
    assert_eq!(
        logs_block.templates[1].message,
        "Checkout failed. Reason: $reason"
    );

    assert!(logs_block.data.contains_key("user"));
    assert!(logs_block.data.contains_key("reason"));
}

#[test]
fn test_parse_template_with_weight() {
    let input = r#"
    logs {
        application_name: "notification-service"
        duration: 30m
        frequency: 100
        
        templates: (
            { severity: "INFO", message: "Normal operation", weight: 10 },
            { severity: "WARN", message: "Slow database query", weight: 3 },
            { severity: "ERROR", message: "Connection failed", weight: 1 }
        )
        
        data: (
            service: "notification-service",
            version: "1.2.3"
        )
    }
    "#;

    let result = parse_muster(input);
    assert!(result.is_ok(), "Failed to parse muster: {:?}", result.err());

    let muster = result.unwrap();
    let logs_block = &muster.logs_blocks[0];

    assert_eq!(logs_block.templates.len(), 3);
    assert_eq!(logs_block.templates[0].weight, Some(10));
    assert_eq!(logs_block.templates[1].weight, Some(3));
    assert_eq!(logs_block.templates[2].weight, Some(1));
}

#[test]
fn test_parse_conditional_template() {
    let input = r#"
    logs {
        application_name: "payment-service",
        duration: 1h,
        frequency: 50,
        
        templates: (
            { severity: "INFO", message: "Payment processed" },
            { 
                severity: "ERROR", 
                message: "Payment declined: $transaction.decline_reason",
                when: "$transaction.status == 'declined'"
            }
        ),
        
        data: (
            transaction: {
                status: fake::utils::either("approved", "declined", "processing"),
                decline_reason: fake::lorem::Sentence(1..2)
            }
        )
    }
    "#;

    let result = parse_muster(input);
    assert!(result.is_ok(), "Failed to parse muster: {:?}", result.err());

    let muster = result.unwrap();
    let logs_block = &muster.logs_blocks[0];

    assert_eq!(logs_block.templates.len(), 2);
    assert_eq!(
        logs_block.templates[1].condition,
        Some("$transaction.status == 'declined'".to_string())
    );
}

#[test]
fn test_parse_flows() {
    let input = r#"
    logs {
        application_name: "user-service",
        duration: indefinite,
        frequency: 200,
        
        templates: (
            { severity: "INFO", message: "User action" }
        ),
        
        flows: (
            signup: [
                { severity: "INFO", message: "User registration started" },
                { severity: "INFO", message: "Validating email", delay: "200ms" },
                { 
                    severity: "ERROR", 
                    message: "Email validation failed: $error.message",
                    probability: 0.1,
                    data: { error: { message: fake::lorem::Sentence(1..2) } }
                },
                { 
                    severity: "INFO", 
                    message: "User registration completed",
                    when: "$error == null" 
                }
            ]
        ),
        
        data: (
            user: {
                email: fake::internet::SafeEmail()
            }
        )
    }
    "#;

    let result = parse_muster(input);
    assert!(result.is_ok(), "Failed to parse muster: {:?}", result.err());

    let muster = result.unwrap();
    let logs_block = &muster.logs_blocks[0];

    assert!(logs_block.flows.is_some());
    let flows = logs_block.flows.as_ref().unwrap();

    assert_eq!(flows.len(), 1);
    assert_eq!(flows[0].name, "signup");
    assert_eq!(flows[0].steps.len(), 4);

    let steps = &flows[0].steps;
    assert_eq!(steps[0].message, "User registration started");

    // Check delay
    assert!(steps[1].delay.is_some());

    // Check probability
    assert_eq!(steps[2].probability, Some(0.1));

    // Check condition
    assert_eq!(steps[3].condition, Some("$error == null".to_string()));
}

#[test]
fn test_parse_context() {
    let input = r#"
    logs {
        application_name: "request-service"
        duration: indefinite
        frequency: 100
        
        templates: (
            { severity: "INFO", message: "Request processed" }
        )
        
        data: (
            request: {
                id: fake::uuid::UUIDv4()
            }
        )
        
        context: {
            trace_id: fake::uuid::UUIDv4(),
            span_id: fake::uuid::UUIDv4(),
            user_session: "$user.id",
            persistent: true
        }
    }
    "#;

    let result = parse_muster(input);
    assert!(result.is_ok(), "Failed to parse muster: {:?}", result.err());

    let muster = result.unwrap();
    let logs_block = &muster.logs_blocks[0];

    assert!(logs_block.context.is_some());
    let context = logs_block.context.as_ref().unwrap();

    assert!(context.persistent);
    assert!(context.fields.contains_key("trace_id"));
    assert!(context.fields.contains_key("span_id"));
    assert!(context.fields.contains_key("user_session"));
}

#[test]
fn test_parse_patterns() {
    let input = r#"
    logs {
        application_name: "traffic-service"
        duration: 24h
        frequency: 100
        
        templates: (
            { severity: "INFO", message: "Request received" }
        )
        
        data: (
            request: {
                id: fake::uuid::UUIDv4()
            }
        )
        
        patterns: {
            business_hours: {
                schedule: "Mon-Fri 09:00-17:00",
                multiplier: 5.0
            },
            maintenance_window: {
                schedule: "Sat 02:00-04:00",
                multiplier: 0.2
            }
        }
    }
    "#;

    let result = parse_muster(input);
    assert!(result.is_ok(), "Failed to parse muster: {:?}", result.err());

    let muster = result.unwrap();
    let logs_block = &muster.logs_blocks[0];

    assert!(logs_block.patterns.is_some());
    let patterns = logs_block.patterns.as_ref().unwrap();

    assert_eq!(patterns.len(), 2);
    assert!(patterns.contains_key("business_hours"));
    assert!(patterns.contains_key("maintenance_window"));

    assert_eq!(patterns["business_hours"].schedule, "Mon-Fri 09:00-17:00");
    assert_eq!(patterns["business_hours"].multiplier, 5.0);

    assert_eq!(patterns["maintenance_window"].schedule, "Sat 02:00-04:00");
    assert_eq!(patterns["maintenance_window"].multiplier, 0.2);
}

#[test]
fn test_parse_output_config() {
    let input = r#"
    logs {
        application_name: "api-service"
        duration: indefinite
        frequency: 100
        
        templates: (
            { severity: "INFO", message: "API request" }
        )
        
        data: (
            request: {
                id: fake::uuid::UUIDv4()
            }
        )
        
        output: {
            format: "json",
            destination: "file",
            file_path: "api-logs.json"
        }
    }
    "#;

    let result = parse_muster(input);
    assert!(result.is_ok(), "Failed to parse muster: {:?}", result.err());

    let muster = result.unwrap();
    let logs_block = &muster.logs_blocks[0];

    assert!(logs_block.output.is_some());
    let output = logs_block.output.as_ref().unwrap();

    assert!(matches!(output.format, OutputFormat::Json));
    assert!(
        matches!(output.destination, OutputDestination::File(ref path) if path == "api-logs.json")
    );
}

#[test]
fn test_parse_multiple_logs_blocks() {
    let input = r#"
    logs {
        application_name: "api-gateway"
        duration: indefinite
        frequency: 100
        
        templates: (
            { severity: "INFO", message: "Gateway request" }
        )
        
        data: (
            request: {
                id: fake::uuid::UUIDv4()
            }
        )
    }
    
    logs {
        application_name: "payment-service"
        duration: indefinite
        frequency: 200
        
        templates: (
            { severity: "INFO", message: "Payment processed" }
        )
        
        data: (
            payment: {
                id: fake::uuid::UUIDv4()
            }
        )
    }
    "#;

    let result = parse_muster(input);
    assert!(result.is_ok(), "Failed to parse muster: {:?}", result.err());

    let muster = result.unwrap();
    assert_eq!(muster.logs_blocks.len(), 2);

    assert_eq!(muster.logs_blocks[0].application_name, "api-gateway");
    assert_eq!(muster.logs_blocks[0].frequency, 100);

    assert_eq!(muster.logs_blocks[1].application_name, "payment-service");
    assert_eq!(muster.logs_blocks[1].frequency, 200);
}

#[test]
fn test_parse_duration_formats() {
    let inputs = vec![
        ("indefinite", MusterDuration::Indefinite),
        ("5s", MusterDuration::Fixed(StdDuration::from_secs(5))),
        (
            "10m",
            MusterDuration::Fixed(StdDuration::from_secs(10 * 60)),
        ),
        (
            "2h",
            MusterDuration::Fixed(StdDuration::from_secs(2 * 3600)),
        ),
        (
            "1d",
            MusterDuration::Fixed(StdDuration::from_secs(1 * 86400)),
        ),
    ];

    for (input_str, expected) in inputs {
        let result = MusterDuration::from_str(input_str);
        assert!(result.is_ok(), "Failed to parse duration: {}", input_str);
        assert_eq!(result.unwrap(), expected);
    }
}

#[test]
fn test_parse_single_template() {
    use super::parser::parse_template;

    let input = r#"{ severity: "INFO", message: "Test message" }"#;
    let result = parse_template(input);
    println!("Template parse result: {:?}", result);
    assert!(
        result.is_ok(),
        "Failed to parse template: {:?}",
        result.err()
    );

    let template = result.unwrap().1;
    assert_eq!(template.severity, LogSeverity::Info);
    assert_eq!(template.message, "Test message");
}
