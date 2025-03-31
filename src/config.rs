use serde::Deserialize;
use std::fs::File;
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub logs: Vec<Task>,
    pub services: Vec<Service>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Service {
    pub name: String,
    pub methods: Vec<Method>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Method {
    pub name: String,
    pub stdout: Option<String>,
    pub sleep_ms: Option<u64>,
    pub calls: Option<Vec<Call>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Call {
    pub name: String,
    pub method: String,
}

#[derive(Debug)]
pub enum ConfigError {
    Io(std::io::Error),
    Yaml(serde_yaml::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Io(e) => write!(f, "IO error: {}", e),
            ConfigError::Yaml(e) => write!(f, "YAML error: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub(crate) fn from_file(file_path: &str) -> Result<Self, ConfigError> {
        let file = File::open(file_path).map_err(ConfigError::Io)?;
        let config = serde_yaml::from_reader(file).map_err(ConfigError::Yaml)?;
        Ok(config)
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Count {
    Amount(u64),
    Const(String),
}
impl std::fmt::Display for Count {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Count::Amount(amount) => write!(f, "{}", amount),
            Count::Const(val) => write!(f, "{}", val),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub enum Severity {
    Info,
    Error,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Task {
    pub name: String,
    pub frequency: u64,
    pub count: Count,
    pub template: String,
    pub vars: Vec<String>,
    pub severity: Severity,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn single_task_config() -> String {
        "
        services: []
        logs:
          - name: App Login Errors
            frequency: 45
            count: 10
            template: \"Failed to login: %s\"
            vars:
              - Invalid username or password
              - Upstream connection refused
            severity: Error
          "
        .to_string()
    }

    fn infinite_frequency_config() -> String {
        "
        services: []
        logs:
        - name: App Logs
          frequency: 45
          count: Infinite
          template: \"User %s logged in\"
          vars:
            - Franz Josef
            - 34
            - Heinz
          severity: Info
        "
        .to_string()
    }

    fn services_config() -> String {
        "
        logs: []
        services:
            - name: payments
              methods:
                - name: charge
                  calls:
                    - name: checkout
                      method: process
              sleep_ms: 500
            - name: checkout
              methods:
                - name: process
                  stdout: Processing Order
        "
        .to_string()
    }

    #[test]
    fn test_config_parse() {
        let config = serde_yaml::from_str::<Config>(&single_task_config()).unwrap();
        assert_eq!(config.logs.len(), 1);
        assert_eq!(config.logs[0].name, "App Login Errors");
        assert_eq!(config.logs[0].frequency, 45);
        assert_eq!(config.logs[0].count, Count::Amount(10));
        assert_eq!(config.logs[0].template, "Failed to login: %s");
        assert_eq!(
            config.logs[0].vars,
            vec![
                "Invalid username or password",
                "Upstream connection refused"
            ]
        );
        assert_eq!(config.logs[0].severity, Severity::Error);
    }

    #[test]
    fn test_config_parse_infinite_frequency() {
        let config = serde_yaml::from_str::<Config>(&infinite_frequency_config()).unwrap();
        assert_eq!(config.logs[0].frequency, 45);
        assert_eq!(config.logs[0].count, Count::Const("Infinite".to_string()));
        assert_eq!(config.logs[0].template, "User %s logged in");
        assert_eq!(config.logs[0].vars, vec!["Franz Josef", "34", "Heinz"]);
        assert_eq!(config.logs[0].severity, Severity::Info);
    }

    #[test]
    fn test_config_parse_services() {
        let config = serde_yaml::from_str::<Config>(&services_config()).unwrap();
        assert_eq!(config.services.len(), 2);
        assert_eq!(config.services[0].name, "payments");
        assert_eq!(config.services[0].methods.len(), 1);
        assert_eq!(config.services[0].methods[0].name, "charge");
        assert_eq!(
            config.services[0].methods[0].calls.as_ref().unwrap()[0].name,
            "checkout"
        );
        assert_eq!(
            config.services[0].methods[0].calls.as_ref().unwrap()[0].method,
            "process"
        );
        assert_eq!(config.services[1].name, "checkout");
        assert_eq!(config.services[1].methods.len(), 1);
        assert_eq!(config.services[1].methods[0].name, "process");
        assert_eq!(
            config.services[1].methods[0].stdout,
            Some("Processing Order".to_string())
        );
    }
}
