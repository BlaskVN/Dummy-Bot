use crate::core::models::{RuleContext, RuleDecision};
use anyhow::{Context, Result, bail};
use rhai::{AST, Dynamic, Engine, Map, Scope};
use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use tokio::sync::RwLock;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Object-safe seam for rule evaluation across message events.
pub trait RuleEngine: Send + Sync {
    fn evaluate_content<'a>(&'a self, ctx: &'a RuleContext) -> BoxFuture<'a, Result<RuleDecision>>;
    fn reload<'a>(&'a self) -> BoxFuture<'a, Result<()>>;
}

pub struct RhaiRuleEngine {
    engine: Engine,
    modules_dir: PathBuf,
    ast_cache: RwLock<HashMap<String, AST>>,
}

impl RhaiRuleEngine {
    pub fn new<P: AsRef<Path>>(modules_dir: P) -> Self {
        let mut engine = Engine::new();
        engine.set_max_operations(100_000);
        engine.set_max_call_levels(50);

        // Register host native logger bindings
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

    async fn call_fn<T: Clone + Send + Sync + 'static>(
        &self,
        module_name: &str,
        fn_name: &str,
        args: impl rhai::FuncArgs,
    ) -> Result<T> {
        let ast = {
            let cache = self.ast_cache.read().await;
            cache
                .get(module_name)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Module '{module_name}' not loaded"))?
        };

        let mut scope = Scope::new();
        match self.engine.call_fn::<T>(&mut scope, &ast, fn_name, args) {
            Ok(result) => Ok(result),
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

    #[cfg(test)]
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
    fn evaluate_content<'a>(&'a self, ctx: &'a RuleContext) -> BoxFuture<'a, Result<RuleDecision>> {
        Box::pin(async move {
            let author_id = ctx.author_id.to_string();
            let guild_id = ctx.guild_id.to_string();

            // Support either "rules" (preferred) or legacy "automod" module name
            let module_name = {
                let cache = self.ast_cache.read().await;
                if cache.contains_key("rules") {
                    "rules"
                } else if cache.contains_key("automod") {
                    "automod"
                } else {
                    bail!("Rule script module not loaded in rule engine");
                }
            };

            let dynamic: Dynamic = self
                .call_fn(
                    module_name,
                    "inspect_message",
                    (ctx.content.clone(), author_id, guild_id),
                )
                .await?;

            let map = dynamic.try_cast::<Map>().ok_or_else(|| {
                anyhow::anyhow!("inspect_message must return a map, received other type")
            })?;

            let action = map
                .get("action")
                .and_then(|v| v.clone().into_string().ok())
                .ok_or_else(|| {
                    anyhow::anyhow!("inspect_message map missing 'action' string field")
                })?;

            match action.as_str() {
                "flag_suggestion" => {
                    let rule_id = map
                        .get("rule_id")
                        .and_then(|v| v.clone().into_string().ok())
                        .unwrap_or_else(|| "general_rule".to_string());
                    let reason = map
                        .get("reason")
                        .and_then(|v| v.clone().into_string().ok())
                        .unwrap_or_else(|| "Message rule flagged".to_string());
                    let should_warn = map
                        .get("should_warn")
                        .and_then(|v| v.as_bool().ok())
                        .unwrap_or(false);

                    Ok(RuleDecision::FlagSuggestion {
                        rule_id,
                        reason,
                        should_warn,
                    })
                }
                "pass" => Ok(RuleDecision::Pass),
                other => bail!("Unknown rule decision action: {other}"),
            }
        })
    }

    fn reload<'a>(&'a self) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            tracing::info!("Hot-reloading rule script modules...");
            self.load_all().await
        })
    }
}

/// In-memory mock rule engine for seam substitution in unit tests.
#[cfg(test)]
pub struct MockRuleEngine {
    decision: std::sync::Mutex<RuleDecision>,
    reload_count: std::sync::atomic::AtomicUsize,
}

#[cfg(test)]
impl MockRuleEngine {
    pub fn new(initial_decision: RuleDecision) -> Self {
        Self {
            decision: std::sync::Mutex::new(initial_decision),
            reload_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub fn set_decision(&self, decision: RuleDecision) {
        *self.decision.lock().unwrap() = decision;
    }

    pub fn reload_count(&self) -> usize {
        self.reload_count.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[cfg(test)]
impl RuleEngine for MockRuleEngine {
    fn evaluate_content<'a>(
        &'a self,
        _ctx: &'a RuleContext,
    ) -> BoxFuture<'a, Result<RuleDecision>> {
        Box::pin(async move { Ok(self.decision.lock().unwrap().clone()) })
    }

    fn reload<'a>(&'a self) -> BoxFuture<'a, Result<()>> {
        Box::pin(async move {
            self.reload_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::models::{RuleContext, RuleDecision};
    use poise::serenity_prelude::{GuildId, UserId};
    use std::sync::Arc;

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
    async fn evaluates_rule_pass_for_clean_message() {
        let temp_dir = make_test_dir("clean-msg");
        let engine = RhaiRuleEngine::new(&temp_dir);
        let rules_script = r#"
            fn inspect_message(content, author_id, guild_id) {
                return #{ action: "pass" };
            }
        "#;
        engine.insert_module("rules", rules_script).await.unwrap();

        let ctx = RuleContext {
            content: "Hello everyone, how are you?".to_string(),
            author_id: UserId::new(12345),
            guild_id: GuildId::new(67890),
        };

        let decision = engine.evaluate_content(&ctx).await.unwrap();
        assert_eq!(decision, RuleDecision::Pass);
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn evaluates_rule_flag_suggestion_for_badword() {
        let temp_dir = make_test_dir("badword");
        let engine = RhaiRuleEngine::new(&temp_dir);
        let rules_script = r#"
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
        engine.insert_module("rules", rules_script).await.unwrap();

        let ctx = RuleContext {
            content: "This message contains badword_test here!".to_string(),
            author_id: UserId::new(12345),
            guild_id: GuildId::new(67890),
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
            .insert_module("rules", infinite_loop_script)
            .await
            .unwrap();

        let ctx = RuleContext {
            content: "test".to_string(),
            author_id: UserId::new(1),
            guild_id: GuildId::new(2),
        };

        let result = engine.evaluate_content(&ctx).await;
        assert!(
            result.is_err(),
            "Expected sandbox budget limit to abort infinite loop"
        );
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn missing_rule_module_returns_error() {
        let temp_dir = make_test_dir("missing");
        let engine = RhaiRuleEngine::new(&temp_dir);

        let ctx = RuleContext {
            content: "clean text".to_string(),
            author_id: UserId::new(1),
            guild_id: GuildId::new(2),
        };

        let result = engine.evaluate_content(&ctx).await;
        assert!(
            result.is_err(),
            "Expected error when rule module is missing"
        );
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn malformed_script_output_returns_error() {
        let temp_dir = make_test_dir("malformed");
        let engine = RhaiRuleEngine::new(&temp_dir);
        let bad_script = r#"
            fn inspect_message(content, author_id, guild_id) {
                return 42; // Returns integer instead of map
            }
        "#;
        engine.insert_module("rules", bad_script).await.unwrap();

        let ctx = RuleContext {
            content: "clean text".to_string(),
            author_id: UserId::new(1),
            guild_id: GuildId::new(2),
        };

        let result = engine.evaluate_content(&ctx).await;
        assert!(result.is_err(), "Expected error on non-map return");
        let _ = tokio::fs::remove_dir_all(&temp_dir).await;
    }

    #[tokio::test]
    async fn hot_reloading_updates_rules() {
        let temp_dir = make_test_dir("reload");
        let script_file = temp_dir.join("rules.rhai");

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
            author_id: UserId::new(1),
            guild_id: GuildId::new(2),
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

    #[tokio::test]
    async fn mock_rule_engine_substitution_via_seam() {
        let mock: Arc<dyn RuleEngine> = Arc::new(MockRuleEngine::new(RuleDecision::Pass));
        let ctx = RuleContext {
            content: "anything".to_string(),
            author_id: UserId::new(10),
            guild_id: GuildId::new(20),
        };

        assert_eq!(
            mock.evaluate_content(&ctx).await.unwrap(),
            RuleDecision::Pass
        );

        mock.reload().await.unwrap();
        assert!(mock.reload().await.is_ok());
    }
}
