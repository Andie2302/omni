use std::io::{BufRead, BufReader};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;

use crate::omni_command::OmniCommand;

// ─────────────────────────────────────────────
//  Output-Event  (was der Executor nach außen liefert)
// ─────────────────────────────────────────────

/// Ein einzelnes Ausgabe-Ereignis, das während der Ausführung auftreten kann.
#[derive(Debug, Clone)]
pub enum OutputEvent {
    /// Eine Zeile auf stdout.
    Stdout(String),
    /// Eine Zeile auf stderr.
    Stderr(String),
    /// Der Prozess hat beendet (ExitCode, falls verfügbar).
    Finished(Option<i32>),
    /// Der Prozess konnte nicht gestartet werden.
    SpawnFailed(String),
}

// ─────────────────────────────────────────────
//  OutputHandler  (Callback-Trait)
// ─────────────────────────────────────────────

/// Implementiere diesen Trait, um Ausgaben des Befehls zu empfangen.
///
/// # Beispiel – einfacher Logging-Handler
/// ```rust
/// struct LogHandler;
/// impl OutputHandler for LogHandler {
///     fn on_event(&self, event: OutputEvent) {
///         match event {
///             OutputEvent::Stdout(line) => println!("[OUT] {}", line),
///             OutputEvent::Stderr(line) => eprintln!("[ERR] {}", line),
///             OutputEvent::Finished(code) => println!("Exit: {:?}", code),
///             OutputEvent::SpawnFailed(msg) => eprintln!("Spawn failed: {}", msg),
///         }
///     }
/// }
/// ```
pub trait OutputHandler: Send + Sync {
    fn on_event(&self, event: OutputEvent);
}

/// Einfacher Default-Handler, der alles auf stdout/stderr weiterleitet.
pub struct PrintHandler;

impl OutputHandler for PrintHandler {
    fn on_event(&self, event: OutputEvent) {
        match event {
            OutputEvent::Stdout(line) => println!("{}", line),
            OutputEvent::Stderr(line) => eprintln!("{}", line),
            OutputEvent::Finished(code) => {
                println!("[omni] Befehl beendet (Exit-Code: {:?})", code)
            }
            OutputEvent::SpawnFailed(msg) => eprintln!("[omni] Spawn fehlgeschlagen: {}", msg),
        }
    }
}

/// Handler, der alle Ausgaben sammelt (z. B. für Tests oder Weiterverarbeitung).
#[derive(Default)]
pub struct CollectingHandler {
    pub stdout_lines: std::sync::Mutex<Vec<String>>,
    pub stderr_lines: std::sync::Mutex<Vec<String>>,
    pub exit_code: std::sync::Mutex<Option<Option<i32>>>,
}

impl CollectingHandler {
    pub fn new() -> Self {
        Self::default()
    }

    /// Gibt alle stdout-Zeilen als einen String zurück.
    pub fn stdout(&self) -> String {
        self.stdout_lines.lock().unwrap().join("\n")
    }

    /// Gibt alle stderr-Zeilen als einen String zurück.
    pub fn stderr(&self) -> String {
        self.stderr_lines.lock().unwrap().join("\n")
    }

    /// Gibt den Exit-Code zurück (None = noch nicht beendet).
    pub fn exit_code(&self) -> Option<Option<i32>> {
        *self.exit_code.lock().unwrap()
    }
}

impl OutputHandler for CollectingHandler {
    fn on_event(&self, event: OutputEvent) {
        match event {
            OutputEvent::Stdout(line) => self.stdout_lines.lock().unwrap().push(line),
            OutputEvent::Stderr(line) => self.stderr_lines.lock().unwrap().push(line),
            OutputEvent::Finished(code) => *self.exit_code.lock().unwrap() = Some(code),
            OutputEvent::SpawnFailed(_) => {}
        }
    }
}

// ─────────────────────────────────────────────
//  ExecuteResult
// ─────────────────────────────────────────────

/// Das Ergebnis einer `execute`-Aufruf.
#[derive(Debug)]
pub enum ExecuteResult {
    /// Dry-Run: Befehl wurde simuliert, nicht ausgeführt.
    Simulated,
    /// Befehl wurde ausgeführt und hat mit diesem Status geendet.
    Completed(ExitStatus),
    /// Prozess konnte nicht gestartet werden.
    SpawnFailed(std::io::Error),
    /// Prozess lief, aber `wait()` schlug fehl.
    WaitFailed(std::io::Error),
}

impl ExecuteResult {
    /// `true` wenn die Ausführung als erfolgreich gilt.
    pub fn is_success(&self) -> bool {
        match self {
            ExecuteResult::Simulated => true,
            ExecuteResult::Completed(s) => s.success(),
            _ => false,
        }
    }

    /// Exit-Code, falls bekannt.
    pub fn exit_code(&self) -> Option<i32> {
        match self {
            ExecuteResult::Completed(s) => s.code(),
            _ => None,
        }
    }

    /// Gibt einen menschenlesbaren Status zurück.
    pub fn status_message(&self) -> String {
        match self {
            ExecuteResult::Simulated => "Simuliert (Dry Run)".into(),
            ExecuteResult::Completed(s) => {
                format!(
                    "Beendet (Exit-Code: {})",
                    s.code().map_or("?".into(), |c| c.to_string())
                )
            }
            ExecuteResult::SpawnFailed(e) => format!("Spawn fehlgeschlagen: {}", e),
            ExecuteResult::WaitFailed(e) => format!("Wait fehlgeschlagen: {}", e),
        }
    }
}

// ─────────────────────────────────────────────
//  ExecutorConfig  (Builder-Muster)
// ─────────────────────────────────────────────

/// Konfiguration für den `OmniExecutor`.
pub struct ExecutorConfig {
    /// Kein Befehl wird wirklich ausgeführt.
    pub dry_run: bool,
    /// stdout und stderr werden zeilenweise gelesen und an den Handler weitergegeben.
    /// Wenn `false`, werden stdin/stdout/stderr direkt vererbt (interaktiv).
    pub capture_output: bool,
    /// Arbeitsverzeichnis für den Prozess.
    pub working_dir: Option<std::path::PathBuf>,
    /// Zusätzliche Umgebungsvariablen.
    pub env_vars: Vec<(String, String)>,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            dry_run: false,
            capture_output: true,
            working_dir: None,
            env_vars: Vec::new(),
        }
    }
}

impl ExecutorConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dry_run(mut self, enabled: bool) -> Self {
        self.dry_run = enabled;
        self
    }

    pub fn capture_output(mut self, enabled: bool) -> Self {
        self.capture_output = enabled;
        self
    }

    pub fn working_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    pub fn env_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.push((key.into(), value.into()));
        self
    }
}

// ─────────────────────────────────────────────
//  OmniExecutor
// ─────────────────────────────────────────────

/// Universeller Executor für `OmniCommand`-Befehle.
///
/// # Schnellstart
/// ```rust
/// let handler = Arc::new(PrintHandler);
/// let executor = OmniExecutor::new(ExecutorConfig::default(), handler);
///
/// let cmd = OmniCommand::new("echo")
///     .with_arg(OmniCommandArg::new("Hello World"));
///
/// let result = executor.execute(&cmd);
/// println!("{}", result.status_message());
/// ```
pub struct OmniExecutor {
    config: ExecutorConfig,
    handler: Arc<dyn OutputHandler>,
}

impl OmniExecutor {
    /// Erstellt einen neuen Executor mit Konfiguration und Handler.
    pub fn new(config: ExecutorConfig, handler: Arc<dyn OutputHandler>) -> Self {
        Self { config, handler }
    }

    /// Bequeme Factory: Dry-Run mit PrintHandler.
    pub fn dry_run() -> Self {
        Self::new(
            ExecutorConfig::new().dry_run(true),
            Arc::new(PrintHandler),
        )
    }

    /// Bequeme Factory: Live-Ausführung mit PrintHandler.
    pub fn live() -> Self {
        Self::new(ExecutorConfig::new(), Arc::new(PrintHandler))
    }

    /// Führt einen `OmniCommand` aus (oder simuliert ihn).
    pub fn execute(&self, cmd: &OmniCommand) -> ExecuteResult {
        if self.config.dry_run {
            self.show_dry_run_info(cmd);
            return ExecuteResult::Simulated;
        }

        let mut process = Command::new(&cmd.name);

        // Argumente
        for arg in &cmd.args {
            process.args(arg.to_os_args());
        }

        // Arbeitsverzeichnis
        if let Some(dir) = &self.config.working_dir {
            process.current_dir(dir);
        }

        // Umgebungsvariablen
        for (key, val) in &self.config.env_vars {
            process.env(key, val);
        }

        if self.config.capture_output {
            self.execute_captured(&mut process)
        } else {
            self.execute_inherited(&mut process)
        }
    }

    // ── Interne Helfer ───────────────────────

    /// Ausführung mit Capture: stdout/stderr werden zeilenweise gelesen.
    fn execute_captured(&self, process: &mut Command) -> ExecuteResult {
        process
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = match process.spawn() {
            Ok(c) => c,
            Err(e) => {
                self.handler
                    .on_event(OutputEvent::SpawnFailed(e.to_string()));
                return ExecuteResult::SpawnFailed(e);
            }
        };

        // stdout in eigenem Thread lesen
        let stdout_handler = Arc::clone(&self.handler);
        let stdout_pipe = child.stdout.take().expect("stdout war nicht piped");
        let stdout_thread = std::thread::spawn(move || {
            let reader = BufReader::new(stdout_pipe);
            for line in reader.lines().map_while(Result::ok) {
                stdout_handler.on_event(OutputEvent::Stdout(line));
            }
        });

        // stderr in eigenem Thread lesen
        let stderr_handler = Arc::clone(&self.handler);
        let stderr_pipe = child.stderr.take().expect("stderr war nicht piped");
        let stderr_thread = std::thread::spawn(move || {
            let reader = BufReader::new(stderr_pipe);
            for line in reader.lines().map_while(Result::ok) {
                stderr_handler.on_event(OutputEvent::Stderr(line));
            }
        });

        // Auf Prozess-Ende warten
        let result = match child.wait() {
            Ok(status) => {
                let _ = stdout_thread.join();
                let _ = stderr_thread.join();
                self.handler
                    .on_event(OutputEvent::Finished(status.code()));
                ExecuteResult::Completed(status)
            }
            Err(e) => {
                let _ = stdout_thread.join();
                let _ = stderr_thread.join();
                ExecuteResult::WaitFailed(e)
            }
        };

        result
    }

    /// Ausführung ohne Capture: stdin/stdout/stderr werden vererbt (interaktiv).
    fn execute_inherited(&self, process: &mut Command) -> ExecuteResult {
        process
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        match process.spawn() {
            Err(e) => {
                self.handler
                    .on_event(OutputEvent::SpawnFailed(e.to_string()));
                ExecuteResult::SpawnFailed(e)
            }
            Ok(mut child) => match child.wait() {
                Ok(status) => {
                    self.handler
                        .on_event(OutputEvent::Finished(status.code()));
                    ExecuteResult::Completed(status)
                }
                Err(e) => ExecuteResult::WaitFailed(e),
            },
        }
    }

    /// Gibt Dry-Run-Informationen aus.
    fn show_dry_run_info(&self, cmd: &OmniCommand) {
        println!("┌─ DRY RUN ──────────────────────────────");
        println!("│  Befehl : {}", cmd);
        if let Some(dir) = &self.config.working_dir {
            println!("│  Verz.  : {}", dir.display());
        }
        if !self.config.env_vars.is_empty() {
            for (k, v) in &self.config.env_vars {
                println!("│  Env    : {}={}", k, v);
            }
        }
        println!("└────────────────────────────────────────");
    }
}

// ─────────────────────────────────────────────
//  Tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use crate::omni_command::{OmniCommand, OmniCommandArg};

    fn collecting_executor(dry_run: bool) -> (OmniExecutor, Arc<CollectingHandler>) {
        let handler = Arc::new(CollectingHandler::new());
        let config = ExecutorConfig::new()
            .dry_run(dry_run)
            .capture_output(true);
        let executor = OmniExecutor::new(config, Arc::clone(&handler) as Arc<dyn OutputHandler>);
        (executor, handler)
    }

    #[test]
    fn dry_run_returns_simulated() {
        let (executor, _) = collecting_executor(true);
        let cmd = OmniCommand::new("echo").with_arg(OmniCommandArg::new("hello"));
        let result = executor.execute(&cmd);
        assert!(matches!(result, ExecuteResult::Simulated));
        assert!(result.is_success());
    }

    #[test]
    fn echo_stdout_captured() {
        let (executor, handler) = collecting_executor(false);
        let cmd = OmniCommand::new("echo").with_arg(OmniCommandArg::new("omni_test"));
        let result = executor.execute(&cmd);
        assert!(result.is_success());
        assert!(handler.stdout().contains("omni_test"));
    }

    #[test]
    fn nonexistent_command_spawn_failed() {
        let (executor, _) = collecting_executor(false);
        let cmd = OmniCommand::new("__does_not_exist_xyz__");
        let result = executor.execute(&cmd);
        assert!(matches!(result, ExecuteResult::SpawnFailed(_)));
        assert!(!result.is_success());
    }

    #[test]
    fn exit_code_nonzero_on_failure() {
        let (executor, _) = collecting_executor(false);
        // `false` ist ein Unix-Befehl der immer exit(1) zurückgibt
        let cmd = OmniCommand::new("false");
        let result = executor.execute(&cmd);
        assert!(!result.is_success());
        assert_eq!(result.exit_code(), Some(1));
    }
}