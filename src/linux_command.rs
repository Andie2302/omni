use std::{fmt, io};
use std::process::{Command, ExitStatus, Output, Stdio};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LinuxCommand {
    program_name: String,
    arguments: Vec<Argument>,
    env_vars: HashMap<String, String>,
    piped_into: Option<Box<LinuxCommand>>,
}

impl LinuxCommand {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            program_name: name.into(),
            arguments: Vec::new(),
            env_vars: HashMap::new(),
            piped_into: None,
        }
    }

    pub fn arg(mut self, arg: Argument) -> Self {
        self.arguments.push(arg);
        self
    }
    pub fn flag(self, prefix: impl Into<String>, key: impl Into<String>) -> Self {
        self.arg(Argument::flag(prefix, key))
    }
    pub fn opt(
        self,
        prefix: impl Into<String>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.arg(Argument::with_value(prefix, key, value))
    }
    pub fn opt_eq(
        self,
        prefix: impl Into<String>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        self.arg(Argument::with_eq_value(prefix, key, value))
    }
    pub fn positional(self, value: impl Into<String>) -> Self {
        self.arg(Argument::positional(value))
    }
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }
    pub fn pipe(mut self, next: LinuxCommand) -> Self {
        self.piped_into = Some(Box::new(next));
        self
    }
    pub fn execute(&self) -> io::Result<ExitStatus> {
        self.run_output().map(|o| o.status)
    }
    pub fn output_string(&self) -> io::Result<String> {
        let output = self.run_output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Command `{}` failed (exit {:?}): {}",
                    self.program_name,
                    output.status.code(),
                    stderr.trim()
                ),
            ));
        }

        String::from_utf8(output.stdout).map_err(|e| {
            io::Error::new(io::ErrorKind::InvalidData, format!("Invalid UTF-8 in stdout: {}", e))
        })
    }

    pub fn run_output(&self) -> io::Result<Output> {
        let mut cmd = self.build_command();
        if self.piped_into.is_some() {
            cmd.stdout(Stdio::piped());
            let child = cmd.spawn()?;
            let child_stdout = child.stdout.ok_or_else(|| {
                io::Error::new(io::ErrorKind::Other, "Failed to capture stdout for pipe")
            })?;

            let mut next_cmd = self.piped_into.as_ref().unwrap().build_command();
            next_cmd.stdin(child_stdout);
            next_cmd.output()
        } else {
            cmd.output()
        }
    }



    fn build_command(&self) -> Command {
        let mut cmd = Command::new(&self.program_name);

        for (k, v) in &self.env_vars {
            cmd.env(k, v);
        }

        for arg in &self.arguments {
            let prefix = arg.prefix.as_deref().unwrap_or("");

            match &arg.separator {
                Some(sep) if sep == "=" => {
                    let val = arg.value.as_deref().unwrap_or("");
                    cmd.arg(format!("{}{}{}{}", prefix, arg.key, sep, val));
                }
                _ if arg.key.is_empty() => {
                    if let Some(val) = &arg.value {
                        cmd.arg(val);
                    }
                }
                _ => {
                    cmd.arg(format!("{}{}", prefix, arg.key));
                    if let Some(val) = &arg.value {
                        cmd.arg(val);
                    }
                }
            }
        }

        cmd
    }
}

impl fmt::Display for LinuxCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (k, v) in &self.env_vars {
            write!(f, "{}={} ", k, v)?;
        }

        write!(f, "{}", self.program_name)?;

        for arg in &self.arguments {
            write!(f, " {}", arg)?;
        }

        if let Some(next) = &self.piped_into {
            write!(f, " | {}", next)?;
        }

        Ok(())
    }
}


#[derive(Debug, Clone)]
pub struct Argument {
    pub prefix: Option<String>,
    pub key: String,
    pub separator: Option<String>,
    pub value: Option<String>,
}

impl Argument {
    pub fn flag(prefix: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            prefix: Some(prefix.into()),
            key: key.into(),
            separator: None,
            value: None,
        }
    }
    pub fn with_value(
        prefix: impl Into<String>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            prefix: Some(prefix.into()),
            key: key.into(),
            separator: None,
            value: Some(value.into()),
        }
    }
    pub fn with_eq_value(
        prefix: impl Into<String>,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            prefix: Some(prefix.into()),
            key: key.into(),
            separator: Some("=".to_string()),
            value: Some(value.into()),
        }
    }
    pub fn positional(value: impl Into<String>) -> Self {
        Self {
            prefix: None,
            key: String::new(),
            separator: None,
            value: Some(value.into()),
        }
    }
    fn quoted_value(val: &str) -> String {
        if val.contains(' ') {
            format!("\"{}\"", val)
        } else {
            val.to_string()
        }
    }
}

impl fmt::Display for Argument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = self.prefix.as_deref().unwrap_or("");
        let key = &self.key;

        match &self.value {
            None => write!(f, "{}{}", prefix, key),
            Some(val) => {
                let sep = self.separator.as_deref().unwrap_or(" ");
                let formatted_val = Self::quoted_value(val);
                if sep == " " {
                    if key.is_empty() {
                        write!(f, "{}", formatted_val)
                    } else {
                        write!(f, "{}{} {}", prefix, key, formatted_val)
                    }
                } else {
                    write!(f, "{}{}{}{}", prefix, key, sep, formatted_val)
                }
            }
        }
    }
}
