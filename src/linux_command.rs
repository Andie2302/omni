use std::{fmt, io};
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::process::{Command, ExitStatus, Output, Stdio};

// ── Fehlertypen ───────────────────────────────────────────────────────────────

/// Präzise Fehlerursachen beim Ausführen von Befehlen und Pipe-Ketten.
#[derive(Debug)]
pub enum CommandError {
    /// Ein Prozess in der Pipe-Kette ist mit einem Fehler-Exit-Code beendet.
    PipeFailed {
        /// 0-basierter Index in der Pipe-Kette
        index: usize,
        /// Name des Programms, das fehlgeschlagen ist
        program: String,
        exit_code: Option<i32>,
    },
    /// Der letzte Befehl ist fehlgeschlagen.
    CommandFailed {
        program: String,
        exit_code: Option<i32>,
        stderr: String,
    },
    /// Ein I/O-Fehler beim Starten oder Lesen des Prozesses.
    Io(io::Error),
    /// Die Ausgabe enthielt kein gültiges UTF-8.
    InvalidUtf8(std::string::FromUtf8Error),
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PipeFailed { index, program, exit_code } => write!(
                f,
                "Pipe-Befehl #{} (`{}`) fehlgeschlagen (Exit {:?})",
                index + 1,
                program,
                exit_code
            ),
            Self::CommandFailed { program, exit_code, stderr } => write!(
                f,
                "Befehl `{}` fehlgeschlagen (Exit {:?}): {}",
                program,
                exit_code,
                stderr.trim()
            ),
            Self::Io(e) => write!(f, "I/O-Fehler: {e}"),
            Self::InvalidUtf8(e) => write!(f, "Ungültiges UTF-8 in stdout: {e}"),
        }
    }
}

impl std::error::Error for CommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::InvalidUtf8(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for CommandError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

// ── Ergebnis einer Pipe-Kette ─────────────────────────────────────────────────

/// Output des letzten Befehls sowie die Exit-Infos aller Prozesse in der Kette.
#[derive(Debug)]
pub struct PipeOutput {
    pub output: Output,
    /// (Programmname, ExitStatus) für jeden Prozess – erster = linkster Befehl.
    pub statuses: Vec<(String, ExitStatus)>,
}

// ── Argument-Enum ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Argument<'a> {
    /// `--verbose` / `-v`
    Flag { prefix: Cow<'a, str>, key: Cow<'a, str> },
    /// `--output file.txt` / `-o file.txt`
    Opt { prefix: Cow<'a, str>, key: Cow<'a, str>, value: Cow<'a, str> },
    /// `--output=file.txt`
    OptEq { prefix: Cow<'a, str>, key: Cow<'a, str>, value: Cow<'a, str> },
    /// `input.txt`
    Positional(Cow<'a, str>),
}

impl<'a> Argument<'a> {
    pub fn flag(prefix: impl Into<Cow<'a, str>>, key: impl Into<Cow<'a, str>>) -> Self {
        Self::Flag { prefix: prefix.into(), key: key.into() }
    }
    pub fn opt(
        prefix: impl Into<Cow<'a, str>>,
        key: impl Into<Cow<'a, str>>,
        value: impl Into<Cow<'a, str>>,
    ) -> Self {
        Self::Opt { prefix: prefix.into(), key: key.into(), value: value.into() }
    }
    pub fn opt_eq(
        prefix: impl Into<Cow<'a, str>>,
        key: impl Into<Cow<'a, str>>,
        value: impl Into<Cow<'a, str>>,
    ) -> Self {
        Self::OptEq { prefix: prefix.into(), key: key.into(), value: value.into() }
    }
    pub fn positional(value: impl Into<Cow<'a, str>>) -> Self {
        Self::Positional(value.into())
    }

    /// Shell-sicheres Quoting für die Display-Ausgabe (NICHT für `Command::arg`!).
    /// Verwendet Single-Quote-Escaping, das alle Sonderzeichen korrekt behandelt.
    /// Hinweis: `Command::arg` umgeht die Shell – dort darf NICHT gequotet werden.
    fn shell_quote(val: &str) -> String {
        if !val.chars().any(|c| {
            matches!(c, ' ' | '"' | '\'' | '\\' | '$' | '`' | '!' | '\n' | '\t')
        }) {
            return val.to_string();
        }
        // ' → '\''  (verlässt Single-Quote, fügt escaped ' ein, öffnet neu)
        format!("'{}'", val.replace('\'', r"'\''"))
    }
}

impl fmt::Display for Argument<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Argument::Flag { prefix, key } =>
                write!(f, "{prefix}{key}"),
            Argument::Opt { prefix, key, value } =>
                write!(f, "{prefix}{key} {}", Self::shell_quote(value)),
            Argument::OptEq { prefix, key, value } =>
                write!(f, "{prefix}{key}={}", Self::shell_quote(value)),
            Argument::Positional(value) =>
                write!(f, "{}", Self::shell_quote(value)),
        }
    }
}

// ── LinuxCommand ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LinuxCommand<'a> {
    program_name: Cow<'a, str>,
    arguments: Vec<Argument<'a>>,
    /// BTreeMap statt HashMap → deterministische Reihenfolge in Display und Tests.
    env_vars: BTreeMap<Cow<'a, str>, Cow<'a, str>>,
    piped_into: Option<Box<LinuxCommand<'a>>>,
}

impl<'a> LinuxCommand<'a> {
    pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
        Self {
            program_name: name.into(),
            arguments: Vec::new(),
            env_vars: BTreeMap::new(),
            piped_into: None,
        }
    }

    // ── Builder-Methoden (Consuming Builder) ──────────────────────────────────

    pub fn arg(mut self, arg: Argument<'a>) -> Self {
        self.arguments.push(arg);
        self
    }
    pub fn flag(self, prefix: impl Into<Cow<'a, str>>, key: impl Into<Cow<'a, str>>) -> Self {
        self.arg(Argument::flag(prefix, key))
    }
    pub fn opt(
        self,
        prefix: impl Into<Cow<'a, str>>,
        key: impl Into<Cow<'a, str>>,
        value: impl Into<Cow<'a, str>>,
    ) -> Self {
        self.arg(Argument::opt(prefix, key, value))
    }
    pub fn opt_eq(
        self,
        prefix: impl Into<Cow<'a, str>>,
        key: impl Into<Cow<'a, str>>,
        value: impl Into<Cow<'a, str>>,
    ) -> Self {
        self.arg(Argument::opt_eq(prefix, key, value))
    }
    pub fn positional(self, value: impl Into<Cow<'a, str>>) -> Self {
        self.arg(Argument::positional(value))
    }
    pub fn env(
        mut self,
        key: impl Into<Cow<'a, str>>,
        value: impl Into<Cow<'a, str>>,
    ) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }
    pub fn pipe(mut self, next: LinuxCommand<'a>) -> Self {
        self.piped_into = Some(Box::new(next));
        self
    }

    // ── Ausführung ────────────────────────────────────────────────────────────

    /// Führt den Befehl aus und gibt nur den Exit-Status zurück.
    pub fn execute(&self) -> Result<ExitStatus, CommandError> {
        self.run_output().map(|o| o.output.status)
    }

    /// Führt den Befehl aus und gibt stdout als `String` zurück.
    ///
    /// Verhält sich wie `set -o pipefail`: schlägt fehl, sobald *irgendein*
    /// Prozess in der Pipe-Kette einen Fehler-Exit-Code zurückgibt.
    /// Die Fehlermeldung enthält Programmname und Position in der Kette.
    pub fn output_string(&self) -> Result<String, CommandError> {
        let pipe_output = self.run_output()?;

        // Alle Zwischenprozesse auf Fehler prüfen (pipefail-Semantik)
        for (index, (program, status)) in pipe_output.statuses.iter().enumerate() {
            if !status.success() {
                return Err(CommandError::PipeFailed {
                    index,
                    program: program.clone(),
                    exit_code: status.code(),
                });
            }
        }

        // Letzter Prozess
        if !pipe_output.output.status.success() {
            let stderr = String::from_utf8_lossy(&pipe_output.output.stderr).into_owned();
            return Err(CommandError::CommandFailed {
                program: self.last_program_name().to_string(),
                exit_code: pipe_output.output.status.code(),
                stderr,
            });
        }

        String::from_utf8(pipe_output.output.stdout)
            .map_err(CommandError::InvalidUtf8)
    }

    /// Führt den Befehl (und die gesamte Pipe-Kette) aus.
    pub fn run_output(&self) -> Result<PipeOutput, CommandError> {
        self.run_piped(None, Vec::new())
    }

    /// Gibt den Namen des letzten Programms in der Pipe-Kette zurück.
    fn last_program_name(&self) -> &str {
        let mut current = self;
        while let Some(next) = &current.piped_into {
            current = next;
        }
        &current.program_name
    }

    /// Interne rekursive Hilfsmethode für Pipe-Ketten.
    ///
    /// **Hinweis zum Deadlock-Risiko:** Bei sehr großen Datenmengen (GB-Bereich)
    /// kann der synchrone `wait_with_output()`-Aufruf blockieren, wenn der
    /// Pipe-Buffer voll ist. Für Standard-CLI-Tools ist das kein Problem.
    /// Bei großen Datenmengen wäre ein Thread- oder async-basierter Ansatz nötig.
    fn run_piped(
        &self,
        stdin: Option<Stdio>,
        mut statuses: Vec<(String, ExitStatus)>,
    ) -> Result<PipeOutput, CommandError> {
        let mut cmd = self.build_command();
        if let Some(s) = stdin {
            cmd.stdin(s);
        }

        match &self.piped_into {
            // Letzter Befehl → direkt ausführen und Output zurückgeben
            None => {
                let output = cmd.output()?;
                Ok(PipeOutput { output, statuses })
            }
            // Mittlerer Befehl → stdout pipen, Exit-Status sammeln, rekursieren
            Some(next) => {
                cmd.stdout(Stdio::piped());
                let mut child = cmd.spawn()?;
                // stdout VOR wait_with_output entnehmen, sonst partial move
                let child_stdout = child.stdout.take().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::Other, "stdout konnte nicht erfasst werden")
                })?;

                let result = child.wait_with_output()?;
                statuses.push((self.program_name.to_string(), result.status));

                next.run_piped(Some(Stdio::from(child_stdout)), statuses)
            }
        }
    }

    fn build_command(&self) -> Command {
        let mut cmd = Command::new(self.program_name.as_ref());

        for (k, v) in &self.env_vars {
            cmd.env(k.as_ref(), v.as_ref());
        }

        for arg in &self.arguments {
            match arg {
                Argument::Flag { prefix, key } =>
                    cmd.arg(format!("{prefix}{key}")),
                Argument::Opt { prefix, key, value } => {
                    cmd.arg(format!("{prefix}{key}"));
                    cmd.arg(value.as_ref())
                }
                Argument::OptEq { prefix, key, value } =>
                    cmd.arg(format!("{prefix}{key}={value}")),
                Argument::Positional(value) =>
                    cmd.arg(value.as_ref()),
            };
        }

        cmd
    }
}

// ── From-Trait für ergonomische Erstellung ────────────────────────────────────

impl<'a> From<&'a str> for LinuxCommand<'a> {
    fn from(name: &'a str) -> Self {
        Self::new(name)
    }
}

impl From<String> for LinuxCommand<'static> {
    fn from(name: String) -> Self {
        Self::new(name)
    }
}

// ── Display ───────────────────────────────────────────────────────────────────

impl fmt::Display for LinuxCommand<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (k, v) in &self.env_vars {
            write!(f, "{k}={} ", Argument::shell_quote(v))?;
        }

        write!(f, "{}", self.program_name)?;

        for arg in &self.arguments {
            write!(f, " {arg}")?;
        }

        if let Some(next) = &self.piped_into {
            write!(f, " | {next}")?;
        }

        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_simple_command() {
        let cmd = LinuxCommand::new("ls").flag("-", "l").flag("-", "a");
        assert_eq!(cmd.to_string(), "ls -l -a");
    }

    #[test]
    fn display_opt_variants() {
        let cmd = LinuxCommand::new("cmd")
            .opt("-", "o", "out.txt")
            .opt_eq("--", "format", "json");
        assert_eq!(cmd.to_string(), "cmd -o out.txt --format=json");
    }

    #[test]
    fn display_with_env() {
        let cmd = LinuxCommand::new("run").env("FOO", "bar").env("AAA", "1");
        assert_eq!(cmd.to_string(), "AAA=1 FOO=bar run");
    }

    #[test]
    fn display_pipe() {
        let cmd = LinuxCommand::new("echo")
            .positional("hello world")
            .pipe(LinuxCommand::new("grep").positional("hello"));
        assert_eq!(cmd.to_string(), "echo 'hello world' | grep hello");
    }

    #[test]
    fn display_deep_pipe() {
        let cmd = LinuxCommand::new("cat")
            .positional("file.txt")
            .pipe(
                LinuxCommand::new("grep")
                    .positional("foo")
                    .pipe(LinuxCommand::new("wc").flag("-", "l")),
            );
        assert_eq!(cmd.to_string(), "cat file.txt | grep foo | wc -l");
    }

    #[test]
    fn shell_quote_sonderzeichen() {
        assert_eq!(Argument::shell_quote("normal"), "normal");
        assert_eq!(Argument::shell_quote("mit leerzeichen"), "'mit leerzeichen'");
        assert_eq!(Argument::shell_quote("it's"), "'it'\\''s'");
        assert_eq!(Argument::shell_quote("$VAR"), "'$VAR'");
        assert_eq!(Argument::shell_quote("back`tick`"), "'back`tick`'");
    }

    #[test]
    fn from_str_trait() {
        let cmd = LinuxCommand::from("echo");
        assert_eq!(cmd.to_string(), "echo");
    }

    #[test]
    fn last_program_name_single() {
        let cmd = LinuxCommand::new("ls");
        assert_eq!(cmd.last_program_name(), "ls");
    }

    #[test]
    fn last_program_name_pipe() {
        let cmd = LinuxCommand::new("cat")
            .pipe(LinuxCommand::new("grep").pipe(LinuxCommand::new("wc")));
        assert_eq!(cmd.last_program_name(), "wc");
    }

    #[test]
    fn execute_echo() {
        let status = LinuxCommand::new("echo").positional("test").execute().unwrap();
        assert!(status.success());
    }

    #[test]
    fn output_string_echo() {
        let out = LinuxCommand::new("echo").positional("hallo").output_string().unwrap();
        assert_eq!(out.trim(), "hallo");
    }

    #[test]
    fn output_string_pipe() {
        let out = LinuxCommand::new("echo")
            .positional("hallo welt")
            .pipe(LinuxCommand::new("grep").positional("welt"))
            .output_string()
            .unwrap();
        assert_eq!(out.trim(), "hallo welt");
    }

    #[test]
    fn error_contains_program_name() {
        let err = LinuxCommand::new("false").output_string().unwrap_err();
        assert!(matches!(err, CommandError::CommandFailed { .. }));
        assert!(err.to_string().contains("false"));
    }

    #[test]
    fn pipe_error_identifies_failed_stage() {
        // `false` schlägt fehl, `cat` danach bekommt leeren Input
        let err = LinuxCommand::new("false")
            .pipe(LinuxCommand::new("cat"))
            .output_string()
            .unwrap_err();
        assert!(matches!(
            err,
            CommandError::PipeFailed { program, .. } if program == "false"
        ));
    }
}