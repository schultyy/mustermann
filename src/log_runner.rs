use std::time::Duration;

use tokio::task::JoinHandle;

use crate::config::{Config, Frequency, Task};

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
            match task.frequency {
                Frequency::Amount(_) => {
                    handles.push(self.run_frequency_task(task.clone()).await);
                }
                Frequency::Const(_) => {
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
            let frequency = match task.frequency {
                Frequency::Amount(amount) => amount,
                Frequency::Const(_) => {
                    return Err(LogRunnerError::InvalidFrequency(format!(
                        "Expected Amount, got {}",
                        task.frequency
                    )))
                }
            };
            let mut interval = tokio::time::interval(Duration::from_secs(frequency as u64));
            let mut index = 0;
            loop {
                interval.tick().await;
                let templated_string = interpolate(&task, index);
                println!("{}", templated_string);
                index += 1;
                if index >= task.vars.len() {
                    index = 0;
                }
            }
        });
    }

    async fn run_infinite_task(&self, task: Task) -> JoinHandle<Result<(), LogRunnerError>> {
        return tokio::spawn(async move {
            if task.frequency != Frequency::Const("Infinite".to_string()) {
                return Err(LogRunnerError::InvalidFrequency(format!(
                    "Expected Infinite, got {}",
                    task.frequency
                )));
            }
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            let mut index = 0;
            loop {
                interval.tick().await;
                println!("Running infinite task");
                let templated_string = interpolate(&task, index);
                println!("{}", templated_string);
                index += 1;
                if index >= task.vars.len() {
                    index = 0;
                }
            }
        });
    }
}

fn interpolate(task: &Task, index: usize) -> String {
    let templated_string = task.template.replace("%s", &task.vars[index]);
    return templated_string;
}
