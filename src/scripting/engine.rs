//! The main script engine that executes Rhai scripts.

use rhai::{Engine, AST};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::actions::ScriptAction;
use super::api::{
    self, create_priority_change_scope, create_status_change_scope, create_task_scope,
    create_time_tracking_scope, new_action_queue,
};
use super::config::ScriptConfig;
use super::error::{ScriptError, ScriptResult};
use super::event::HookEvent;

/// The script engine that manages and executes Rhai scripts.
pub struct ScriptEngine {
    /// The Rhai engine instance (kept for potential future use with dynamic scripts).
    #[allow(dead_code)]
    engine: Engine,
    /// Loaded configuration.
    config: ScriptConfig,
    /// Directory containing the config file (for resolving script_file paths).
    #[allow(dead_code)]
    config_dir: PathBuf,
    /// Pre-compiled hook scripts (keyed by hook name).
    compiled_hooks: HashMap<String, AST>,
    /// Pre-compiled command scripts (keyed by command name).
    compiled_commands: HashMap<String, AST>,
}

impl ScriptEngine {
    /// Creates a new script engine with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if scripts fail to compile.
    pub fn new(config: ScriptConfig, config_dir: PathBuf) -> ScriptResult<Self> {
        let mut engine = Engine::new();

        // Set resource limits for safety
        engine.set_max_expr_depths(64, 64);
        engine.set_max_string_size(10_000);
        engine.set_max_array_size(1_000);
        engine.set_max_map_size(500);
        engine.set_max_operations(100_000);

        // Register custom types and functions
        api::register_task_type(&mut engine);

        // We don't register API functions here because they need the action queue,
        // which is created fresh for each execution

        let mut compiled_hooks = HashMap::new();
        let mut compiled_commands = HashMap::new();

        // Pre-compile all enabled hooks
        for (name, hook) in &config.hooks {
            if hook.enabled {
                let script = hook.get_script(&config_dir)?;
                if !script.trim().is_empty() {
                    let ast = engine.compile(&script)?;
                    compiled_hooks.insert(name.clone(), ast);
                }
            }
        }

        // Pre-compile all commands
        for (name, cmd) in &config.commands {
            let script = cmd.get_script(&config_dir)?;
            if !script.trim().is_empty() {
                let ast = engine.compile(&script)?;
                compiled_commands.insert(name.clone(), ast);
            }
        }

        Ok(Self {
            engine,
            config,
            config_dir,
            compiled_hooks,
            compiled_commands,
        })
    }

    /// Loads a script engine from a configuration file.
    ///
    /// # Errors
    ///
    /// Returns an error if the config cannot be loaded or scripts fail to compile.
    pub fn load(config_path: &Path) -> ScriptResult<Self> {
        let config = ScriptConfig::load(config_path)?;
        let config_dir = config_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        Self::new(config, config_dir)
    }

    /// Returns whether the scripting system is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.config.settings.enabled
    }

    /// Returns the configured timeout in seconds.
    #[must_use]
    pub fn timeout_secs(&self) -> u64 {
        self.config.settings.timeout
    }

    /// Returns whether debug mode is enabled.
    #[must_use]
    pub fn is_debug(&self) -> bool {
        self.config.settings.debug
    }

    /// Checks if a hook exists and is enabled.
    #[must_use]
    pub fn has_hook(&self, name: &str) -> bool {
        self.compiled_hooks.contains_key(name)
    }

    /// Returns a list of all registered command names.
    #[must_use]
    pub fn command_names(&self) -> Vec<&str> {
        self.compiled_commands.keys().map(String::as_str).collect()
    }

    /// Executes a hook for the given event.
    ///
    /// Returns a list of actions requested by the script.
    ///
    /// # Errors
    ///
    /// Returns an error if the script fails to execute.
    pub fn execute_hook(&self, event: &HookEvent) -> ScriptResult<Vec<ScriptAction>> {
        if !self.config.settings.enabled {
            return Ok(Vec::new());
        }

        let hook_name = event.hook_name();
        let ast = match self.compiled_hooks.get(hook_name) {
            Some(ast) => ast,
            None => return Ok(Vec::new()), // No hook registered
        };

        // Create fresh action queue
        let actions = new_action_queue();

        // Create a fresh engine with the action queue registered
        let engine = self.create_execution_engine(actions.clone());

        // Create appropriate scope based on event type
        let mut scope = self.create_scope_for_event(event);

        // Execute with timeout check
        let start = Instant::now();
        let timeout = Duration::from_secs(self.config.settings.timeout);

        // Note: Rhai doesn't have async timeout, so we check after execution
        // For truly long-running scripts, we'd need to use Rhai's progress callback
        engine.run_ast_with_scope(&mut scope, ast)?;

        if start.elapsed() > timeout {
            return Err(ScriptError::Timeout(self.config.settings.timeout));
        }

        // Extract collected actions
        let result = actions
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default();

        if self.config.settings.debug && !result.is_empty() {
            eprintln!(
                "[scripting] Hook '{}' produced {} action(s)",
                hook_name,
                result.len()
            );
        }

        Ok(result)
    }

    /// Executes a custom command by name.
    ///
    /// # Errors
    ///
    /// Returns an error if the command doesn't exist or fails to execute.
    pub fn execute_command(&self, name: &str) -> ScriptResult<Vec<ScriptAction>> {
        if !self.config.settings.enabled {
            return Err(ScriptError::Config("Scripting is disabled".to_string()));
        }

        let ast = self
            .compiled_commands
            .get(name)
            .ok_or_else(|| ScriptError::CommandNotFound(name.to_string()))?;

        // Create fresh action queue
        let actions = new_action_queue();

        // Create a fresh engine with the action queue registered
        let engine = self.create_execution_engine(actions.clone());

        let mut scope = rhai::Scope::new();

        // Execute with timeout check
        let start = Instant::now();
        let timeout = Duration::from_secs(self.config.settings.timeout);

        engine.run_ast_with_scope(&mut scope, ast)?;

        if start.elapsed() > timeout {
            return Err(ScriptError::Timeout(self.config.settings.timeout));
        }

        let result = actions
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default();

        if self.config.settings.debug {
            eprintln!(
                "[scripting] Command '{}' produced {} action(s)",
                name,
                result.len()
            );
        }

        Ok(result)
    }

    /// Creates a new engine instance with the action queue registered.
    fn create_execution_engine(&self, actions: api::ActionQueue) -> Engine {
        let mut engine = Engine::new();

        // Copy resource limits
        engine.set_max_expr_depths(64, 64);
        engine.set_max_string_size(10_000);
        engine.set_max_array_size(1_000);
        engine.set_max_map_size(500);
        engine.set_max_operations(100_000);

        // Register types and API
        api::register_task_type(&mut engine);
        api::register_api(&mut engine, actions);

        engine
    }

    /// Creates the appropriate scope for an event.
    fn create_scope_for_event(&self, event: &HookEvent) -> rhai::Scope<'static> {
        match event {
            HookEvent::TaskCreated { task }
            | HookEvent::TaskCompleted { task }
            | HookEvent::TaskDeleted { task } => create_task_scope(task),

            HookEvent::TaskStatusChanged {
                task,
                old_status,
                new_status,
            } => create_status_change_scope(task, *old_status, *new_status),

            HookEvent::TaskPriorityChanged {
                task,
                old_priority,
                new_priority,
            } => create_priority_change_scope(task, *old_priority, *new_priority),

            HookEvent::TimeTrackingStarted { task } => create_time_tracking_scope(task, None),

            HookEvent::TimeTrackingStopped {
                task,
                duration_mins,
            } => create_time_tracking_scope(task, Some(*duration_mins)),

            HookEvent::PomodoroPhaseCompleted { phase, task } => {
                let mut scope = create_task_scope(task);
                scope.push("phase", format!("{phase:?}").to_lowercase());
                scope
            }

            HookEvent::TagAdded { task, tag } | HookEvent::TagRemoved { task, tag } => {
                let mut scope = create_task_scope(task);
                scope.push("tag", tag.clone());
                scope
            }
        }
    }
}

impl std::fmt::Debug for ScriptEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptEngine")
            .field("enabled", &self.config.settings.enabled)
            .field("hooks", &self.compiled_hooks.keys().collect::<Vec<_>>())
            .field(
                "commands",
                &self.compiled_commands.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Task;

    fn create_test_engine() -> ScriptEngine {
        let config = ScriptConfig::default();
        ScriptEngine::new(config, PathBuf::new()).unwrap()
    }

    #[test]
    fn test_engine_creation() {
        let engine = create_test_engine();
        assert!(engine.is_enabled());
        assert_eq!(engine.timeout_secs(), 5);
    }

    #[test]
    fn test_no_hooks_returns_empty() {
        let engine = create_test_engine();
        let task = Task::new("Test task");
        let event = HookEvent::TaskCreated { task };

        let actions = engine.execute_hook(&event).unwrap();
        assert!(actions.is_empty());
    }

    #[test]
    fn test_execute_simple_hook() {
        let toml = r#"
            [hooks.on_task_completed]
            script = "log(task.title);"
        "#;
        let config: ScriptConfig = toml::from_str(toml).unwrap();
        let engine = ScriptEngine::new(config, PathBuf::new()).unwrap();

        let task = Task::new("Test task");
        let event = HookEvent::TaskCompleted { task };

        let actions = engine.execute_hook(&event).unwrap();
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], ScriptAction::Log { .. }));
    }

    #[test]
    fn test_execute_hook_with_create_task() {
        let toml = r#"
            [hooks.on_task_completed]
            script = """
                create_task("Follow-up: " + task.title);
            """
        "#;
        let config: ScriptConfig = toml::from_str(toml).unwrap();
        let engine = ScriptEngine::new(config, PathBuf::new()).unwrap();

        let task = Task::new("Original task");
        let event = HookEvent::TaskCompleted { task };

        let actions = engine.execute_hook(&event).unwrap();
        assert_eq!(actions.len(), 1);

        if let ScriptAction::CreateTask { title, .. } = &actions[0] {
            assert_eq!(title, "Follow-up: Original task");
        } else {
            panic!("Expected CreateTask action");
        }
    }

    #[test]
    fn test_status_change_scope() {
        // Note: Status is formatted as Debug lowercased, so InProgress -> "inprogress"
        let toml = r#"
            [hooks.on_task_status_changed]
            script = """
                if new_status == "inprogress" {
                    log("Started: " + task.title);
                }
            """
        "#;
        let config: ScriptConfig = toml::from_str(toml).unwrap();
        let engine = ScriptEngine::new(config, PathBuf::new()).unwrap();

        let task = Task::new("Test task");
        let event = HookEvent::TaskStatusChanged {
            task,
            old_status: crate::domain::TaskStatus::Todo,
            new_status: crate::domain::TaskStatus::InProgress,
        };

        let actions = engine.execute_hook(&event).unwrap();
        assert_eq!(actions.len(), 1);
    }

    #[test]
    fn test_disabled_engine_returns_empty() {
        // When scripting is disabled, hooks should not be compiled
        // so we can use an empty script to avoid compilation errors
        let toml = r#"
            [settings]
            enabled = false

            [hooks.on_task_completed]
            script = ""
        "#;
        let config: ScriptConfig = toml::from_str(toml).unwrap();
        let engine = ScriptEngine::new(config, PathBuf::new()).unwrap();

        let task = Task::new("Test task");
        let event = HookEvent::TaskCompleted { task };

        let actions = engine.execute_hook(&event).unwrap();
        assert!(actions.is_empty());
    }

    #[test]
    fn test_invalid_script_fails_compilation() {
        let toml = r#"
            [hooks.on_task_completed]
            script = "this is not valid rhai syntax {"
        "#;
        let config: ScriptConfig = toml::from_str(toml).unwrap();
        let result = ScriptEngine::new(config, PathBuf::new());

        assert!(result.is_err());
    }

    #[test]
    fn test_command_execution() {
        let toml = r#"
            [commands.test_cmd]
            description = "Test command"
            script = """
                create_task("New task from command");
                log("Command executed");
            """
        "#;
        let config: ScriptConfig = toml::from_str(toml).unwrap();
        let engine = ScriptEngine::new(config, PathBuf::new()).unwrap();

        let actions = engine.execute_command("test_cmd").unwrap();
        assert_eq!(actions.len(), 2);
    }

    #[test]
    fn test_command_not_found() {
        let engine = create_test_engine();
        let result = engine.execute_command("nonexistent");

        assert!(result.is_err());
        assert!(matches!(result, Err(ScriptError::CommandNotFound(_))));
    }
}
