#![allow(dead_code)]
use std::fmt;

#[derive(Debug, Default, Clone)]
pub struct OmniCommand {
    pub name: String,
    pub args: Vec<OmniCommandArg>,
}

impl OmniCommand {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn with_arg(mut self, arg: OmniCommandArg) -> Self {
        self.args.push(arg);
        self
    }

    pub fn add_arg(&mut self, arg: OmniCommandArg) -> &mut Self {
        self.args.push(arg);
        self
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for OmniCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)?;
        for arg in &self.args {
            for part in arg.to_os_args() {
                if part.contains(' ') {
                    write!(f, " \"{}\"", part)?;
                } else {
                    write!(f, " {}", part)?;
                }
            }
        }
        Ok(())
    }
}


#[derive(Debug, Default, Clone)]
pub struct OmniCommandArg {
    pub prefix: Option<String>,
    pub name: String,
    pub separator: Option<String>,
    pub value: Option<String>,
}

impl OmniCommandArg {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    pub fn with_separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = Some(separator.into());
        self
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn get_prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_separator(&self) -> Option<&str> {
        self.separator.as_deref()
    }

    pub fn get_value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    pub fn get_argument(&self) -> String {
        self.to_string()
    }
    pub fn to_os_args(&self) -> Vec<String> {
        let flag = format!(
            "{}{}",
            self.prefix.as_deref().unwrap_or(""),
            self.name
        );

        match (&self.separator, &self.value) {
            (Some(sep), Some(val)) => {
                vec![format!("{}{}{}", flag, sep, val)]
            }
            (None, Some(val)) => {
                vec![flag, val.clone()]
            }
            _ => vec![flag],
        }
    }
}

impl fmt::Display for OmniCommandArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_os_args().join(" "))
    }
}