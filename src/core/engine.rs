use crate::core::models::{RuleContext, RuleDecision};
use anyhow::{Context, Result};
use rhai::{AST, Dynamic, Engine, Map, Scope};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

/// Seam for rule evaluation across message events.
pub trait RuleEngine: Send + Sync {
    fn evaluate_content(
        &self,
        ctx: &RuleContext,
    ) -> impl std::future::Future<Output = Result<RuleDecision>> + Send;

    fn reload(&self) -> impl std::future::Future<Output = Result<()>> + Send;
}

pub struct RhaiRuleEngine {
    engine: Engine,
    modules_dir: PathBuf,
    ast_cache: RwLock<HashMap<String, AST>>,
}

pub type RhaiManager = RhaiRuleEngine;

impl RhaiRuleEngine {
    pub fn new<P: AsRef<Path>>(modules_dir: P) -> Self {
        let mut engine = Engine::new();
        engine.set_max_operations(100_000);
        engine.set_max_call_levels(50);

        // Register host native bindings (logger and i18n)
        crate::core::bindings::register_all(&mut engine);

        Self {
            engine,
            modules_dir: modules_dir.as_ref().to_path_buf(),
            ast_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Load and compile all .rhai script files from modules directory
    pub async fn load_all(&self) -> Result<()> {
        let mut ast_map = HashMap::new();
        if !self.modules_dir.exists() {
            tokio::fs::create_dir_all(&self.modules_dir).await?;
        }

        let mut dir = tokio::fs::read_dir(&self.modules_dir).await?;
        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rhai") {
                let name = path.file_stem().unwrap().to_string_lossy().to_string();
                let content = tokio::fs::read_to_string(&path).await?;
                let ast = self.engine.compile(&content).with_context(|| {
                    format!("Failed to compile Rhai script: {}", path.display())
                })?;

                tracing::info!(module_name = %name, path = %path.display(), "Loaded Rhai script module");
                ast_map.insert(name, ast);
            }
        }

        let mut cache = self.ast_cache.write().await;
        *cache = ast_map;
        Ok(())
    }

    /// Reload all module scripts (Hot-reload)
    pub async fn reload(&self) -> Result<()> {
        tracing::info!("Hot-reloading all Rhai script modules...");
        self.load_all().await
    }

    /// Call a function in a specific loaded Rhai module script
    pub async fn call_fn<T: Clone + Send + Sync + 'static>(
        &self,
        module_name: &str,
        fn_name: &str,
        args: impl rhai::FuncArgs,
    ) -> Result<Option<T>> {
        let ast = {
            let cache = self.ast_cache.read().await;
            match cache.get(module_name) {
                Some(ast) => ast.clone(),
                None => return Ok(None),
            }
        };

        let mut scope = Scope::new();
        match self.engine.call_fn::<T>(&mut scope, &ast, fn_name, args) {
            Ok(result) => Ok(Some(result)),
            Err(err) => {
                tracing::error!(module = %module_name, function = %fn_name, error = %err, "Rhai script execution failed");
                Err(anyhow::anyhow!(
                    "Rhai execution error in {}.{}: {}",
                    module_name,
                    fn_name,
                    err
                ))
            }
        }
    }

    /// Compiles a script directly into the AST cache for testing or dynamic loading.
    pub async fn insert_module(&self, name: &str, script: &str) -> Result<()> {
        let ast = self
            .engine
            .compile(script)
            .with_context(|| format!("Failed to compile in-memory script module: {name}"))?;
        let mut cache = self.ast_cache.write().await;
        cache.insert(name.to_string(), ast);
        Ok(())
    }
}

impl RuleEngine for RhaiRuleEngine {
    async fn evaluate_content(&self, ctx: &RuleContext) -> Result<RuleDecision> {
        let result: Option<Dynamic> = self
            .call_fn(
                "automod",
                "inspect_message",
                (
                    ctx.content.clone(),
                    ctx.author_id.clone(),
                    ctx.guild_id.clone(),
                ),
            )
            .await?;

        let Some(dynamic) = result else {
            return Ok(RuleDecision::Pass);
        };

        if let Some(map) = dynamic.try_cast::<Map>() {
            let action = map
                .get("action")
                .and_then(|v| v.clone().into_string().ok())
                .unwrap_or_default();

            if action == "flag_suggestion" {
                let rule_id = map
                    .get("rule_id")
                    .and_then(|v| v.clone().into_string().ok())
                    .unwrap_or_else(|| "general_rule".to_string());
                let reason = map
                    .get("reason")
                    .and_then(|v| v.clone().into_string().ok())
                    .unwrap_or_else(|| "Automod flagged".to_string());
                let should_warn = map
                    .get("should_warn")
                    .and_then(|v| v.as_bool().ok())
                    .unwrap_or(false);

                return Ok(RuleDecision::FlagSuggestion {
                    rule_id,
                    reason,
                    should_warn,
                });
            }
        }

        Ok(RuleDecision::Pass)
    }

    async fn reload(&self) -> Result<()> {
        self.reload().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::{RuleContext, RuleDecision};

    fn make_test_dir(prefix: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "dummy-bot-{prefix}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    #[tokio::test]
    async fn evaluates_automod_pass_for_clean_message() {
        let temp_dir = make_test_dir("clean-msg");
        let engine = RhaiRuleEngine::new(&temp_dir);
        let automod_script = r#"
            fn inspect_message(content, author_id, guild_id) {
                return #{ action: "pass" };
            }
        "#;
        engine
            .insert_module("automod", automod_script)
            .await
            .unwrap();

        let ctx = RuleContext {
            content: "Hello everyone, how are you?".to_string(),
            author_id: "12345".to_string(),
            guild_id: "67890".to_string(),
        };

        let decision = engine.evaluate_content(&ctx).await.unwrap();
        assert_eq!(decision, RuleDecision::Pass);
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn evaluates_automod_flag_suggestion_for_badword() {
        let temp_dir = make_test_dir("badword");
        let engine = RhaiRuleEngine::new(&temp_dir);
        let automod_script = r#"
            fn inspect_message(content, author_id, guild_id) {
                let lower = content.to_lower();
                if lower.contains("badword_test") {
                    return #{
                        action: "flag_suggestion",
                        rule_id: "blocked_words",
                        reason: "Detected blocked content",
                        should_warn: true
                    };
                }
                return #{ action: "pass" };
            }
        "#;
        engine
            .insert_module("automod", automod_script)
            .await
            .unwrap();

        let ctx = RuleContext {
            content: "This message contains badword_test here!".to_string(),
            author_id: "12345".to_string(),
            guild_id: "67890".to_string(),
        };

        let decision = engine.evaluate_content(&ctx).await.unwrap();
        assert_eq!(
            decision,
            RuleDecision::FlagSuggestion {
                rule_id: "blocked_words".to_string(),
                reason: "Detected blocked content".to_string(),
                should_warn: true,
            }
        );
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn sandboxed_execution_budget_limits_infinite_loops() {
        let temp_dir = make_test_dir("loop");
        let engine = RhaiRuleEngine::new(&temp_dir);
        let infinite_loop_script = r#"
            fn inspect_message(content, author_id, guild_id) {
                while true {}
                return #{ action: "pass" };
            }
        "#;
        engine
            .insert_module("automod", infinite_loop_script)
            .await
            .unwrap();

        let ctx = RuleContext {
            content: "test".to_string(),
            author_id: "1".to_string(),
            guild_id: "2".to_string(),
        };

        let result = engine.evaluate_content(&ctx).await;
        assert!(
            result.is_err(),
            "Expected sandbox budget limit to abort infinite loop"
        );
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn missing_automod_module_returns_pass() {
        let temp_dir = make_test_dir("missing");
        let engine = RhaiRuleEngine::new(&temp_dir);

        let ctx = RuleContext {
            content: "clean text".to_string(),
            author_id: "1".to_string(),
            guild_id: "2".to_string(),
        };

        let decision = engine.evaluate_content(&ctx).await.unwrap();
        assert_eq!(decision, RuleDecision::Pass);
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn hot_reloading_updates_rules() {
        let temp_dir = make_test_dir("reload");
        let script_file = temp_dir.join("automod.rhai");

        tokio::fs::write(
            &script_file,
            r#"fn inspect_message(c, a, g) { return #{ action: "pass" }; }"#,
        )
        .await
        .unwrap();

        let engine = RhaiRuleEngine::new(&temp_dir);
        engine.load_all().await.unwrap();

        let ctx = RuleContext {
            content: "danger".to_string(),
            author_id: "1".to_string(),
            guild_id: "2".to_string(),
        };
        assert_eq!(
            engine.evaluate_content(&ctx).await.unwrap(),
            RuleDecision::Pass
        );

        // Update rule on disk and reload
        tokio::fs::write(
            &script_file,
            r#"fn inspect_message(c, a, g) {
                if c.contains("danger") {
                    return #{ action: "flag_suggestion", rule_id: "danger_rule", reason: "Danger found", should_warn: false };
                }
                return #{ action: "pass" };
            }"#,
        )
        .await
        .unwrap();

        engine.reload().await.unwrap();
        assert_eq!(
            engine.evaluate_content(&ctx).await.unwrap(),
            RuleDecision::FlagSuggestion {
                rule_id: "danger_rule".to_string(),
                reason: "Danger found".to_string(),
                should_warn: false,
            }
        );
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }
}
