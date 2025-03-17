use fake::{faker, Fake};
use rand::Rng;
use std::collections::HashMap;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle as TokioJoinHandle;
use tokio::time::sleep;
use tracing::{debug, error, info, trace, warn};
use uuid::Uuid;

use crate::parser::{
    Context, Flow, FlowStep, LogSeverity, Muster, MusterDuration, Template, Value,
};

#[derive(Debug)]
pub enum VistorError {
    DurationNotImplemented,
    FlowsNotImplemented,
    ContextNotImplemented,
    TemplateEvaluationError(String),
    ValueEvaluationError(String),
}

pub struct Visitor<'a> {
    muster: &'a Muster,
    context: HashMap<String, HashMap<String, Value>>,
}

impl<'a> Visitor<'a> {
    pub(crate) fn new(muster: &'a Muster) -> Self {
        Self {
            muster,
            context: HashMap::new(),
        }
    }

    pub(crate) async fn run(&self) -> Result<(), VistorError> {
        debug!("Starting visitor execution");

        let mut handles: Vec<TokioJoinHandle<()>> = Vec::new();
        let mut has_indefinite = false;

        for logs_block in &self.muster.logs_blocks {
            trace!(
                "Processing logs block for application: {}",
                logs_block.application_name
            );

            // Clone data for the tasks
            let app_name = logs_block.application_name.clone();
            let templates = logs_block.templates.clone();
            let frequency = logs_block.frequency;
            let duration = logs_block.duration.clone();
            let data = logs_block.data.clone();

            // Handle flows if present
            if let Some(flows) = &logs_block.flows {
                info!("Processing {} flows for {}", flows.len(), app_name);
                for flow in flows {
                    let flow_clone = flow.clone();
                    let app_name_clone = app_name.clone();
                    let data_clone = data.clone();

                    // Spawn a task for each flow
                    let handle = tokio::spawn(async move {
                        Self::process_flow(&app_name_clone, &flow_clone, &data_clone).await;
                    });
                    handles.push(handle);
                }
            }

            // Process regular templates
            info!("Processing {} templates for {}", templates.len(), app_name);

            match duration {
                MusterDuration::Indefinite => {
                    has_indefinite = true;
                    // For indefinite duration, spawn a task that will run until cancelled
                    let app_name_clone = app_name.clone();
                    let templates_clone = templates.clone();
                    let data_clone = data.clone();

                    let handle = tokio::spawn(async move {
                        let interval_ms = frequency as u64;
                        loop {
                            Self::process_templates(&app_name_clone, &templates_clone, &data_clone);
                            sleep(Duration::from_millis(interval_ms)).await;
                        }
                    });
                    handles.push(handle);
                }
                MusterDuration::Fixed(duration) => {
                    // For fixed duration, spawn a task that will run for the specified duration
                    let app_name_clone = app_name.clone();
                    let templates_clone = templates.clone();
                    let data_clone = data.clone();

                    let handle = tokio::spawn(async move {
                        let interval_ms = frequency as u64;
                        let start = Instant::now();

                        while start.elapsed() < duration {
                            Self::process_templates(&app_name_clone, &templates_clone, &data_clone);
                            sleep(Duration::from_millis(interval_ms)).await;
                        }

                        info!(
                            "Finished processing templates for {} after {:?}",
                            app_name_clone, duration
                        );
                    });
                    handles.push(handle);
                }
            }
        }

        // If there are no indefinite tasks, we can just wait for all tasks to complete
        if !has_indefinite {
            for handle in handles {
                let _ = handle.await;
            }
        } else {
            // If there are indefinite tasks, we need to use a different approach to keep the program running
            // We'll create a channel to signal when to stop, and just wait for that signal
            let (tx, rx) = tokio::sync::oneshot::channel();

            // Set up a Ctrl+C handler
            let ctrl_c = tokio::spawn(async move {
                tokio::signal::ctrl_c()
                    .await
                    .expect("Failed to listen for ctrl-c event");
                info!("Received Ctrl+C, shutting down...");
                let _ = tx.send(());
            });

            // Wait for either the Ctrl+C signal or all fixed-duration tasks to complete
            tokio::select! {
                _ = rx => {
                    info!("Shutdown signal received, stopping all tasks");
                    for handle in handles {
                        handle.abort();
                    }
                }
                _ = async {
                    // Just wait indefinitely
                    loop {
                        sleep(Duration::from_secs(3600)).await; // Sleep for an hour and check again
                    }
                } => {}
            }

            ctrl_c.abort();
        }

        debug!("Visitor execution completed");
        Ok(())
    }

    fn process_templates(app_name: &str, templates: &[Template], data: &HashMap<String, Value>) {
        let mut rng = rand::thread_rng();

        // Create a mutable clone of the data for this iteration
        let mut iteration_data = data.clone();

        // Generate all fake data for this iteration
        for (_key, value) in iteration_data.iter_mut() {
            match value {
                Value::FakeGenerator { generator, args } => {
                    // Replace the generator with an actual value
                    let fake_value = Self::generate_fake_data(generator, args);
                    *value = Value::String(fake_value);
                }
                Value::Object(obj) => {
                    // Handle nested objects with fake generators
                    for (_, subvalue) in obj.iter_mut() {
                        if let Value::FakeGenerator { generator, args } = subvalue {
                            let fake_value = Self::generate_fake_data(generator, args);
                            *subvalue = Value::String(fake_value);
                        }
                    }
                }
                _ => {}
            }
        }

        // Select a template weighted by the weight field if present
        let template = if templates.iter().any(|t| t.weight.is_some()) {
            // Weight-based selection using weighted random selection
            let total_weight: u32 = templates.iter().map(|t| t.weight.unwrap_or(1)).sum();

            // Generate a random value between 0 and the total weight
            let random_value = rng.gen_range(0..total_weight);

            // Find the template that corresponds to the random value
            let mut cumulative_weight = 0;
            let mut selected = &templates[0];

            for template in templates {
                let weight = template.weight.unwrap_or(1);
                cumulative_weight += weight;

                if random_value < cumulative_weight {
                    selected = template;
                    break;
                }
            }

            selected
        } else {
            // Random selection if no weights
            &templates[rng.gen_range(0..templates.len())]
        };

        // Check if condition is met (if present)
        if let Some(condition) = &template.condition {
            // For a basic implementation, we'll check if the condition values exist in our data
            // In a real implementation, this would parse and evaluate the condition like "$payment.status == 'declined'"
            trace!(app_name = app_name, "Evaluating condition: {}", condition);

            // Simplified condition evaluation - check if the field exists and is "declined"
            // This is a very basic implementation that just checks for status = declined
            if condition.contains("==") {
                let parts: Vec<&str> = condition.split("==").collect();
                if parts.len() == 2 {
                    let field_ref = parts[0].trim();
                    let expected_value = parts[1].trim().trim_matches('\'').trim_matches('"');

                    // Extract field path from something like $payment.status
                    if field_ref.starts_with('$') {
                        let field_path = &field_ref[1..]; // Remove the $
                        let path_parts: Vec<&str> = field_path.split('.').collect();

                        if path_parts.len() == 2 {
                            let object_name = path_parts[0];
                            let field_name = path_parts[1];

                            // Check if the field exists and matches the expected value
                            if let Some(Value::Object(obj)) = iteration_data.get(object_name) {
                                if let Some(value) = obj.get(field_name) {
                                    let value_str = Self::value_to_string(value);
                                    if value_str != expected_value {
                                        // Condition not met, skip this template
                                        trace!(
                                            app_name = app_name,
                                            "Condition not met: {} != {}",
                                            value_str,
                                            expected_value
                                        );
                                        return;
                                    }
                                    // Otherwise, condition is met, continue with this template
                                    trace!(
                                        app_name = app_name,
                                        "Condition met: {} == {}",
                                        value_str,
                                        expected_value
                                    );
                                } else {
                                    // Field doesn't exist, skip
                                    trace!(
                                        app_name = app_name,
                                        "Field doesn't exist: {}.{}",
                                        object_name,
                                        field_name
                                    );
                                    return;
                                }
                            } else {
                                // Object doesn't exist, skip
                                trace!(
                                    app_name = app_name,
                                    "Object doesn't exist: {}",
                                    object_name
                                );
                                return;
                            }
                        } else {
                            // Invalid path format, skip
                            trace!(app_name = app_name, "Invalid path format: {}", field_path);
                            return;
                        }
                    } else {
                        // Not a field reference, skip
                        trace!(app_name = app_name, "Not a field reference: {}", field_ref);
                        return;
                    }
                } else {
                    // Invalid condition format, skip
                    trace!(
                        app_name = app_name,
                        "Invalid condition format: {}",
                        condition
                    );
                    return;
                }
            } else {
                // Unsupported condition format, skip
                trace!(
                    app_name = app_name,
                    "Unsupported condition format: {}",
                    condition
                );
                return;
            }
        }

        // Evaluate the template message with data substitutions
        let message = Self::evaluate_message(&template.message, &iteration_data);

        // Log using the appropriate severity
        match template.severity {
            LogSeverity::Trace => trace!(app_name = app_name, "{}", message),
            LogSeverity::Debug => debug!(app_name = app_name, "{}", message),
            LogSeverity::Info => info!(app_name = app_name, "{}", message),
            LogSeverity::Warn => warn!(app_name = app_name, "{}", message),
            LogSeverity::Error => error!(app_name = app_name, "{}", message),
            LogSeverity::Fatal => error!(app_name = app_name, "FATAL: {}", message),
        }
    }

    async fn process_flow(app_name: &str, flow: &Flow, data: &HashMap<String, Value>) {
        info!(app_name = app_name, "Starting flow: {}", flow.name);

        // Create a base clone of the data for this flow
        let mut flow_data = data.clone();

        for step in &flow.steps {
            // Generate a new set of fake data for each step
            let mut step_data = flow_data.clone();

            // Generate all fake data for this step
            for (_, value) in step_data.iter_mut() {
                match value {
                    Value::FakeGenerator { generator, args } => {
                        // Replace the generator with an actual value
                        let fake_value = Self::generate_fake_data(generator, args);
                        *value = Value::String(fake_value);
                    }
                    Value::Object(obj) => {
                        // Handle nested objects with fake generators
                        for (_, subvalue) in obj.iter_mut() {
                            if let Value::FakeGenerator { generator, args } = subvalue {
                                let fake_value = Self::generate_fake_data(generator, args);
                                *subvalue = Value::String(fake_value);
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Check probability if specified
            if let Some(probability) = step.probability {
                let mut rng = rand::thread_rng();
                if rng.gen::<f64>() > probability {
                    trace!(app_name = app_name, "Skipping step due to probability");
                    continue;
                }
            }

            // Check condition if present
            if let Some(condition) = &step.condition {
                trace!(app_name = app_name, "Step has condition: {}", condition);
                // Skip for now since condition evaluation is not fully implemented
                continue;
            }

            // Handle delay if specified
            if let Some(delay) = &step.delay {
                match delay {
                    MusterDuration::Fixed(duration) => {
                        debug!(app_name = app_name, "Delaying for {:?}", duration);
                        sleep(*duration).await;
                    }
                    MusterDuration::Indefinite => {
                        error!(
                            app_name = app_name,
                            "Indefinite delay not supported in flow steps"
                        );
                        continue;
                    }
                }
            }

            // Merge flow step data with global data
            if let Some(step_specific_data) = &step.data {
                for (k, v) in step_specific_data {
                    // Generate fake data for step-specific data
                    match v {
                        Value::FakeGenerator { generator, args } => {
                            let fake_value = Self::generate_fake_data(generator, args);
                            step_data.insert(k.clone(), Value::String(fake_value));
                        }
                        _ => {
                            step_data.insert(k.clone(), v.clone());
                        }
                    }
                }
            }

            // Evaluate the message with data substitutions
            let message = Self::evaluate_message(&step.message, &step_data);

            // Log using the appropriate severity
            match step.severity {
                LogSeverity::Trace => trace!(app_name = app_name, flow = flow.name, "{}", message),
                LogSeverity::Debug => debug!(app_name = app_name, flow = flow.name, "{}", message),
                LogSeverity::Info => info!(app_name = app_name, flow = flow.name, "{}", message),
                LogSeverity::Warn => warn!(app_name = app_name, flow = flow.name, "{}", message),
                LogSeverity::Error => error!(app_name = app_name, flow = flow.name, "{}", message),
                LogSeverity::Fatal => {
                    error!(app_name = app_name, flow = flow.name, "FATAL: {}", message)
                }
            }
        }

        info!(app_name = app_name, "Completed flow: {}", flow.name);
    }

    fn evaluate_message(message: &str, data: &HashMap<String, Value>) -> String {
        let mut result = message.to_string();

        // Simple placeholder replacement
        // For each key in data, replace $key.subkey with the corresponding value
        for (key, value) in data.iter() {
            match value {
                Value::Object(obj) => {
                    for (subkey, subvalue) in obj.iter() {
                        let placeholder = format!("${}.{}", key, subkey);
                        if result.contains(&placeholder) {
                            let str_value = Self::value_to_string(subvalue);
                            result = result.replace(&placeholder, &str_value);
                        }
                    }
                }
                _ => {
                    let placeholder = format!("${}", key);
                    if result.contains(&placeholder) {
                        let str_value = Self::value_to_string(value);
                        result = result.replace(&placeholder, &str_value);
                    }
                }
            }
        }

        result
    }

    fn value_to_string(value: &Value) -> String {
        match value {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Boolean(b) => b.to_string(),
            Value::Reference(r) => format!("${}", r),
            Value::FakeGenerator { generator, args } => {
                // Now we'll try to generate fake data based on the generator name
                Self::generate_fake_data(generator, args)
            }
            Value::Object(_) => "[object]".to_string(),
            Value::Array(_) => "[array]".to_string(),
        }
    }

    fn generate_fake_data(generator: &str, args: &[Value]) -> String {
        // Parse generator in format like "fake::name::Name"
        let parts: Vec<&str> = generator.split("::").collect();
        if parts.len() < 3 || parts[0] != "fake" {
            return format!("<Invalid generator format: {}>", generator);
        }

        let category = parts[1];
        let function = parts[2];

        match (category, function) {
            // Name generators
            ("name", "Name") => faker::name::en::Name().fake::<String>(),
            ("name", "FirstName") => faker::name::en::FirstName().fake::<String>(),
            ("name", "LastName") => faker::name::en::LastName().fake::<String>(),

            // Internet generators
            ("internet", "Username") => faker::internet::en::Username().fake::<String>(),
            ("internet", "FreeEmail") => faker::internet::en::FreeEmail().fake::<String>(),
            ("internet", "SafeEmail") => faker::internet::en::SafeEmail().fake::<String>(),
            ("internet", "IPv4") => faker::internet::en::IPv4().fake::<String>(),
            ("internet", "IPv6") => faker::internet::en::IPv6().fake::<String>(),
            ("internet", "Password") => faker::internet::en::Password(8..15).fake::<String>(),
            ("internet", "UserAgent") => faker::internet::en::UserAgent().fake::<String>(),

            // Lorem generators
            ("lorem", "Word") => {
                // Generate a single random word
                faker::lorem::en::Word().fake::<String>()
            }
            ("lorem", "Words") => {
                // Generate multiple words
                // Default to 5 words if no argument is provided
                let count = match args.get(0) {
                    Some(Value::Number(n)) => *n as usize,
                    _ => 5,
                };

                // Generate words one at a time and join them
                let mut words = Vec::with_capacity(count);
                for _ in 0..count {
                    words.push(faker::lorem::en::Word().fake::<String>());
                }
                words.join(" ")
            }
            ("lorem", "Sentence") => {
                // Generate a sentence
                // Default to 5-15 words if no range is provided
                let word_count = match args.get(0) {
                    Some(Value::Number(n)) => *n as usize,
                    _ => 5 + rand::thread_rng().gen_range(0..10),
                };

                // Generate words one at a time
                let mut words = Vec::with_capacity(word_count);
                for _ in 0..word_count {
                    words.push(faker::lorem::en::Word().fake::<String>());
                }

                // Join words and add period
                let mut sentence = words.join(" ");
                // Capitalize first letter
                if !sentence.is_empty() {
                    let first_char = sentence
                        .chars()
                        .next()
                        .unwrap()
                        .to_uppercase()
                        .collect::<String>();
                    if let Some(rest) = sentence.get(1..) {
                        sentence = first_char + rest;
                    } else {
                        sentence = first_char;
                    }
                }
                sentence.push('.');
                sentence
            }

            // Number generators
            ("number", "Digit") => faker::number::en::Digit().fake::<String>(),
            ("number", "NumberWithFormat") => {
                let format = if !args.is_empty() {
                    if let Value::String(s) = &args[0] {
                        s.clone()
                    } else {
                        "###.##".to_string()
                    }
                } else {
                    "###.##".to_string()
                };
                faker::number::en::NumberWithFormat(&format).fake::<String>()
            }

            // Currency
            ("currency", "CurrencyCode") => faker::currency::en::CurrencyCode().fake::<String>(),
            ("currency", "CurrencyName") => faker::currency::en::CurrencyName().fake::<String>(),
            ("currency", "CurrencySymbol") => {
                faker::currency::en::CurrencySymbol().fake::<String>()
            }

            // UUID
            ("uuid", "UUIDv4") => Uuid::new_v4().to_string(),

            // Fallback
            _ => format!("<Unsupported generator: {}::{}>", category, function),
        }
    }

    fn evaluate_value(
        value: &Value,
        context: &HashMap<String, HashMap<String, Value>>,
    ) -> Result<Value, VistorError> {
        match value {
            Value::FakeGenerator { generator, args } => {
                // Generate fake data using our helper function
                let fake_value = Self::generate_fake_data(generator, args);
                Ok(Value::String(fake_value))
            }
            Value::Reference(reference) => {
                // Parse reference like "user.id"
                let parts: Vec<&str> = reference.split('.').collect();
                if parts.len() != 2 {
                    return Err(VistorError::ValueEvaluationError(format!(
                        "Invalid reference format: {}",
                        reference
                    )));
                }

                let namespace = parts[0];
                let field = parts[1];

                // Look up in context
                if let Some(namespace_data) = context.get(namespace) {
                    if let Some(value) = namespace_data.get(field) {
                        return Ok(value.clone());
                    }
                }

                Err(VistorError::ValueEvaluationError(format!(
                    "Reference not found: {}",
                    reference
                )))
            }
            // Other value types are returned as-is
            _ => Ok(value.clone()),
        }
    }
}
