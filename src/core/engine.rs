use anyhow::{Context, Result};
use rhai::{Engine, Scope, AST};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

pub struct RhaiManager {
    engine: Engine,
    modules_dir: PathBuf,
    ast_cache: RwLock<HashMap<String, AST>>,
}

impl RhaiManager {
    pub fn new<P: AsRef<Path>>(modules_dir: P) -> Self {
        let mut engine = Engine::new();
        engine.set_max_operations(100_000);
        engine.set_max_call_levels(50);
        
        // Register all host native bindings
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
                let ast = self.engine.compile(&content)
                    .with_context(|| format!("Failed to compile Rhai script: {}", path.display()))?;
                
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
                Err(anyhow::anyhow!("Rhai execution error in {}.{}: {}", module_name, fn_name, err))
            }
        }
    }
}
