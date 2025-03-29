use std::time::Duration;

use tokio::task::JoinHandle;

use crate::config::{Config, Count, Severity, Task};

#[derive(Debug)]
pub enum LogRunnerError {
    InvalidFrequency(String),
    JoinError(tokio::task::JoinError),
}

impl From<tokio::task::JoinError> for LogRunnerError {
    fn from(err: tokio::task::JoinError) -> Self {
        LogRunnerError::JoinError(err)
    }
}

impl std::fmt::Display for LogRunnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl std::error::Error for LogRunnerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn std::error::Error> {
        self.source()
    }
}

pub struct LogRunner {
    config: Config,
}

impl LogRunner {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub async fn run(&self) -> Result<(), LogRunnerError> {
        let mut handles = Vec::new();
        for task in self.config.tasks.iter() {
            let task = task.clone();
            match task.count {
                Count::Amount(_) => {
                    handles.push(self.run_frequency_task(task.clone()).await);
                }
                Count::Const(_) => {
                    handles.push(self.run_infinite_task(task.clone()).await);
                }
            }
        }
        for handle in handles {
            let result = handle.await?;
            if let Err(e) = result {
                return Err(e);
            }
        }
        Ok(())
    }

    async fn run_frequency_task(&self, task: Task) -> JoinHandle<Result<(), LogRunnerError>> {
        return tokio::spawn(async move {
            let count_target = match task.count {
                Count::Amount(amount) => amount,
                Count::Const(_) => {
                    return Err(LogRunnerError::InvalidFrequency(format!(
                        "Expected Amount, got {}",
                        task.frequency
                    )))
                }
            };
            let mut interval = tokio::time::interval(Duration::from_millis(task.frequency));
            let mut index = 0;
            let mut count = 0;
            loop {
                interval.tick().await;
                print_task(&task, index);
                index += 1;
                if index >= task.vars.len() {
                    index = 0;
                }
                count += 1;
                if count >= count_target {
                    break;
                }
            }
            Ok(())
        });
    }

    async fn run_infinite_task(&self, task: Task) -> JoinHandle<Result<(), LogRunnerError>> {
        return tokio::spawn(async move {
            if task.count != Count::Const("Infinite".to_string()) {
                return Err(LogRunnerError::InvalidFrequency(format!(
                    "Expected Infinite, got {}",
                    task.frequency
                )));
            }
            let mut interval = tokio::time::interval(Duration::from_millis(task.frequency));
            let mut index = 0;
            loop {
                interval.tick().await;
                print_task(&task, index);
                index += 1;
                if index >= task.vars.len() {
                    index = 0;
                }
            }
        });
    }
}

fn print_task(task: &Task, index: usize) {
    let templated_string = interpolate(&task, index);
    match task.severity {
        Severity::Info => {
            tracing::info!(app_name = task.name, "{}", templated_string);
        }
        Severity::Error => {
            tracing::error!(app_name = task.name, "{}", templated_string);
        }
    }
}

fn interpolate(task: &Task, index: usize) -> String {
    let templated_string = task.template.replace("%s", &task.vars[index]);
    return templated_string;
}
