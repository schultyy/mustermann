use serde::Deserialize;
use std::fs::File;
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub tasks: Vec<Task>,
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
        tasks:
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
        tasks:
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

    #[test]
    fn test_config_parse() {
        let config = serde_yaml::from_str::<Config>(&single_task_config()).unwrap();
        assert_eq!(config.tasks.len(), 1);
        assert_eq!(config.tasks[0].name, "App Login Errors");
        assert_eq!(config.tasks[0].frequency, 45);
        assert_eq!(config.tasks[0].count, Count::Amount(10));
        assert_eq!(config.tasks[0].template, "Failed to login: %s");
        assert_eq!(
            config.tasks[0].vars,
            vec![
                "Invalid username or password",
                "Upstream connection refused"
            ]
        );
        assert_eq!(config.tasks[0].severity, Severity::Error);
    }

    #[test]
    fn test_config_parse_infinite_frequency() {
        let config = serde_yaml::from_str::<Config>(&infinite_frequency_config()).unwrap();
        assert_eq!(config.tasks[0].frequency, 45);
        assert_eq!(config.tasks[0].count, Count::Const("Infinite".to_string()));
        assert_eq!(config.tasks[0].template, "User %s logged in");
        assert_eq!(config.tasks[0].vars, vec!["Franz Josef", "34", "Heinz"]);
        assert_eq!(config.tasks[0].severity, Severity::Info);
    }
}
