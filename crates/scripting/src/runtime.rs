//! Runtime Python — charge et exécute les scripts utilisateur.

use pyo3::prelude::*;
use pyo3::types::PyModule;
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;
use engine::InputState;

type ObjectScriptState = world::object::SceneObject;
use crate::{
    api::{CommandQueue, PyGameContext},
    command::GameCommand,
};

/// Erreurs du runtime de script.
#[derive(Debug, Error)]
pub enum ScriptError {
    #[error("Erreur Python : {0}")]
    Python(String),
    #[error("Le script n'a pas de fonction `{0}`")]
    MissingFunction(String),
    #[error("Erreur de chargement du fichier : {0}")]
    Io(#[from] std::io::Error),
    #[error("Verrou de commande inaccessible")]
    LockError,
}

impl From<PyErr> for ScriptError {
    fn from(e: PyErr) -> Self {
        Python::with_gil(|_py| {
            ScriptError::Python(e.to_string())
        })
    }
}

/// Une instance de script chargée.
struct LoadedScript {
    /// Chemin du fichier source.
    path: std::path::PathBuf,
    /// Module Python chargé.
    module: Py<PyModule>,
    kind: ScriptKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScriptKind { Legacy, World, Object(String) }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HookTarget { Legacy, World, Objects }

/// Gère le cycle de vie des scripts Python.
///
/// Un seul `ScriptRuntime` par application. Détient le GIL quand il en a besoin,
/// mais ne le garde jamais entre les frames.
pub struct ScriptRuntime {
    scripts: Vec<LoadedScript>,
    /// File de commandes partagée.
    queue: CommandQueue,
    /// Indique si on_start a déjà été appelé.
    started: bool,
}

impl ScriptRuntime {
    pub fn new() -> Self {
        Self {
            scripts: Vec::new(),
            queue: Arc::new(Mutex::new(Vec::new())),
            started: false,
        }
    }

    /// Charge un script Python depuis un fichier.
    ///
    /// Le script peut définir `on_start(game)` et/ou `on_update(game, delta)`.
    /// Les fonctions manquantes sont silencieusement ignorées.
    pub fn load_script(&mut self, path: &Path) -> Result<(), ScriptError> {
        self.load_script_with_kind(path, ScriptKind::Legacy)
    }

    /// Charge le script unique du monde et ses hooks `_on_*`.
    pub fn load_world_script(&mut self, path: &Path) -> Result<(), ScriptError> {
        self.load_script_with_kind(path, ScriptKind::World)
    }

    /// Charge une instance de script attachée à un objet.
    pub fn load_object_script(&mut self, path: &Path, object_id: impl Into<String>) -> Result<(), ScriptError> {
        self.load_script_with_kind(path, ScriptKind::Object(object_id.into()))
    }

    fn load_script_with_kind(&mut self, path: &Path, kind: ScriptKind) -> Result<(), ScriptError> {
        let source = std::fs::read_to_string(path)?;
        let module = Python::with_gil(|py| -> PyResult<Py<PyModule>> {
            let module_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("script");
            let m = PyModule::from_code_bound(py, &source, path.to_string_lossy().as_ref(), module_name)?;
            Ok(m.unbind())
        })?;
        self.scripts.push(LoadedScript {
            path: path.to_owned(),
            module,
            kind,
        });
        log::info!("Script chargé : {}", path.display());
        Ok(())
    }

    /// Décharge tous les scripts et réinitialise l'état.
    pub fn unload_all(&mut self) {
        self.scripts.clear();
        self.started = false;
        if let Ok(mut q) = self.queue.lock() {
            q.clear();
        }
    }

    /// Appelle `on_start(game)` sur tous les scripts chargés.
    ///
    /// N'est appelé qu'une seule fois au démarrage de la simulation.
    pub fn call_start(&mut self) -> Result<Vec<GameCommand>, ScriptError> {
        if self.started {
            return Ok(Vec::new());
        }
        self.started = true;
        self.call_hook("on_start", 0.0, HookTarget::Legacy, &[], InputState::default())
    }

    pub fn call_world_launch(&self, objects: &[ObjectScriptState]) -> Result<Vec<GameCommand>, ScriptError> {
        self.call_hook("_on_game_launch", 0.0, HookTarget::World, objects, InputState::default())
    }

    pub fn call_world_update(&self, delta_time: f32, input: &InputState, objects: &[ObjectScriptState]) -> Result<Vec<GameCommand>, ScriptError> {
        self.call_hook("_on_update", delta_time, HookTarget::World, objects, input.clone())
    }

    pub fn call_object_launch(&self, objects: &[ObjectScriptState]) -> Result<Vec<GameCommand>, ScriptError> {
        self.call_hook("_on_game_launch", 0.0, HookTarget::Objects, objects, InputState::default())
    }

    pub fn call_object_update(&self, delta_time: f32, objects: &[ObjectScriptState], input: &InputState) -> Result<Vec<GameCommand>, ScriptError> {
        self.call_hook("_on_update", delta_time, HookTarget::Objects, objects, input.clone())
    }

    /// Appelle `on_update(game, delta_time)` sur tous les scripts.
    pub fn call_update(&self, delta_time: f32) -> Result<Vec<GameCommand>, ScriptError> {
        self.call_hook("on_update", delta_time, HookTarget::Legacy, &[], InputState::default())
    }

    /// Appelle `on_stop(game)` sur tous les scripts.
    pub fn call_stop(&mut self) -> Result<Vec<GameCommand>, ScriptError> {
        let cmds = self.call_hook("on_stop", 0.0, HookTarget::Legacy, &[], InputState::default())?;
        self.started = false;
        Ok(cmds)
    }

    /// Implémentation interne : appelle une fonction hook sur tous les scripts.
    fn call_hook(&self, hook: &str, delta_time: f32, target: HookTarget, objects: &[ObjectScriptState], input: InputState) -> Result<Vec<GameCommand>, ScriptError> {
        // Vide la file avant l'appel.
        {
            let mut q = self.queue.lock().map_err(|_| ScriptError::LockError)?;
            q.clear();
        }

        let mut script_errors = Vec::new();
        let scene_objects = Arc::new(objects.to_vec());
        Python::with_gil(|py| {
            for loaded in &self.scripts {
                let object_id = match (&loaded.kind, target) {
                    (ScriptKind::Legacy, HookTarget::Legacy) | (ScriptKind::World, HookTarget::World) => None,
                    (ScriptKind::Object(id), HookTarget::Objects) if objects.iter().any(|state| state.id.to_string() == *id) => Some(id.clone()),
                    _ => continue,
                };
                if (target == HookTarget::Legacy && hook.starts_with("_on_"))
                    || (target != HookTarget::Legacy && !hook.starts_with("_on_")) {
                    continue;
                }
                let module = loaded.module.bind(py);

                // Vérifie si la fonction existe.
                let func = match module.getattr(hook) {
                    Ok(f) => f,
                    Err(_) => {
                        // Fonction absente : c'est OK, on passe au script suivant.
                        log::debug!("Script `{}` : pas de `{}`", loaded.path.display(), hook);
                        continue;
                    }
                };

                // Crée un GameContext frais pour cet appel.
                let self_object = object_id.as_ref().and_then(|id| {
                    objects.iter().find(|state| state.id.to_string() == *id).cloned()
                });
                let tile_position = self_object.as_ref().map(|object| {
                    (object.tile_pos.x, object.tile_pos.y, object.tile_pos.z)
                });
                let script_file = loaded.path.file_name().and_then(|name| name.to_str())
                    .unwrap_or_else(|| loaded.path.to_str().unwrap_or("script Python"));
                let script_source = self_object.as_ref()
                    .map(|object| format!("{script_file} — {}", object.name))
                    .unwrap_or_else(|| script_file.to_owned());
                if hook == "_on_game_launch" && target == HookTarget::Objects {
                    log::info!("Appel de _on_game_launch pour l'objet {} ({})", object_id.as_deref().unwrap_or("?"), loaded.path.display());
                }
                let ctx = PyGameContext::new(
                    Arc::clone(&self.queue), delta_time, object_id, tile_position,
                    self_object, Arc::clone(&scene_objects), input.clone(), script_source,
                );
                let py_ctx = Py::new(py, ctx)
                    .map_err(|e| ScriptError::Python(e.to_string()))?;

                // Appelle la fonction selon le hook.
                let result = if hook == "on_update" || hook == "_on_update" {
                    func.call1((py_ctx, delta_time))
                } else {
                    func.call1((py_ctx,))
                };

                // Capture et logue les erreurs Python sans crasher le moteur.
                if let Err(e) = result {
                    script_errors.push(format!("{} ({hook}) : {e}", loaded.path.display()));
                }
            }
            Ok::<(), ScriptError>(())
        })?;

        // Récupère toutes les commandes émises.
        let cmds = self.queue.lock().map_err(|_| ScriptError::LockError)?.drain(..).collect();
        if !script_errors.is_empty() {
            return Err(ScriptError::Python(script_errors.join("\n")));
        }
        Ok(cmds)
    }
}

impl Default for ScriptRuntime {
    fn default() -> Self {
        Self::new()
    }
}
