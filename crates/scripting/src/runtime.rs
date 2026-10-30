//! Runtime Python — charge et exécute les scripts utilisateur.

use engine::InputState;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use std::collections::HashSet;
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;

type ObjectScriptState = world::object::SceneObject;
use crate::{
    api::{CommandQueue, GlobalStore, PyGameContext, SceneSnapshot},
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
        Python::with_gil(|_py| ScriptError::Python(e.to_string()))
    }
}

/// Une instance de script chargée.
struct LoadedScript {
    /// Chemin du fichier source.
    path: std::path::PathBuf,
    /// Module Python chargé.
    module: Py<PyModule>,
    kind: ScriptKind,
    hooks: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScriptKind {
    Legacy,
    World,
    Object(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HookTarget {
    Legacy,
    World,
    Objects,
}

/// Gère le cycle de vie des scripts Python.
///
/// Un seul `ScriptRuntime` par application. Détient le GIL quand il en a besoin,
/// mais ne le garde jamais entre les frames.
pub struct ScriptRuntime {
    scripts: Vec<LoadedScript>,
    /// File de commandes partagée.
    queue: CommandQueue,
    scenes: Arc<Vec<SceneSnapshot>>,
    globals: GlobalStore,
    /// Indique si on_start a déjà été appelé.
    started: bool,
}

impl ScriptRuntime {
    pub fn new() -> Self {
        Self {
            scripts: Vec::new(),
            queue: Arc::new(Mutex::new(Vec::new())),
            scenes: Arc::new(Vec::new()),
            globals: Arc::new(Mutex::new(std::collections::HashMap::new())),
            started: false,
        }
    }

    pub fn set_scenes(&mut self, scenes: Vec<SceneSnapshot>) {
        self.scenes = Arc::new(scenes);
    }

    pub fn clear_globals(&mut self) {
        if let Ok(mut globals) = self.globals.lock() {
            globals.clear();
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
    pub fn load_object_script(
        &mut self,
        path: &Path,
        object_id: impl Into<String>,
    ) -> Result<(), ScriptError> {
        self.load_script_with_kind(path, ScriptKind::Object(object_id.into()))
    }

    fn load_script_with_kind(&mut self, path: &Path, kind: ScriptKind) -> Result<(), ScriptError> {
        let source = std::fs::read_to_string(path)?;
        let module = Python::with_gil(|py| -> PyResult<Py<PyModule>> {
            let module_name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("script");
            let m = PyModule::from_code_bound(
                py,
                &source,
                path.to_string_lossy().as_ref(),
                module_name,
            )?;
            Ok(m.unbind())
        })?;
        let hooks = Python::with_gil(|py| {
            let module = module.bind(py);
            [
                "on_update",
                "_on_update",
                "on_start",
                "_on_game_launch",
                "on_stop",
                "_on_area_entered",
                "_on_area_stayed",
                "_on_area_exited",
                "_on_click",
            ]
            .into_iter()
            .filter_map(|hook| {
                module
                    .hasattr(hook)
                    .ok()
                    .filter(|exists| *exists)
                    .map(|_| hook.to_owned())
            })
            .collect()
        });
        self.scripts.push(LoadedScript {
            path: path.to_owned(),
            module,
            kind,
            hooks,
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
        self.clear_globals();
    }

    /// Appelle `on_start(game)` sur tous les scripts chargés.
    ///
    /// N'est appelé qu'une seule fois au démarrage de la simulation.
    pub fn call_start(&mut self) -> Result<Vec<GameCommand>, ScriptError> {
        if self.started {
            return Ok(Vec::new());
        }
        self.started = true;
        self.call_hook(
            "on_start",
            0.0,
            HookTarget::Legacy,
            &[],
            InputState::default(),
        )
    }

    pub fn call_world_launch(
        &self,
        objects: &[ObjectScriptState],
    ) -> Result<Vec<GameCommand>, ScriptError> {
        self.call_hook(
            "_on_game_launch",
            0.0,
            HookTarget::World,
            objects,
            InputState::default(),
        )
    }

    pub fn call_world_update(
        &self,
        delta_time: f32,
        input: &InputState,
        objects: &[ObjectScriptState],
    ) -> Result<Vec<GameCommand>, ScriptError> {
        self.call_hook(
            "_on_update",
            delta_time,
            HookTarget::World,
            objects,
            input.clone(),
        )
    }

    pub fn call_object_launch(
        &self,
        objects: &[ObjectScriptState],
    ) -> Result<Vec<GameCommand>, ScriptError> {
        self.call_hook(
            "_on_game_launch",
            0.0,
            HookTarget::Objects,
            objects,
            InputState::default(),
        )
    }

    pub fn call_object_update(
        &self,
        delta_time: f32,
        objects: &[ObjectScriptState],
        input: &InputState,
    ) -> Result<Vec<GameCommand>, ScriptError> {
        self.call_hook(
            "_on_update",
            delta_time,
            HookTarget::Objects,
            objects,
            input.clone(),
        )
    }

    /// Appelle `on_update(game, delta_time)` sur tous les scripts.
    pub fn call_update(&self, delta_time: f32) -> Result<Vec<GameCommand>, ScriptError> {
        self.call_hook(
            "on_update",
            delta_time,
            HookTarget::Legacy,
            &[],
            InputState::default(),
        )
    }

    pub fn has_legacy_update(&self) -> bool {
        self.has_hook("on_update", HookTarget::Legacy)
    }

    pub fn has_world_update(&self) -> bool {
        self.has_hook("_on_update", HookTarget::World)
    }

    pub fn has_object_update(&self) -> bool {
        self.has_hook("_on_update", HookTarget::Objects)
    }

    pub fn has_any_area_hooks(&self) -> bool {
        self.scripts.iter().any(|script| {
            ["_on_area_entered", "_on_area_stayed", "_on_area_exited"]
                .iter()
                .any(|hook| script.hooks.contains(*hook))
        })
    }

    pub fn object_has_area_hooks(&self, object_id: &str) -> bool {
        self.scripts.iter().any(|script| {
            matches!(&script.kind, ScriptKind::Object(id) if id == object_id)
                && ["_on_area_entered", "_on_area_stayed", "_on_area_exited"]
                    .iter()
                    .any(|hook| script.hooks.contains(*hook))
        })
    }

    /// Appelle `_on_click(game)` sur le script attaché à l'objet cliqué.
    pub fn call_object_click(
        &self,
        object_id: &str,
        objects: &[ObjectScriptState],
        input: &InputState,
    ) -> Result<Vec<GameCommand>, ScriptError> {
        let Some(object) = objects
            .iter()
            .find(|object| object.id.to_string() == object_id)
            .cloned()
        else {
            return Ok(Vec::new());
        };
        if !self.scripts.iter().any(|script| {
            matches!(&script.kind, ScriptKind::Object(id) if id == object_id)
                && script.hooks.contains("_on_click")
        }) {
            return Ok(Vec::new());
        }

        let mut queue = self.queue.lock().map_err(|_| ScriptError::LockError)?;
        queue.clear();
        drop(queue);

        let scene_objects = Arc::new(objects.to_vec());
        let mut script_errors = Vec::new();
        Python::with_gil(|py| {
            for loaded in &self.scripts {
                let ScriptKind::Object(id) = &loaded.kind else {
                    continue;
                };
                if id != object_id || !loaded.hooks.contains("_on_click") {
                    continue;
                }
                let module = loaded.module.bind(py);
                let Ok(function) = module.getattr("_on_click") else {
                    continue;
                };
                let script_file = loaded
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("script Python");
                let ctx = PyGameContext::new(
                    Arc::clone(&self.queue),
                    0.0,
                    Some(object_id.to_owned()),
                    Some((object.tile_pos.x, object.tile_pos.y, object.tile_pos.z)),
                    Some(object.clone()),
                    Arc::clone(&scene_objects),
                    Arc::clone(&self.scenes),
                    Arc::clone(&self.globals),
                    input.clone(),
                    format!("{script_file} — {}", object.name),
                );
                let py_ctx =
                    Py::new(py, ctx).map_err(|error| ScriptError::Python(error.to_string()))?;
                if let Err(error) = function.call1((py_ctx,)) {
                    script_errors.push(format!("{} (_on_click) : {error}", loaded.path.display()));
                }
            }
            Ok::<(), ScriptError>(())
        })?;

        let commands = self
            .queue
            .lock()
            .map_err(|_| ScriptError::LockError)?
            .drain(..)
            .collect();
        if script_errors.is_empty() {
            Ok(commands)
        } else {
            Err(ScriptError::Python(script_errors.join("\n")))
        }
    }

    /// Appelle un événement de zone sur le script de l'objet qui porte la zone.
    pub fn call_object_area_event(
        &self,
        hook: &str,
        area_id: &str,
        other_id: &str,
        delta_time: f32,
        objects: &[ObjectScriptState],
        input: &InputState,
    ) -> Result<Vec<GameCommand>, ScriptError> {
        if !["_on_area_entered", "_on_area_stayed", "_on_area_exited"].contains(&hook) {
            return Err(ScriptError::Python(format!(
                "Événement de zone inconnu : {hook}"
            )));
        }
        let Some(other) = objects
            .iter()
            .find(|object| object.id.to_string() == other_id)
            .cloned()
        else {
            return Ok(Vec::new());
        };
        let mut queue = self.queue.lock().map_err(|_| ScriptError::LockError)?;
        queue.clear();
        drop(queue);

        let scene_objects = Arc::new(objects.to_vec());
        let mut script_errors = Vec::new();
        Python::with_gil(|py| {
            for loaded in &self.scripts {
                let ScriptKind::Object(id) = &loaded.kind else {
                    continue;
                };
                if id != area_id || !loaded.hooks.contains(hook) {
                    continue;
                }
                let Some(area) = objects
                    .iter()
                    .find(|object| object.id.to_string() == *id)
                    .cloned()
                else {
                    continue;
                };
                let module = loaded.module.bind(py);
                let func = match module.getattr(hook) {
                    Ok(function) => function,
                    Err(_) => continue,
                };
                let script_file = loaded
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("script Python");
                let script_source = format!("{script_file} — {}", area.name);
                let ctx = PyGameContext::new(
                    Arc::clone(&self.queue),
                    delta_time,
                    Some(id.clone()),
                    Some((area.tile_pos.x, area.tile_pos.y, area.tile_pos.z)),
                    Some(area),
                    Arc::clone(&scene_objects),
                    Arc::clone(&self.scenes),
                    Arc::clone(&self.globals),
                    input.clone(),
                    script_source,
                );
                let py_ctx =
                    Py::new(py, ctx).map_err(|error| ScriptError::Python(error.to_string()))?;
                let py_other = Py::new(py, crate::api::PyGameObject::from_object(other.clone()))
                    .map_err(|error| ScriptError::Python(error.to_string()))?;
                if let Err(error) = func.call1((py_ctx, py_other)) {
                    script_errors.push(format!("{} ({hook}) : {error}", loaded.path.display()));
                }
            }
            Ok::<(), ScriptError>(())
        })?;

        let commands = self
            .queue
            .lock()
            .map_err(|_| ScriptError::LockError)?
            .drain(..)
            .collect();
        if script_errors.is_empty() {
            Ok(commands)
        } else {
            Err(ScriptError::Python(script_errors.join("\n")))
        }
    }

    fn has_hook(&self, hook: &str, target: HookTarget) -> bool {
        self.scripts.iter().any(|script| {
            script.hooks.contains(hook)
                && matches!(
                    (&script.kind, target),
                    (ScriptKind::Legacy, HookTarget::Legacy)
                        | (ScriptKind::World, HookTarget::World)
                        | (ScriptKind::Object(_), HookTarget::Objects)
                )
        })
    }

    /// Appelle `on_stop(game)` sur tous les scripts.
    pub fn call_stop(&mut self) -> Result<Vec<GameCommand>, ScriptError> {
        let result = self.call_hook(
            "on_stop",
            0.0,
            HookTarget::Legacy,
            &[],
            InputState::default(),
        );
        self.started = false;
        self.clear_globals();
        result
    }

    /// Implémentation interne : appelle une fonction hook sur tous les scripts.
    fn call_hook(
        &self,
        hook: &str,
        delta_time: f32,
        target: HookTarget,
        objects: &[ObjectScriptState],
        input: InputState,
    ) -> Result<Vec<GameCommand>, ScriptError> {
        if !self.has_hook(hook, target) {
            return Ok(Vec::new());
        }
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
                    (ScriptKind::Legacy, HookTarget::Legacy)
                    | (ScriptKind::World, HookTarget::World) => None,
                    (ScriptKind::Object(id), HookTarget::Objects)
                        if objects.iter().any(|state| state.id.to_string() == *id) =>
                    {
                        Some(id.clone())
                    }
                    _ => continue,
                };
                if (target == HookTarget::Legacy && hook.starts_with("_on_"))
                    || (target != HookTarget::Legacy && !hook.starts_with("_on_"))
                {
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
                    objects
                        .iter()
                        .find(|state| state.id.to_string() == *id)
                        .cloned()
                });
                let tile_position = self_object
                    .as_ref()
                    .map(|object| (object.tile_pos.x, object.tile_pos.y, object.tile_pos.z));
                let script_file = loaded
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_else(|| loaded.path.to_str().unwrap_or("script Python"));
                let script_source = self_object
                    .as_ref()
                    .map(|object| format!("{script_file} — {}", object.name))
                    .unwrap_or_else(|| script_file.to_owned());
                if hook == "_on_game_launch" && target == HookTarget::Objects {
                    log::info!(
                        "Appel de _on_game_launch pour l'objet {} ({})",
                        object_id.as_deref().unwrap_or("?"),
                        loaded.path.display()
                    );
                }
                let ctx = PyGameContext::new(
                    Arc::clone(&self.queue),
                    delta_time,
                    object_id,
                    tile_position,
                    self_object,
                    Arc::clone(&scene_objects),
                    Arc::clone(&self.scenes),
                    Arc::clone(&self.globals),
                    input.clone(),
                    script_source,
                );
                let py_ctx = Py::new(py, ctx).map_err(|e| ScriptError::Python(e.to_string()))?;

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
        let cmds = self
            .queue
            .lock()
            .map_err(|_| ScriptError::LockError)?
            .drain(..)
            .collect();
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
