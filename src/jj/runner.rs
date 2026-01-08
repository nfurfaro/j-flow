use anyhow::{Context, Result};
use std::process::Command;

/// Trait for executing shell commands, allowing for mocking in tests
pub trait CommandRunner: Send + Sync {
    /// Run a command and return its stdout as a string
    fn run(&self, program: &str, args: &[&str]) -> Result<String>;

    /// Run a command and check if it succeeds (for existence checks)
    fn run_success(&self, program: &str, args: &[&str]) -> bool {
        self.run(program, args).is_ok()
    }
}

/// Real command runner that executes actual shell commands
#[derive(Default)]
pub struct RealRunner;

impl CommandRunner for RealRunner {
    fn run(&self, program: &str, args: &[&str]) -> Result<String> {
        let output = Command::new(program)
            .args(args)
            .output()
            .with_context(|| format!("Failed to execute {} command", program))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("{} command failed: {}", program, stderr);
        }

        Ok(String::from_utf8(output.stdout)?)
    }
}

#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    /// Mock runner for testing - returns pre-configured responses
    pub struct MockRunner {
        /// Map from (program, args) to response
        responses: Mutex<HashMap<String, Result<String, String>>>,
        /// Track which commands were called
        calls: Mutex<Vec<(String, Vec<String>)>>,
    }

    impl MockRunner {
        pub fn new() -> Self {
            Self {
                responses: Mutex::new(HashMap::new()),
                calls: Mutex::new(Vec::new()),
            }
        }

        /// Add a mock response for a command
        /// Key format: "program arg1 arg2 ..."
        pub fn mock_response(&self, key: &str, response: &str) {
            self.responses
                .lock()
                .unwrap()
                .insert(key.to_string(), Ok(response.to_string()));
        }

        /// Add a mock error for a command
        pub fn mock_error(&self, key: &str, error: &str) {
            self.responses
                .lock()
                .unwrap()
                .insert(key.to_string(), Err(error.to_string()));
        }

        /// Get all commands that were called
        pub fn get_calls(&self) -> Vec<(String, Vec<String>)> {
            self.calls.lock().unwrap().clone()
        }

        /// Check if a specific command was called
        pub fn was_called(&self, program: &str, args: &[&str]) -> bool {
            let calls = self.calls.lock().unwrap();
            calls.iter().any(|(p, a)| {
                p == program && a.iter().map(|s| s.as_str()).collect::<Vec<_>>() == args
            })
        }
    }

    impl Default for MockRunner {
        fn default() -> Self {
            Self::new()
        }
    }

    impl CommandRunner for MockRunner {
        fn run(&self, program: &str, args: &[&str]) -> Result<String> {
            // Record the call
            self.calls.lock().unwrap().push((
                program.to_string(),
                args.iter().map(|s| s.to_string()).collect(),
            ));

            // Build the key
            let key = std::iter::once(program)
                .chain(args.iter().copied())
                .collect::<Vec<_>>()
                .join(" ");

            // Look up response
            let responses = self.responses.lock().unwrap();
            match responses.get(&key) {
                Some(Ok(response)) => Ok(response.clone()),
                Some(Err(error)) => anyhow::bail!("{}", error),
                None => anyhow::bail!("No mock response configured for: {}", key),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_mock_runner_response() {
            let runner = MockRunner::new();
            runner.mock_response("jj status", "nothing to commit");

            let result = runner.run("jj", &["status"]).unwrap();
            assert_eq!(result, "nothing to commit");
        }

        #[test]
        fn test_mock_runner_error() {
            let runner = MockRunner::new();
            runner.mock_error("jj status", "not a jj repo");

            let result = runner.run("jj", &["status"]);
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("not a jj repo"));
        }

        #[test]
        fn test_mock_runner_tracks_calls() {
            let runner = MockRunner::new();
            runner.mock_response("jj log", "commit 1");
            runner.mock_response("jj status", "clean");

            let _ = runner.run("jj", &["log"]);
            let _ = runner.run("jj", &["status"]);

            assert!(runner.was_called("jj", &["log"]));
            assert!(runner.was_called("jj", &["status"]));
            assert!(!runner.was_called("jj", &["push"]));
        }

        #[test]
        fn test_mock_runner_no_response_configured() {
            let runner = MockRunner::new();
            let result = runner.run("jj", &["unknown"]);
            assert!(result.is_err());
            assert!(result
                .unwrap_err()
                .to_string()
                .contains("No mock response configured"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_real_runner_echo() {
        let runner = RealRunner;
        let result = runner.run("echo", &["hello"]).unwrap();
        assert_eq!(result.trim(), "hello");
    }

    #[test]
    fn test_real_runner_failure() {
        let runner = RealRunner;
        let result = runner.run("false", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_real_runner_nonexistent_command() {
        let runner = RealRunner;
        let result = runner.run("nonexistent_command_xyz", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_run_success() {
        let runner = RealRunner;
        assert!(runner.run_success("true", &[]));
        assert!(!runner.run_success("false", &[]));
    }
}
