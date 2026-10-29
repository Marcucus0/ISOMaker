//! Application principale de l'éditeur.
//!
//! Gère :
//! - Le layout egui (toolbar, panneau gauche, vue centrale, inspecteur)
//! - L'état global de l'éditeur
//! - Le dispatch des actions vers la scène et le runtime de scripts

use egui::Context;
use engine::game_loop::{EngineMode, GameLoop};
use engine::InputState;
use serde::{Deserialize, Serialize};
use scripting::{ScriptRuntime, GameCommand};
use std::collections::VecDeque;
use world::{
    object::{ObjectId, ObjectType},
    project::Project,
    scene::Scene,
    coords::TilePos,
};
use crate::{
    inspector::Inspector,
    panels::{LeftPanel, PanelAction},
    scene_view::SceneView,
    toolbar::Toolbar,
};

/// État persistant de l'éditeur entre les frames.
pub struct EditorState {
    /// Projet courant contenant un monde et ses scènes.
    pub project: Project,
    /// Objet sélectionné.
    pub selected: Option<ObjectId>,
    /// Runtime Python.
    pub script_runtime: ScriptRuntime,
    /// Boucle de jeu / mode.
    pub game_loop: GameLoop,
    /// État clavier transmis aux scripts pendant Lecture.
    pub input: InputState,
    /// Instantané de la scène au démarrage du play (pour restauration au Stop).
    scene_snapshot: Option<String>,
    /// Messages de log accumulés.
    pub log_messages: VecDeque<LogEntry>,
    /// Historique des états de scène pour annuler/rétablir les éditions.
    undo_history: Vec<String>,
    redo_history: Vec<String>,
}

pub struct LogEntry {
    pub level: log::Level,
    pub message: String,
    pub repetitions: usize,
}

impl EditorState {
    pub fn new() -> Self {
        let mut project = Project::new("Mon projet");
        let scene = project.world.active_scene_mut();
        // Pré-peupler avec quelques tuiles de démonstration.
        use world::grid::TileData;
        for x in -3..=3 {
            for y in -3..=3 {
                scene.set_tile(TilePos::new(x, y), TileData::terrain(1));
            }
        }
        Self {
            project,
            selected: None,
            script_runtime: ScriptRuntime::new(),
            game_loop: GameLoop::new(),
            input: InputState::new(),
            scene_snapshot: None,
            log_messages: VecDeque::new(),
            undo_history: Vec::new(),
            redo_history: Vec::new(),
        }
    }

    pub fn add_scene(&mut self) -> usize {
        let index = self.project.world.scenes.len() + 1;
        self.project.world.add_scene(format!("Scène {index}"))
    }

    pub fn active_scene(&self) -> &Scene {
        self.project.world.active_scene()
    }

    pub fn active_scene_mut(&mut self) -> &mut Scene {
        self.project.world.active_scene_mut()
    }

    pub fn select_scene(&mut self, index: usize) -> bool {
        if !self.project.world.select_scene(index) {
            return false;
        }
        self.selected = None;
        self.scene_snapshot = None;
        self.undo_history.clear();
        self.redo_history.clear();
        true
    }

    /// Démarre la simulation.
    pub fn play(&mut self) {
        if self.game_loop.mode == EngineMode::Editor {
            self.game_loop.play();
        }
        // Le toolbar applique déjà la transition de mode. L'absence de snapshot
        // distingue un nouveau Play d'une reprise après pause.
        if self.game_loop.mode == EngineMode::Playing && self.scene_snapshot.is_none() {
            // Sauvegarde un instantané de la scène pour restauration au Stop.
            self.scene_snapshot = self.active_scene().to_json().ok();
            match self.script_runtime.call_start() {
                Ok(cmds) => self.apply_commands(cmds),
                Err(e) => self.push_log(log::Level::Error, format!("on_start : {e}")),
            }
            let objects = self.active_scene().objects.values().cloned().collect::<Vec<_>>();
            match self.script_runtime.call_world_launch(&objects) {
                Ok(cmds) => self.apply_commands(cmds),
                Err(e) => self.push_log(log::Level::Error, format!("_on_game_launch : {e}")),
            }
            let objects = self.active_scene().objects.values().cloned().collect::<Vec<_>>();
            match self.script_runtime.call_object_launch(&objects) {
                Ok(cmds) => self.apply_commands(cmds),
                Err(e) => self.push_log(log::Level::Error, format!("Script objet _on_game_launch : {e}")),
            }
        }
    }

    /// Met en pause.
    pub fn pause(&mut self) { self.game_loop.pause(); }

    /// Reprend depuis la pause.
    pub fn resume(&mut self) { self.game_loop.resume(); }

    /// Arrête la simulation et restaure la scène de l'éditeur.
    pub fn stop(&mut self) {
        // La barre d'outils peut avoir déjà basculé GameLoop en mode Editor
        // avant d'appeler cette méthode. L'instantané indique alors qu'une
        // session doit encore être nettoyée malgré l'absence de transition ici.
        let has_active_session = self.game_loop.mode != EngineMode::Editor
            || self.scene_snapshot.is_some();
        self.game_loop.stop();
        if !has_active_session {
            return;
        }

        // Appelle on_stop.
        let cmds = self.script_runtime.call_stop().unwrap_or_default();
        // On ignore les commandes au Stop pour ne pas corrompre la restauration.
        let _ = cmds;
        self.script_runtime.unload_all();

        // Restaure la scène.
        if let Some(snapshot) = self.scene_snapshot.take() {
            match Scene::from_json(&snapshot) {
                Ok(restored) => *self.active_scene_mut() = restored,
                Err(e) => self.push_log(log::Level::Error, format!("Restauration : {e}")),
            }
        }
        self.selected = None;
    }

    /// Appelé chaque frame pendant Playing.
    pub fn update(&mut self, delta_time: f32) {
        if !self.game_loop.is_simulating() { return; }
        match self.script_runtime.call_update(delta_time) {
            Ok(cmds) => self.apply_commands(cmds),
            Err(e) => self.push_log(log::Level::Error, format!("on_update : {e}")),
        }
        let objects = self.active_scene().objects.values().cloned().collect::<Vec<_>>();
        match self.script_runtime.call_world_update(delta_time, &self.input, &objects) {
            Ok(cmds) => self.apply_commands(cmds),
            Err(e) => self.push_log(log::Level::Error, format!("_on_update : {e}")),
        }
        let objects = self.active_scene().objects.values().cloned().collect::<Vec<_>>();
        match self.script_runtime.call_object_update(delta_time, &objects, &self.input) {
            Ok(cmds) => self.apply_commands(cmds),
            Err(e) => self.push_log(log::Level::Error, format!("Script objet _on_update : {e}")),
        }
    }

    /// Applique les commandes Python sur la scène.
    fn apply_commands(&mut self, cmds: Vec<GameCommand>) {
        for cmd in cmds {
            match cmd {
                GameCommand::Spawn { name, object_type, x, y } => {
                    let otype = match object_type.as_str() {
                        "Building"   => ObjectType::Building,
                        "Character"  => ObjectType::Character,
                        "Item"       => ObjectType::Item,
                        "Decoration" => ObjectType::Decoration,
                        other        => ObjectType::Custom(other.to_string()),
                    };
                    self.active_scene_mut().spawn(name, otype, TilePos::new(x, y));
                }
                GameCommand::Destroy { object_id } => {
                    if let Ok(uuid) = uuid::Uuid::parse_str(&object_id) {
                        self.active_scene_mut().remove_object(world::object::ObjectId(uuid));
                    }
                }
                GameCommand::Move { object_id, x, y } => {
                    if let Ok(uuid) = uuid::Uuid::parse_str(&object_id) {
                        let id = world::object::ObjectId(uuid);
                        if let Some(obj) = self.active_scene_mut().get_object_mut(id) {
                            obj.tile_pos.x = x;
                            obj.tile_pos.y = y;
                        }
                    }
                }
                GameCommand::SetProperty { object_id, key, value } => {
                    if let Ok(uuid) = uuid::Uuid::parse_str(&object_id) {
                        let id = world::object::ObjectId(uuid);
                        let result = match self.active_scene_mut().get_object_mut(id) {
                            Some(obj) => apply_script_object_property(obj, &key, value),
                            None => Err(format!("Objet introuvable : {object_id}")),
                        };
                        if let Err(error) = result {
                            self.push_log(log::Level::Error, format!("Propriété objet `{key}` : {error}"));
                        }
                    }
                }
                GameCommand::SetGlobal { key, value } => {
                    self.push_log(
                        log::Level::Debug,
                        format!("SetGlobal: {} = {:?}", key, value),
                    );
                }
                GameCommand::Log { source, level, message } => {
                    let lvl = match level {
                        scripting::command::LogLevel::Debug   => log::Level::Debug,
                        scripting::command::LogLevel::Info    => log::Level::Info,
                        scripting::command::LogLevel::Warning => log::Level::Warn,
                        scripting::command::LogLevel::Error   => log::Level::Error,
                    };
                    self.push_script_log(lvl, &source, message);
                }
                GameCommand::Quit => { self.stop(); }
            }
        }
    }

    fn push_log(&mut self, level: log::Level, msg: String) {
        log::log!(level, "[Script] {}", msg);
        self.store_log(level, msg);
    }

    fn push_script_log(&mut self, level: log::Level, source: &str, msg: String) {
        let formatted = format!("[{source}] {msg}");
        // Les messages d'information restent dans le panneau egui ; les
        // recopier au logger natif provoquerait des écritures console par frame.
        if matches!(level, log::Level::Warn | log::Level::Error) {
            log::log!(level, "[Script] {}", formatted);
        }
        self.store_log(level, formatted);
    }

    fn store_log(&mut self, level: log::Level, msg: String) {
        if let Some(last) = self.log_messages.back_mut() {
            if last.level == level && last.message == msg {
                last.repetitions += 1;
                return;
            }
        }
        const LOG_LIMIT: usize = 200;
        if self.log_messages.len() >= LOG_LIMIT {
            self.log_messages.pop_front();
        }
        self.log_messages.push_back(LogEntry { level, message: msg, repetitions: 1 });
    }

    fn record_undo(&mut self, snapshot: String) {
        const HISTORY_LIMIT: usize = 128;
        self.undo_history.push(snapshot);
        if self.undo_history.len() > HISTORY_LIMIT {
            self.undo_history.remove(0);
        }
        self.redo_history.clear();
    }

    fn undo(&mut self) {
        if let Some(previous) = self.undo_history.pop() {
            if let Ok(current) = self.active_scene().to_json() {
                self.redo_history.push(current);
            }
            match Scene::from_json(&previous) {
                Ok(scene) => *self.active_scene_mut() = scene,
                Err(error) => self.push_log(log::Level::Error, format!("Annulation : {error}")),
            }
        }
    }

    fn redo(&mut self) {
        if let Some(next) = self.redo_history.pop() {
            if let Ok(current) = self.active_scene().to_json() {
                self.undo_history.push(current);
            }
            match Scene::from_json(&next) {
                Ok(scene) => *self.active_scene_mut() = scene,
                Err(error) => self.push_log(log::Level::Error, format!("Rétablissement : {error}")),
            }
        }
    }
}

fn json_to_prop(v: serde_json::Value) -> world::object::PropValue {
    use world::object::PropValue;
    match v {
        serde_json::Value::Null        => PropValue::Str(String::new()),
        serde_json::Value::Bool(b)     => PropValue::Bool(b),
        serde_json::Value::Number(n)   => {
            if let Some(i) = n.as_i64() { PropValue::Int(i) }
            else { PropValue::Float(n.as_f64().unwrap_or(0.0)) }
        }
        serde_json::Value::String(s)   => PropValue::Str(s),
        serde_json::Value::Array(arr)  => PropValue::List(arr.into_iter().map(json_to_prop).collect()),
        serde_json::Value::Object(_)   => PropValue::Str("[object]".to_string()),
    }
}

fn apply_script_object_property(
    object: &mut world::object::SceneObject,
    key: &str,
    value: serde_json::Value,
) -> Result<(), String> {
    use world::{layer::Layer, object::{PropValue, Rotation4}};

    match key {
        "id" | "object_id" | "type" | "object_type" => {
            Err(format!("`{key}` est en lecture seule"))
        }
        "name" => {
            object.name = value.as_str().ok_or("attend une chaîne")?.to_owned();
            Ok(())
        }
        "x" => {
            object.tile_pos.x = value.as_i64().and_then(|v| i32::try_from(v).ok()).ok_or("attend un entier")?;
            Ok(())
        }
        "y" => {
            object.tile_pos.y = value.as_i64().and_then(|v| i32::try_from(v).ok()).ok_or("attend un entier")?;
            Ok(())
        }
        "z" => {
            object.tile_pos.z = value.as_i64().and_then(|v| i32::try_from(v).ok()).ok_or("attend un entier")?;
            Ok(())
        }
        "camera_zoom" => {
            let zoom = value.as_f64().ok_or("attend un nombre")? as f32;
            if !zoom.is_finite() || !(0.1..=10.0).contains(&zoom) {
                return Err("camera_zoom doit être compris entre 0.1 et 10.0".to_owned());
            }
            object.camera_zoom = zoom;
            Ok(())
        }
        "camera_interpolation" => {
            object.camera_interpolation = value.as_bool().ok_or("attend True ou False")?;
            Ok(())
        }
        "camera_interpolation_speed" => {
            let speed = value.as_f64().ok_or("attend un nombre")? as f32;
            if !speed.is_finite() || !(0.1..=20.0).contains(&speed) {
                return Err("camera_interpolation_speed doit être compris entre 0.1 et 20.0".to_owned());
            }
            object.camera_interpolation_speed = speed;
            Ok(())
        }
        "has_collisions" => {
            object.has_collisions = value.as_bool().ok_or("attend True ou False")?;
            Ok(())
        }
        "hidden" => {
            object.hidden = value.as_bool().ok_or("attend True ou False")?;
            Ok(())
        }
        "fit_texture_to_diamond" => {
            object.fit_texture_to_diamond = value.as_bool().ok_or("attend True ou False")?;
            Ok(())
        }
        "rotation" => {
            object.rotation = match value.as_str().ok_or("attend North, East, South ou West")? {
                "North" => Rotation4::North,
                "East" => Rotation4::East,
                "South" => Rotation4::South,
                "West" => Rotation4::West,
                _ => return Err("valeur attendue : North, East, South ou West".to_owned()),
            };
            Ok(())
        }
        "layer" => {
            object.layer = match value.as_str().ok_or("attend un nom de couche")? {
                "Terrain" => Layer::Terrain,
                "Decoration" => Layer::Decoration,
                "Ground" => Layer::Ground,
                "Objects" => Layer::Objects,
                "Buildings" => Layer::Buildings,
                "Foreground" => Layer::Foreground,
                "Overlay" => Layer::Overlay,
                _ => return Err("couche inconnue : Terrain, Decoration, Ground, Objects, Buildings, Foreground ou Overlay".to_owned()),
            };
            Ok(())
        }
        "texture" | "asset_path" => {
            let path = match value {
                serde_json::Value::Null => None,
                serde_json::Value::String(path) if path.is_empty() => None,
                serde_json::Value::String(path) => Some(path),
                _ => return Err("attend un chemin texte ou None".to_owned()),
            };
            object.asset_path = path.clone();
            object.set_prop("texture", PropValue::Str(path.unwrap_or_default()));
            Ok(())
        }
        "script" => {
            object.script = match value {
                serde_json::Value::Null => None,
                serde_json::Value::String(path) if path.is_empty() => None,
                serde_json::Value::String(path) => Some(path),
                _ => return Err("attend un chemin texte ou None".to_owned()),
            };
            Ok(())
        }
        "footprint_width" | "footprint_height" => {
            let size = value.as_u64().and_then(|v| u32::try_from(v).ok()).ok_or("attend un entier positif")?;
            if key == "footprint_width" { object.footprint.width = size; }
            else { object.footprint.height = size; }
            Ok(())
        }
        _ => {
            object.set_prop(key, json_to_prop(value));
            Ok(())
        }
    }
}

// ── EditorApp ─────────────────────────────────────────────────────────────────

/// Application éditeur complète (egui).
pub struct EditorApp {
    pub state: EditorState,
    toolbar: Toolbar,
    left_panel: LeftPanel,
    scene_view: SceneView,
    show_log: bool,
    show_world_settings: bool,
    show_editor_settings: bool,
    editor_settings: EditorSettings,
    fps_estimate: f32,
    runtime_camera: Option<ObjectId>,
    runtime_camera_position: Option<egui::Vec2>,
    runtime_camera_zoom: Option<f32>,
    pending_deletion: Option<(Vec<ObjectId>, Vec<TilePos>)>,
    /// Chemin du projet courant pour la sauvegarde.
    project_path: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct EditorSettings {
    hide_grid_during_play: bool,
    recent_projects: Vec<String>,
    grid_color: [u8; 4],
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            hide_grid_during_play: false,
            recent_projects: Vec::new(),
            grid_color: [180, 200, 220, 80],
        }
    }
}

impl EditorApp {
    pub fn new() -> Self {
        let editor_settings = load_editor_settings();
        let mut scene_view = SceneView::new();
        scene_view.options.grid_color = egui::Color32::from_rgba_unmultiplied(
            editor_settings.grid_color[0],
            editor_settings.grid_color[1],
            editor_settings.grid_color[2],
            editor_settings.grid_color[3],
        );
        Self {
            state: EditorState::new(),
            toolbar: Toolbar::new(),
            left_panel: LeftPanel::new(),
            scene_view,
            show_log: true,
            show_world_settings: false,
            show_editor_settings: false,
            editor_settings,
            fps_estimate: 0.0,
            runtime_camera: None,
            runtime_camera_position: None,
            runtime_camera_zoom: None,
            pending_deletion: None,
            project_path: None,
        }
    }

    /// Appelé à chaque frame par la boucle winit.
    pub fn update(&mut self, ctx: &Context, delta_time: f32) {
        if delta_time.is_finite() && delta_time > 0.0 {
            let instantaneous = 1.0 / delta_time;
            self.fps_estimate = if self.fps_estimate == 0.0 {
                instantaneous
            } else {
                self.fps_estimate * 0.9 + instantaneous * 0.1
            };
        }
        // Met à jour la simulation si en cours.
        self.state.update(delta_time);

        // Synchronise la sélection avec la vue scène.
        self.scene_view.selected_object = self.state.selected;
        // Synchronise le type de tuile depuis la palette.
        self.scene_view.paint_tile_type = self.left_panel.selected_tile_type;

        self.draw_ui(ctx, delta_time);

        // Récupère la sélection de la vue scène.
        self.state.selected = self.scene_view.selected_object;
    }

    /// Capture Ctrl+C/Ctrl+V avant qu'egui-winit les traite comme des raccourcis
    /// du presse-papiers texte.
    pub fn queue_tile_clipboard_shortcut(&mut self, copy: bool) -> bool {
        if !self.state.game_loop.is_editor()
            || self.toolbar.active_tool != crate::toolbar::EditorTool::Select
        {
            return false;
        }
        self.scene_view.request_tile_clipboard_shortcut(copy);
        true
    }

    fn draw_ui(&mut self, ctx: &Context, delta_time: f32) {
        let scene_before_ui = self.state.game_loop.is_editor()
            .then(|| self.state.active_scene().to_json().ok())
            .flatten();
        let mut history_action = false;
        if self.state.game_loop.is_editor() && !ctx.wants_keyboard_input() {
            let history_shortcut = ctx.input(|input| {
                if !input.modifiers.ctrl || input.modifiers.alt || input.modifiers.mac_cmd {
                    return None;
                }
                if input.key_pressed(egui::Key::Z) {
                    Some(false)
                } else if input.key_pressed(egui::Key::Y) {
                    Some(true)
                } else {
                    None
                }
            });
            if let Some(redo) = history_shortcut {
                if redo { self.state.redo(); }
                else { self.state.undo(); }
                history_action = true;
            }
        }
        if self.state.game_loop.is_editor() && !ctx.wants_keyboard_input() {
            let tool_shortcut = ctx.input(|input| {
                if input.modifiers.ctrl || input.modifiers.alt || input.modifiers.mac_cmd {
                    return None;
                }
                if input.key_pressed(egui::Key::S) {
                    Some(crate::toolbar::EditorTool::Select)
                } else if input.key_pressed(egui::Key::D) {
                    Some(crate::toolbar::EditorTool::Move)
                } else if input.key_pressed(egui::Key::R) {
                    Some(crate::toolbar::EditorTool::Rotate)
                } else {
                    None
                }
            });
            if let Some(tool) = tool_shortcut {
                self.toolbar.active_tool = tool;
            }
        }
        let save_shortcut = self.state.game_loop.is_editor() && !ctx.wants_keyboard_input()
            && ctx.input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::S));
        let scene_names: Vec<String> = self.state.project.world.scenes.iter()
            .map(|scene| scene.meta.name.clone())
            .collect();
        let recent_projects: Vec<std::path::PathBuf> = self.editor_settings.recent_projects.iter()
            .map(std::path::PathBuf::from)
            .collect();
        let active_scene_index = self.state.project.world.active_scene_index;
        // ── Barre d'outils ────────────────────────────────────────────────────
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            // Intercept les clics Play/Pause/Stop.
            let mode_before = self.state.game_loop.mode;
            self.toolbar.show(
                ui,
                &mut self.state.game_loop,
                &scene_names,
                active_scene_index,
                &recent_projects,
            );
            if self.toolbar.undo_requested && self.state.game_loop.mode == EngineMode::Editor {
                self.state.undo();
                history_action = true;
            } else if self.toolbar.redo_requested && self.state.game_loop.mode == EngineMode::Editor {
                self.state.redo();
                history_action = true;
            }
            if self.state.game_loop.mode == EngineMode::Editor {
                if self.toolbar.new_scene_requested {
                    self.state.add_scene();
                    self.scene_view.selected_object = None;
                    self.scene_view.selected_tiles.clear();
                    self.scene_view.selected_objects.clear();
                    history_action = true;
                } else if let Some(index) = self.toolbar.scene_switch_requested {
                    if self.state.select_scene(index) {
                        self.scene_view.selected_object = None;
                        self.scene_view.selected_tiles.clear();
                        self.scene_view.selected_objects.clear();
                        history_action = true;
                    }
                }
            }
            let mode_after = self.state.game_loop.mode;

            // Synchronise les transitions.
            if mode_before != mode_after {
                match mode_after {
                    EngineMode::Playing => {
                        if mode_before == EngineMode::Editor {
                            match self.load_world_script() {
                                Ok(()) => {
                                    self.load_object_scripts();
                                    self.show_world_settings = false;
                                    self.runtime_camera = self.state.active_scene().main_camera;
                                    self.runtime_camera_position = None;
                                    self.runtime_camera_zoom = None;
                                    self.state.play();
                                }
                                Err(error) => {
                                    self.state.game_loop.stop();
                                    self.show_world_settings = true;
                                    self.state.push_log(log::Level::Error,
                                        format!("Lecture impossible : {error}"));
                                }
                            }
                        } else {
                            self.state.resume();
                        }
                    }
                    EngineMode::Paused  => { /* déjà appliqué */ }
                    EngineMode::Editor  => {
                        self.state.stop();
                        self.runtime_camera = None;
                        self.runtime_camera_position = None;
                        self.runtime_camera_zoom = None;
                    }
                }
            }
        });

        if self.toolbar.world_settings_requested {
            self.show_world_settings = true;
        }

        if self.toolbar.editor_settings_requested {
            self.show_editor_settings = true;
        }
        if self.show_editor_settings {
            let mut settings_changed = false;
            egui::Window::new("Paramètres de l'éditeur")
                .open(&mut self.show_editor_settings)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(egui::RichText::new("Affichage du viewport").strong());
                    settings_changed |= ui.checkbox(
                        &mut self.editor_settings.hide_grid_during_play,
                        "Masquer la grille pendant la lecture",
                    ).changed();
                    ui.horizontal(|ui| {
                        ui.label("Couleur de la grille");
                        if ui.color_edit_button_srgba(&mut self.scene_view.options.grid_color).changed() {
                            self.editor_settings.grid_color = self.scene_view.options.grid_color.to_array();
                            settings_changed = true;
                        }
                    });
                    ui.label(egui::RichText::new(
                        "La grille reste visible en mode édition et est masquée pendant Lecture et Pause.",
                    ).small().color(egui::Color32::GRAY));
                });
            if settings_changed {
                if let Err(error) = self.save_editor_settings() {
                    self.state.push_log(log::Level::Error,
                        format!("Enregistrement des paramètres de l'éditeur : {error}"));
                }
            }
        }

        if self.show_world_settings {
            let current_script = self.state.project.world.script.clone();
            let mut choose_script = false;
            let mut create_script = false;
            egui::Window::new("Monde — script Python")
                .open(&mut self.show_world_settings)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!("Monde : {}", self.state.project.world.name));
                    ui.horizontal(|ui| {
                        ui.label("Script lié :");
                        ui.label(current_script.as_deref().unwrap_or("Aucun"));
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Choisir un script…").clicked() { choose_script = true; }
                        if ui.button("Créer un script…").clicked() { create_script = true; }
                    });
                    if let Some(path) = current_script.as_deref() {
                        let root = self.project_path.as_ref().and_then(|p| p.parent())
                            .map(std::path::Path::to_path_buf)
                            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
                        let path = std::path::Path::new(path);
                        let resolved = if path.is_absolute() { path.to_owned() } else { root.join(path) };
                        if !resolved.is_file() {
                            ui.colored_label(egui::Color32::YELLOW, "Fichier introuvable. Choisissez ou créez un script.");
                        }
                    }
                });

            if choose_script {
                match choose_script_file() {
                    Ok(Some(path)) => match self.assign_world_script(&path) {
                        Ok(()) => self.state.push_log(log::Level::Info, "Script du monde configuré".into()),
                        Err(error) => self.state.push_log(log::Level::Error, format!("Script du monde : {error}")),
                    },
                    Ok(None) => {},
                    Err(error) => self.state.push_log(log::Level::Error, format!("Sélecteur de script : {error}")),
                }
            } else if create_script {
                match choose_new_script_file("world.py") {
                    Ok(Some(path)) => match self.create_world_script(&path) {
                        Ok(()) => self.state.push_log(log::Level::Info, "Script du monde créé".into()),
                        Err(error) => self.state.push_log(log::Level::Error, format!("Création du script : {error}")),
                    },
                    Ok(None) => {},
                    Err(error) => self.state.push_log(log::Level::Error, format!("Création du script : {error}")),
                }
            }
        }

        if self.toolbar.new_requested && self.state.game_loop.mode == EngineMode::Editor {
            self.state.project = Project::new("Nouveau projet");
            self.state.selected = None;
            self.state.scene_snapshot = None;
            self.state.undo_history.clear();
            self.state.redo_history.clear();
            self.scene_view.selected_object = None;
            self.scene_view.selected_tiles.clear();
            self.scene_view.selected_objects.clear();
            self.project_path = None;
            history_action = true;
        }
        if self.toolbar.open_requested && self.state.game_loop.mode == EngineMode::Editor {
            match choose_project_file(false) {
                Ok(Some(path)) => match self.load_scene(&path) {
                    Ok(()) => history_action = true,
                    Err(error) => self.state.push_log(log::Level::Error, format!("Ouverture : {error}")),
                },
                Ok(None) => {},
                Err(error) => self.state.push_log(log::Level::Error, format!("Dialogue Ouvrir : {error}")),
            }
        }
        if let Some(path) = self.toolbar.recent_project_requested.clone() {
            if self.state.game_loop.mode == EngineMode::Editor {
                match self.load_scene(&path) {
                    Ok(()) => history_action = true,
                    Err(error) => self.state.push_log(log::Level::Error,
                        format!("Ouverture du projet récent : {error}")),
                }
            }
        }
        if (self.toolbar.save_requested || self.toolbar.save_as_requested || save_shortcut)
            && self.state.game_loop.mode == EngineMode::Editor
        {
            if !self.toolbar.save_as_requested {
                if let Some(path) = self.project_path.clone() {
                    if let Err(error) = self.save_scene(&path) {
                        self.state.push_log(log::Level::Error, format!("Sauvegarde : {error}"));
                    }
                } else {
                    match choose_project_file(true) {
                        Ok(Some(path)) => if let Err(error) = self.save_scene(&path) {
                            self.state.push_log(log::Level::Error, format!("Sauvegarde : {error}"));
                        },
                        Ok(None) => {},
                        Err(error) => self.state.push_log(log::Level::Error, format!("Dialogue Enregistrer : {error}")),
                    }
                }
            } else {
                match choose_project_file(true) {
                    Ok(Some(path)) => if let Err(error) = self.save_scene(&path) {
                        self.state.push_log(log::Level::Error, format!("Sauvegarde : {error}"));
                    },
                    Ok(None) => {},
                    Err(error) => self.state.push_log(log::Level::Error, format!("Dialogue Enregistrer : {error}")),
                }
            }
        }

        // ── Barre de statut en bas ────────────────────────────────────────────
        egui::TopBottomPanel::bottom("statusbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Coordonnées de la tuile survolée.
                if let Some(tile) = self.scene_view.hovered_tile {
                    ui.label(format!("Tuile : ({}, {})", tile.x, tile.y));
                    ui.separator();
                }
                ui.label(format!("Mode : {}", self.state.game_loop.mode));
                ui.separator();
                ui.label(format!(
                    "Objets : {} | Chunks : {}",
                    self.state.active_scene().objects.len(),
                    self.state.active_scene().grid.chunks.len(),
                ));
                ui.separator();
                ui.label(format!("Zoom : {:.1}×", self.scene_view.camera.zoom));
                ui.separator();
                ui.checkbox(&mut self.scene_view.options.show_grid, "Grille");

                // Toggle log.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.toggle_value(&mut self.show_log, "📋 Log");
                });
            });
        });

        // ── Log en bas si activé ──────────────────────────────────────────────
        if self.show_log {
            egui::TopBottomPanel::bottom("log_panel")
                .resizable(true)
                .default_height(120.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Log");
                        if ui.small_button("🗑 Effacer").clicked() {
                            self.state.log_messages.clear();
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .stick_to_bottom(true)
                        .show_rows(ui, 18.0, self.state.log_messages.len(), |ui, rows| {
                            for row in rows {
                                let entry = &self.state.log_messages[row];
                                let color = match entry.level {
                                    log::Level::Error => egui::Color32::from_rgb(255, 100, 100),
                                    log::Level::Warn  => egui::Color32::from_rgb(255, 200, 80),
                                    log::Level::Info  => egui::Color32::WHITE,
                                    _                 => egui::Color32::GRAY,
                                };
                                let text = if entry.repetitions > 1 {
                                    format!("{}  (répété {} fois)", entry.message, entry.repetitions)
                                } else {
                                    entry.message.clone()
                                };
                                ui.label(egui::RichText::new(text).color(color).monospace());
                            }
                        });
                });
        }

        // ── Panneau gauche ────────────────────────────────────────────────────
        let editing_enabled = self.state.game_loop.is_editor();
        let project_root = self.project_path.as_deref()
            .and_then(|path| project_browser_directory(path).ok())
            .and_then(|path| path.canonicalize().ok());
        let active_project_file = self.project_path.as_deref()
            .and_then(|path| path.canonicalize().ok());
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.add_enabled_ui(editing_enabled, |ui| {
                let selection_before_panel = self.state.selected;
                let action = self.left_panel.show(
                    ui,
                    self.state.project.world.active_scene_mut(),
                    &mut self.state.selected,
                    project_root.as_deref(),
                    active_project_file.as_deref(),
                );
                if self.state.selected != selection_before_panel {
                    self.scene_view.selected_object = self.state.selected;
                    self.scene_view.selected_objects.clear();
                    self.scene_view.selected_tiles.clear();
                }
                match action {
                    PanelAction::OpenFile(path) => {
                        if let Err(error) = open_file_with_default_app(&path) {
                            self.state.push_log(log::Level::Error, format!("Ouverture du fichier : {error}"));
                        }
                    }
                    PanelAction::CreateFile(path) => {
                        if let Err(error) = self.create_project_file(&path) {
                            self.state.push_log(log::Level::Error, format!("Création du fichier : {error}"));
                        }
                    }
                    PanelAction::CreateDirectory(path) => {
                        if let Err(error) = self.create_project_directory(&path) {
                            self.state.push_log(log::Level::Error, format!("Création du dossier : {error}"));
                        }
                    }
                    PanelAction::RenamePath(old, new) => {
                        if let Err(error) = self.rename_project_path(&old, &new) {
                            self.state.push_log(log::Level::Error, format!("Renommage : {error}"));
                        }
                    }
                    PanelAction::DeleteObject(id) => {
                        self.pending_deletion = Some((vec![id], Vec::new()));
                    }
                    PanelAction::AddObject => {
                        let id = self.state.active_scene_mut().spawn(
                            "Nouvel objet",
                            world::object::ObjectType::Custom("custom".to_string()),
                            TilePos::new(0, 0),
                        );
                        self.state.selected = Some(id);
                    }
                    PanelAction::AddCamera => {
                        let camera = world::SceneObject::new(
                            "Caméra",
                            world::ObjectType::Camera,
                            TilePos::new(0, 0),
                        );
                        let id = self.state.active_scene_mut().add_object(camera);
                        self.state.selected = Some(id);
                        self.scene_view.selected_object = Some(id);
                    }
                    PanelAction::AddModule(kind) => {
                        let mut obj = world::SceneObject::new(
                            kind.label(),
                            world::ObjectType::Building,
                            TilePos::new(0, 0),
                        );
                        if kind == world::BuildingModuleKind::Roof {
                            obj.layer = world::Layer::Foreground;
                            obj.tile_pos.z = 1;
                        } else {
                            obj.layer = world::Layer::Buildings;
                        }
                        let id = self.state.active_scene_mut().add_object(obj);
                        self.state.selected = Some(id);
                    }
                    PanelAction::InstantiatePrefab(prefab) => {
                        let ids = self.state.active_scene_mut().instantiate_prefab(&prefab, TilePos::new(0, 0));
                        if let Some(&first) = ids.first() {
                            self.state.selected = Some(first);
                        }
                    }
                    _ => {}
                }
                });
            });

        // ── Inspecteur à droite ───────────────────────────────────────────────
        let mut texture_picker_object = None;
        let mut script_picker_object = None;
        let mut script_create_object = None;
        egui::SidePanel::right("inspector_panel")
            .resizable(true)
            .default_width(240.0)
            .width_range(48.0..=f32::INFINITY)
            .show(ctx, |ui| {
                ui.add_enabled_ui(editing_enabled, |ui| {
                  ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                  egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                    let selected = self.state.selected;
                    let mut request_texture_picker = false;
                    let mut request_script_picker = false;
                    let mut request_script_create = false;
                    Inspector::show(
                        ui,
                        self.state.project.world.active_scene_mut(),
                        selected,
                        self.scene_view.selected_objects.len(),
                        &mut request_texture_picker,
                        &mut request_script_picker,
                        &mut request_script_create,
                    );
                    if request_texture_picker {
                        texture_picker_object = selected;
                    }
                    if request_script_picker { script_picker_object = selected; }
                    if request_script_create { script_create_object = selected; }
                  });
                });
            });

        if let Some(object_id) = texture_picker_object {
            match choose_texture_file() {
                Ok(Some(path)) => match self.import_texture(&path) {
                    Ok(project_relative_path) => {
                        if let Some(object) = self.state.active_scene_mut().get_object_mut(object_id) {
                            object.properties.insert(
                                "texture".to_owned(),
                                world::object::PropValue::Str(project_relative_path.clone()),
                            );
                            object.asset_path = Some(project_relative_path);
                        }
                    }
                    Err(error) => self.state.push_log(
                        log::Level::Error,
                        format!("Import de texture : {error}"),
                    ),
                },
                Ok(None) => {}
                Err(error) => self.state.push_log(
                    log::Level::Error,
                    format!("Sélecteur de texture : {error}"),
                ),
            }
        }

        if let Some(object_id) = script_picker_object {
            match choose_script_file() {
                Ok(Some(path)) => match self.import_object_script(&path) {
                    Ok(stored_path) => {
                        if let Some(object) = self.state.active_scene_mut().get_object_mut(object_id) {
                            object.script = Some(stored_path);
                        }
                    }
                    Err(error) => self.state.push_log(log::Level::Error,
                        format!("Script de l'objet : {error}")),
                },
                Ok(None) => {},
                Err(error) => self.state.push_log(log::Level::Error,
                    format!("Sélecteur de script : {error}")),
            }
        }
        if let Some(object_id) = script_create_object {
            match choose_new_script_file("object.py") {
                Ok(Some(path)) => match self.create_object_script(&path) {
                    Ok(stored_path) => {
                        if let Some(object) = self.state.active_scene_mut().get_object_mut(object_id) {
                            object.script = Some(stored_path);
                        }
                    }
                    Err(error) => self.state.push_log(log::Level::Error,
                        format!("Création du script d'objet : {error}")),
                },
                Ok(None) => {},
                Err(error) => self.state.push_log(log::Level::Error,
                    format!("Création du script d'objet : {error}")),
            }
        }

        // ── Vue centrale (scène iso) ──────────────────────────────────────────
        let asset_root = self.project_path.as_ref()
            .and_then(|path| path.parent())
            .map(std::path::Path::to_path_buf)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_default();
        let fps_overlay = (self.state.game_loop.mode == EngineMode::Playing)
            .then_some(self.fps_estimate);
        let grid_visible = self.scene_view.options.show_grid
            && (self.state.game_loop.mode == EngineMode::Editor
                || !self.editor_settings.hide_grid_during_play);
        let editor_camera = self.scene_view.camera.clone();
        if !editing_enabled {
            let camera_pose = self.runtime_camera.and_then(|id| {
                self.state.active_scene().get_object(id)
                    .filter(|object| object.object_type == ObjectType::Camera)
                    .map(|object| (
                        object.tile_pos,
                        object.camera_zoom,
                        object.camera_interpolation,
                        object.camera_interpolation_speed,
                    ))
            });
            if let Some((tile_pos, zoom, interpolation, interpolation_speed)) = camera_pose {
                let position = self.state.active_scene().projection().tile_to_screen(tile_pos);
                let target_position = egui::vec2(position.x, position.y);
                let target_zoom = zoom.clamp(0.1, 10.0);
                let blend = if interpolation {
                    1.0 - (-interpolation_speed.clamp(0.1, 20.0) * delta_time.max(0.0)).exp()
                } else {
                    1.0
                };
                let camera_position = self.runtime_camera_position.get_or_insert(target_position);
                *camera_position += (target_position - *camera_position) * blend;
                let camera_zoom = self.runtime_camera_zoom.get_or_insert(target_zoom);
                *camera_zoom += (target_zoom - *camera_zoom) * blend;
                self.scene_view.camera.position = *camera_position;
                self.scene_view.camera.zoom = *camera_zoom;
            } else {
                self.runtime_camera = None;
                self.runtime_camera_position = None;
                self.runtime_camera_zoom = None;
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            self.scene_view.show(
                ui,
                self.state.project.world.active_scene_mut(),
                self.toolbar.active_tool,
                &asset_root,
                fps_overlay,
                editing_enabled,
                grid_visible,
                &mut self.runtime_camera,
            );
        });
        if self.pending_deletion.is_none() {
            if let Some(request) = self.scene_view.take_delete_request() {
                self.pending_deletion = Some(request);
            }
        } else {
            let _ = self.scene_view.take_delete_request();
        }
        let mut confirm_deletion = false;
        let mut cancel_deletion = false;
        if let Some((object_ids, tiles)) = &self.pending_deletion {
            egui::Window::new("Confirmer la suppression")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                        cancel_deletion = true;
                    }
                    if ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                        confirm_deletion = true;
                    }
                    if object_ids.len() == 1 {
                        let name = self.state.active_scene().get_object(object_ids[0])
                            .map(|object| object.name.as_str()).unwrap_or("cet objet");
                        ui.label(format!("Supprimer l'objet « {name} » de la scène ?"));
                    } else {
                        ui.label(format!("Supprimer ces {} objets de la scène ?", object_ids.len()));
                    }
                    if !tiles.is_empty() {
                        ui.label(format!("{} tuiles sélectionnées seront également supprimées.", tiles.len()));
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Annuler").clicked() { cancel_deletion = true; }
                        if ui.button("Supprimer").clicked() { confirm_deletion = true; }
                    });
                });
        }
        if cancel_deletion {
            self.pending_deletion = None;
        } else if confirm_deletion {
            if let Some((object_ids, tiles)) = self.pending_deletion.take() {
                for id in object_ids {
                    self.state.active_scene_mut().remove_object(id);
                    self.scene_view.selected_objects.remove(&id);
                    if self.state.selected == Some(id) { self.state.selected = None; }
                    if self.scene_view.selected_object == Some(id) { self.scene_view.selected_object = None; }
                }
                for tile in &tiles {
                    self.state.active_scene_mut().clear_tile(*tile);
                    self.scene_view.selected_tiles.remove(tile);
                }
            }
        }
        if !editing_enabled {
            self.scene_view.camera = editor_camera;
        }

        if !history_action && self.state.game_loop.mode == EngineMode::Editor {
            if let (Some(before), Ok(after)) = (scene_before_ui, self.state.active_scene().to_json()) {
                if before != after {
                    self.state.record_undo(before);
                }
            }
        }
    }

    /// Sauvegarde le projet complet (métadonnées, monde et toutes ses scènes).
    pub fn save_scene(&mut self, path: &std::path::Path) -> anyhow::Result<()> {
        let path = with_project_extension(path);
        let path = if path.is_absolute() { path } else { std::env::current_dir()?.join(path) };
        let project_root = path.parent().ok_or_else(|| anyhow::anyhow!("Dossier du projet introuvable"))?;
        std::fs::create_dir_all(project_root)?;

        let source_root = self.project_path.as_ref()
            .and_then(|path| path.parent())
            .map(std::path::Path::to_path_buf)
            .unwrap_or(std::env::current_dir()?);
        let source_root = source_root.canonicalize()?;
        let target_asset_dir = project_asset_directory(&path)?;
        std::fs::create_dir_all(&target_asset_dir)?;
        let target_asset_dir = target_asset_dir.canonicalize()?;
        let target_root = project_root.canonicalize()?;

        let texture_references: Vec<(usize, ObjectId, String)> = self.state.project.world.scenes
            .iter()
            .enumerate()
            .flat_map(|(scene_index, scene)| scene.objects.values().filter_map(move |object| {
                let path = object.asset_path.as_deref().filter(|path| !path.is_empty())
                    .or_else(|| object.properties.get("texture")
                        .and_then(|value| value.as_str()).filter(|path| !path.is_empty()))?;
                Some((scene_index, object.id, path.to_owned()))
            }))
            .collect();

        let asset_prefix = target_asset_dir.strip_prefix(&target_root)?;
        let mut copied_sources = std::collections::HashMap::<std::path::PathBuf, String>::new();
        let mut texture_updates = Vec::new();
        for (scene_index, object_id, stored_path) in texture_references {
            let stored = std::path::Path::new(&stored_path);
            let source_path = if stored.is_absolute() { stored.to_owned() } else { source_root.join(stored) };
            let source = match source_path.canonicalize() {
                Ok(source) => source,
                Err(error) => {
                    self.state.push_log(
                        log::Level::Warn,
                        format!("Texture manquante `{stored_path}` : {error}"),
                    );
                    continue;
                }
            };

            let relative = if let Some(relative) = copied_sources.get(&source) {
                relative.clone()
            } else if let Ok(within_assets) = source.strip_prefix(&target_asset_dir) {
                let relative = asset_prefix.join(within_assets);
                let relative = relative.to_string_lossy().replace('\\', "/");
                copied_sources.insert(source, relative.clone());
                relative
            } else {
                let destination = copy_asset_to_directory(&source, &target_asset_dir)?;
                let relative = asset_prefix.join(destination.file_name().unwrap_or_default());
                let relative = relative.to_string_lossy().replace('\\', "/");
                copied_sources.insert(source, relative.clone());
                relative
            };
            texture_updates.push((scene_index, object_id, relative));
        }

        for (scene_index, object_id, relative) in texture_updates {
            if let Some(object) = self.state.project.world.scenes[scene_index].get_object_mut(object_id) {
                object.asset_path = Some(relative.clone());
                object.properties.insert(
                    "texture".to_owned(),
                    world::object::PropValue::Str(relative),
                );
            }
        }

        let script_references: Vec<(usize, ObjectId, String)> = self.state.project.world.scenes
            .iter().enumerate().flat_map(|(scene_index, scene)| scene.objects.values()
                .filter_map(move |object| object.script.as_ref().filter(|value| !value.trim().is_empty())
                    .map(|value| (scene_index, object.id, value.clone()))))
            .collect();
        let target_scripts_dir = project_script_directory(&path)?;
        std::fs::create_dir_all(&target_scripts_dir)?;
        let target_scripts_dir = target_scripts_dir.canonicalize()?;
        let scripts_prefix = project_script_prefix(&path)?;
        if let Some(stored_path) = self.state.project.world.script.clone()
            .filter(|value| !value.trim().is_empty()) {
            let stored = std::path::Path::new(&stored_path);
            let source_path = if stored.is_absolute() { stored.to_owned() } else { source_root.join(stored) };
            match source_path.canonicalize() {
                Ok(source) => {
                    let relative = if let Ok(within_scripts) = source.strip_prefix(&target_scripts_dir) {
                        scripts_prefix.join(within_scripts)
                    } else {
                        let destination = copy_asset_to_directory(&source, &target_scripts_dir)?;
                        scripts_prefix.join(destination.file_name().unwrap_or_default())
                    };
                    self.state.project.world.script = Some(relative.to_string_lossy().replace('\\', "/"));
                }
                Err(error) => self.state.push_log(log::Level::Warn,
                    format!("Script du monde manquant `{stored_path}` : {error}")),
            }
        }
        let mut script_updates = Vec::new();
        for (scene_index, object_id, stored_path) in script_references {
            let stored = std::path::Path::new(&stored_path);
            let source_path = if stored.is_absolute() { stored.to_owned() } else { source_root.join(stored) };
            let source = match source_path.canonicalize() {
                Ok(source) => source,
                Err(error) => {
                    self.state.push_log(log::Level::Warn,
                        format!("Script Python manquant `{stored_path}` : {error}"));
                    continue;
                }
            };
            let relative = if let Ok(within_scripts) = source.strip_prefix(&target_scripts_dir) {
                scripts_prefix.join(within_scripts)
            } else {
                let destination = copy_asset_to_directory(&source, &target_scripts_dir)?;
                scripts_prefix.join(destination.file_name().unwrap_or_default())
            };
            script_updates.push((scene_index, object_id, relative.to_string_lossy().replace('\\', "/")));
        }
        for (scene_index, object_id, relative) in script_updates {
            if let Some(object) = self.state.project.world.scenes[scene_index].get_object_mut(object_id) {
                object.script = Some(relative);
            }
        }

        let json = self.state.project.to_json()?;
        std::fs::write(&path, &json)?;
        self.project_path = Some(path.clone());
        self.remember_recent_project(&path);
        self.state.push_log(log::Level::Info, format!("Projet sauvegardé : {}", path.display()));
        Ok(())
    }

    fn save_editor_settings(&self) -> anyhow::Result<()> {
        let path = editor_settings_path()?;
        let parent = path.parent().ok_or_else(|| anyhow::anyhow!("Dossier de configuration introuvable"))?;
        std::fs::create_dir_all(parent)?;
        std::fs::write(path, serde_json::to_vec_pretty(&self.editor_settings)?)?;
        Ok(())
    }

    fn create_project_file(&mut self, path: &std::path::Path) -> anyhow::Result<()> {
        validate_project_entry_name(path.file_name().and_then(|name| name.to_str()))?;
        let path = resolve_project_browser_path(self.project_path.as_deref(), path)?;
        let current = self.left_panel.current_browser_directory()
            .ok_or_else(|| anyhow::anyhow!("Dossier courant introuvable"))?.canonicalize()?;
        anyhow::ensure!(path.parent() == Some(current.as_path()), "Le fichier doit être créé dans le dossier affiché");
        anyhow::ensure!(!path.exists(), "Un élément portant ce nom existe déjà");
        std::fs::OpenOptions::new().write(true).create_new(true).open(&path)?;
        self.left_panel.refresh_file_browser();
        Ok(())
    }

    fn create_project_directory(&mut self, path: &std::path::Path) -> anyhow::Result<()> {
        validate_project_entry_name(path.file_name().and_then(|name| name.to_str()))?;
        let path = resolve_project_browser_path(self.project_path.as_deref(), path)?;
        let current = self.left_panel.current_browser_directory()
            .ok_or_else(|| anyhow::anyhow!("Dossier courant introuvable"))?.canonicalize()?;
        anyhow::ensure!(path.parent() == Some(current.as_path()), "Le dossier doit être créé dans le dossier affiché");
        anyhow::ensure!(!path.exists(), "Un élément portant ce nom existe déjà");
        std::fs::create_dir(&path)?;
        self.left_panel.refresh_file_browser();
        Ok(())
    }

    fn rename_project_path(&mut self, old: &std::path::Path, new: &std::path::Path) -> anyhow::Result<()> {
        validate_project_entry_name(new.file_name().and_then(|name| name.to_str()))?;
        let root = self.project_path.as_deref()
            .and_then(std::path::Path::parent)
            .ok_or_else(|| anyhow::anyhow!("Aucun projet n'est ouvert"))?
            .canonicalize()?;
        let old = resolve_project_browser_path(self.project_path.as_deref(), old)?;
        let new = resolve_project_browser_path(self.project_path.as_deref(), new)?;
        anyhow::ensure!(old != root, "Impossible de renommer la racine du projet");
        anyhow::ensure!(new.parent() == old.parent(), "Le renommage doit rester dans le dossier d'origine");
        anyhow::ensure!(!new.exists(), "Un élément portant ce nom existe déjà");

        let mut texture_updates = Vec::new();
        let mut script_updates = Vec::new();
        for (scene_index, scene) in self.state.project.world.scenes.iter().enumerate() {
            for object in scene.objects.values() {
                let asset_path = object.asset_path.as_deref()
                    .and_then(|path| remap_project_reference(path, &root, &old, &new));
                let texture_property = object.properties.get("texture")
                    .and_then(world::object::PropValue::as_str)
                    .and_then(|path| remap_project_reference(path, &root, &old, &new));
                let script = object.script.as_deref()
                    .and_then(|path| remap_project_reference(path, &root, &old, &new));
                if asset_path.is_some() || texture_property.is_some() {
                    texture_updates.push((scene_index, object.id, asset_path, texture_property));
                }
                if let Some(script) = script {
                    script_updates.push((scene_index, object.id, script));
                }
            }
        }
        let world_script = self.state.project.world.script.as_deref()
            .and_then(|path| remap_project_reference(path, &root, &old, &new));

        std::fs::rename(&old, &new)?;
        for (scene_index, object_id, asset_path, texture) in texture_updates {
            if let Some(object) = self.state.project.world.scenes[scene_index].get_object_mut(object_id) {
                if let Some(path) = asset_path { object.asset_path = Some(path); }
                if let Some(path) = texture {
                    object.properties.insert("texture".to_owned(), world::object::PropValue::Str(path));
                }
            }
        }
        for (scene_index, object_id, script) in script_updates {
            if let Some(object) = self.state.project.world.scenes[scene_index].get_object_mut(object_id) {
                object.script = Some(script);
            }
        }
        if let Some(script) = world_script {
            self.state.project.world.script = Some(script);
        }
        self.left_panel.refresh_file_browser();
        if let Some(project_file) = self.project_path.clone() {
            let json = self.state.project.to_json()?;
            std::fs::write(&project_file, json)?;
            self.state.push_log(log::Level::Info,
                format!("Références mises à jour après renommage : {}", project_file.display()));
        }
        Ok(())
    }

    fn import_texture(&self, selected: &std::path::Path) -> anyhow::Result<String> {
        let extension = selected.extension().and_then(|value| value.to_str())
            .unwrap_or_default().to_ascii_lowercase();
        if !matches!(extension.as_str(), "png" | "jpg" | "jpeg") {
            anyhow::bail!("Formats acceptés : PNG et JPEG");
        }
        let source = selected.canonicalize()?;
        assets::loader::load_image(&source)?;
        let Some(project_file) = &self.project_path else {
            // Keep the source until the first Save As chooses the isolated
            // project directory; save_scene then copies it into that project.
            return Ok(source.to_string_lossy().into_owned());
        };

        let asset_dir = project_asset_directory(project_file)?;
        std::fs::create_dir_all(&asset_dir)?;
        let asset_dir = asset_dir.canonicalize()?;
        if let Ok(within_assets) = source.strip_prefix(&asset_dir) {
            let relative = project_asset_prefix(project_file)?.join(within_assets);
            return Ok(relative.to_string_lossy().replace('\\', "/"));
        }

        let destination = copy_asset_to_directory(&source, &asset_dir)?;
        let relative = project_asset_prefix(project_file)?.join(
            destination.file_name().ok_or_else(|| anyhow::anyhow!("Nom de fichier invalide"))?,
        );
        Ok(relative.to_string_lossy().replace('\\', "/"))
    }

    fn import_script(&self, selected: &std::path::Path) -> anyhow::Result<String> {
        if !selected.extension().and_then(|value| value.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("py")) {
            anyhow::bail!("Le fichier doit avoir l'extension .py");
        }
        let source = selected.canonicalize()?;
        std::fs::read_to_string(&source)?;
        let Some(project_file) = &self.project_path else {
            return Ok(source.to_string_lossy().into_owned());
        };
        let scripts_dir = project_script_directory(project_file)?;
        std::fs::create_dir_all(&scripts_dir)?;
        let scripts_dir = scripts_dir.canonicalize()?;
        let relative = if let Ok(within_scripts) = source.strip_prefix(&scripts_dir) {
            project_script_prefix(project_file)?.join(within_scripts)
        } else {
            let destination = copy_asset_to_directory(&source, &scripts_dir)?;
            project_script_prefix(project_file)?.join(destination.file_name().unwrap_or_default())
        };
        Ok(relative.to_string_lossy().replace('\\', "/"))
    }

    fn import_object_script(&self, selected: &std::path::Path) -> anyhow::Result<String> {
        ensure_script_file(selected, OBJECT_SCRIPT_TEMPLATE)?;
        self.import_script(selected)
    }

    fn create_object_script(&self, selected: &std::path::Path) -> anyhow::Result<String> {
        validate_python_script_path(selected)?;
        if let Some(parent) = selected.parent() { std::fs::create_dir_all(parent)?; }
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(selected)
            .map_err(|error| anyhow::anyhow!("Impossible de créer le fichier (il existe peut-être déjà) : {error}"))?;
        use std::io::Write;
        file.write_all(OBJECT_SCRIPT_TEMPLATE.as_bytes())?;
        self.import_script(selected)
    }

    fn assign_world_script(&mut self, selected: &std::path::Path) -> anyhow::Result<()> {
        let source = selected.canonicalize()?;
        if !source.extension().and_then(|value| value.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("py")) {
            anyhow::bail!("Le fichier doit avoir l'extension .py");
        }
        ensure_world_script_entrypoints(&source)?;
        self.state.project.world.script = Some(self.import_script(&source)?);
        Ok(())
    }

    fn create_world_script(&mut self, selected: &std::path::Path) -> anyhow::Result<()> {
        if !selected.extension().and_then(|value| value.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("py")) {
            anyhow::bail!("Le fichier doit avoir l'extension .py");
        }
        if let Some(parent) = selected.parent() { std::fs::create_dir_all(parent)?; }
        let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(selected)
            .map_err(|error| anyhow::anyhow!("Impossible de créer le fichier (il existe peut-être déjà) : {error}"))?;
        use std::io::Write;
        file.write_all(WORLD_SCRIPT_TEMPLATE.as_bytes())?;
        self.state.project.world.script = Some(self.import_script(selected)?);
        Ok(())
    }

    /// Charge un projet complet, ou importe un ancien fichier de scène seul.
    pub fn load_scene(&mut self, path: &std::path::Path) -> anyhow::Result<()> {
        let json = std::fs::read_to_string(path)?;
        let project = match Project::from_json(&json) {
            Ok(project) => project,
            Err(project_error) => match Scene::from_json(&json) {
                Ok(scene) => {
                    let mut project = Project::new(format!("{} (importé)", scene.meta.name));
                    project.world.scenes = vec![scene];
                    project.world.active_scene_index = 0;
                    project
                }
                Err(scene_error) => anyhow::bail!(
                    "Fichier ni projet ni scène JSON valide (projet: {project_error}; scène: {scene_error})"
                ),
            },
        };
        self.state.project = project;
        self.state.selected = None;
        self.scene_view.selected_object = None;
        self.scene_view.selected_tiles.clear();
        self.scene_view.selected_objects.clear();
        self.state.undo_history.clear();
        self.state.redo_history.clear();
        self.project_path = Some(path.to_owned());
        self.remember_recent_project(path);
        self.state.push_log(log::Level::Info, format!("Projet chargé : {}", path.display()));
        Ok(())
    }

    fn remember_recent_project(&mut self, path: &std::path::Path) {
        let path = if path.is_absolute() {
            path.to_owned()
        } else {
            match std::env::current_dir() {
                Ok(directory) => directory.join(path),
                Err(_) => path.to_owned(),
            }
        };
        let path = path.to_string_lossy().into_owned();
        self.editor_settings.recent_projects.retain(|recent| recent != &path);
        self.editor_settings.recent_projects.insert(0, path);
        self.editor_settings.recent_projects.truncate(10);
        if let Err(error) = self.save_editor_settings() {
            self.state.push_log(log::Level::Warn,
                format!("Enregistrement des projets récents : {error}"));
        }
    }

    /// Charge un script Python dans le runtime.
    pub fn load_script(&mut self, path: &std::path::Path) -> anyhow::Result<()> {
        self.state.script_runtime.load_script(path)
            .map_err(|e| anyhow::anyhow!("{e}"))
    }

    fn load_world_script(&mut self) -> anyhow::Result<()> {
        let Some(script_path) = self.state.project.world.script.clone()
            .filter(|path| !path.trim().is_empty()) else {
                anyhow::bail!("Aucun script n'est défini pour ce monde")
            };
        let root = self.project_path.as_ref()
            .and_then(|path| path.parent())
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let path = std::path::Path::new(&script_path);
        let resolved = if path.is_absolute() { path.to_owned() } else { root.join(path) };
        self.state.script_runtime.load_world_script(&resolved)
            .map_err(|error| anyhow::anyhow!("Chargement de `{script_path}` : {error}"))
    }

    fn load_object_scripts(&mut self) {
        let root = self.project_path.as_ref()
            .and_then(|path| path.parent())
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let scripts: Vec<(String, String)> = self.state.active_scene().objects.values()
            .filter_map(|object| object.script.as_ref().filter(|path| !path.trim().is_empty())
                .map(|path| (object.id.to_string(), path.clone())))
            .collect();
        for (object_id, script_path) in scripts {
            let path = std::path::Path::new(&script_path);
            let resolved = if path.is_absolute() { path.to_owned() } else { root.join(path) };
            log::info!("Chargement du script objet {} : {}", object_id, resolved.display());
            if let Err(error) = self.state.script_runtime.load_object_script(&resolved, object_id) {
                self.state.push_log(log::Level::Error,
                    format!("Chargement du script d'objet `{script_path}` : {error}"));
            }
        }
    }
}

fn with_project_extension(path: &std::path::Path) -> std::path::PathBuf {
    let has_extension = path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("isomaker"));
    if has_extension {
        path.to_owned()
    } else {
        path.with_extension("isomaker")
    }
}

const WORLD_SCRIPT_TEMPLATE: &str = "from __future__ import annotations\nfrom typing import TYPE_CHECKING\n\nif TYPE_CHECKING:\n    from engine_api import GameContext\n\n\ndef _on_game_launch(game: GameContext) -> None:\n    \"\"\"Called once when the game starts.\"\"\"\n    pass\n\n\ndef _on_update(game: GameContext, delta_time: float) -> None:\n    \"\"\"Called on each game update.\"\"\"\n    pass\n";
const OBJECT_SCRIPT_TEMPLATE: &str = "from __future__ import annotations\nfrom typing import TYPE_CHECKING\n\nif TYPE_CHECKING:\n    from engine_api import GameContext\n\n\ndef _on_game_launch(game: GameContext) -> None:\n    \"\"\"Called once when the game starts.\"\"\"\n    # Object id: game.object_id; tile position: game.x, game.y, game.z\n    pass\n\n\ndef _on_update(game: GameContext, delta_time: float) -> None:\n    \"\"\"Called on each game update.\"\"\"\n    # Use game.move_self(x, y), game.destroy_self(), or game.set_self_property(...).\n    pass\n";

fn validate_python_script_path(path: &std::path::Path) -> anyhow::Result<()> {
    if !path.extension().and_then(|value| value.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("py")) {
        anyhow::bail!("Le fichier doit avoir l'extension .py");
    }
    Ok(())
}

fn ensure_script_file(path: &std::path::Path, template: &str) -> anyhow::Result<()> {
    validate_python_script_path(path)?;
    if std::fs::read_to_string(path)?.trim().is_empty() {
        std::fs::write(path, template)?;
    }
    Ok(())
}

fn ensure_world_script_entrypoints(path: &std::path::Path) -> anyhow::Result<()> {
    let source = std::fs::read_to_string(path)?;
    if source.trim().is_empty() {
        std::fs::write(path, WORLD_SCRIPT_TEMPLATE)?;
    }
    Ok(())
}

fn editor_settings_path() -> anyhow::Result<std::path::PathBuf> {
    #[cfg(target_os = "windows")]
    let config_root = std::env::var_os("APPDATA")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("USERPROFILE").map(|path| {
            std::path::PathBuf::from(path).join("AppData").join("Roaming")
        }));
    #[cfg(not(target_os = "windows"))]
    let config_root = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|path| {
            std::path::PathBuf::from(path).join(".config")
        }));

    let config_root = config_root.ok_or_else(|| anyhow::anyhow!("Dossier de configuration utilisateur introuvable"))?;
    Ok(config_root.join("MoteurDeJeu").join("editor_settings.json"))
}

fn load_editor_settings() -> EditorSettings {
    let path = match editor_settings_path() {
        Ok(path) => path,
        Err(error) => {
            log::warn!("Chargement des paramètres de l'éditeur : {error}");
            return EditorSettings::default();
        }
    };
    match std::fs::read_to_string(path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|error| {
            log::warn!("Fichier de paramètres invalide, valeurs par défaut utilisées : {error}");
            EditorSettings::default()
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => EditorSettings::default(),
        Err(error) => {
            log::warn!("Lecture des paramètres de l'éditeur impossible : {error}");
            EditorSettings::default()
        }
    }
}

fn validate_project_entry_name(name: Option<&str>) -> anyhow::Result<()> {
    let name = name.ok_or_else(|| anyhow::anyhow!("Nom de fichier invalide"))?;
    anyhow::ensure!(
        !name.is_empty()
            && name != "."
            && name != ".."
            && !name.chars().any(|character| matches!(character, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
            && !name.ends_with(' ')
            && !name.ends_with('.'),
        "Nom invalide ou séparateurs interdits"
    );
    Ok(())
}

fn resolve_project_browser_path(
    project_file: Option<&std::path::Path>,
    requested: &std::path::Path,
) -> anyhow::Result<std::path::PathBuf> {
    let root = project_browser_directory(
        project_file.ok_or_else(|| anyhow::anyhow!("Aucun projet n'est ouvert"))?,
    )?.canonicalize()?;
    let requested = if requested.is_absolute() { requested.to_path_buf() } else { root.join(requested) };
    let resolved = if requested.exists() {
        requested.canonicalize()?
    } else {
        let parent = requested.parent().ok_or_else(|| anyhow::anyhow!("Chemin de destination invalide"))?
            .canonicalize()?;
        parent.join(requested.file_name().ok_or_else(|| anyhow::anyhow!("Nom de fichier invalide"))?)
    };
    anyhow::ensure!(resolved.starts_with(&root), "Le gestionnaire ne peut agir qu'à l'intérieur du dossier du projet");
    Ok(resolved)
}

fn remap_project_reference(
    stored: &str,
    root: &std::path::Path,
    old: &std::path::Path,
    new: &std::path::Path,
) -> Option<String> {
    let stored_path = std::path::Path::new(stored);
    let was_absolute = stored_path.is_absolute();
    let resolved = if was_absolute { stored_path.to_path_buf() } else { root.join(stored_path) }
        .canonicalize().ok()?;
    let suffix = resolved.strip_prefix(old).ok()?;
    let remapped = new.join(suffix);
    let stored_remapped = if was_absolute { remapped } else { remapped.strip_prefix(root).ok()?.to_path_buf() };
    Some(stored_remapped.to_string_lossy().replace('\\', "/"))
}

#[cfg(target_os = "windows")]
fn open_file_with_default_app(path: &std::path::Path) -> anyhow::Result<()> {
    use std::{ffi::c_void, os::windows::ffi::OsStrExt};
    let operation: Vec<u16> = "open\0".encode_utf16().collect();
    let file: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    #[link(name = "shell32")]
    unsafe extern "system" {
        fn ShellExecuteW(
            window: *mut c_void,
            operation: *const u16,
            file: *const u16,
            parameters: *const u16,
            directory: *const u16,
            show_command: i32,
        ) -> isize;
    }
    let result = unsafe {
        ShellExecuteW(std::ptr::null_mut(), operation.as_ptr(), file.as_ptr(),
            std::ptr::null(), std::ptr::null(), 1)
    };
    anyhow::ensure!(result > 32, "Windows n'a pas pu ouvrir {}", path.display());
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn open_file_with_default_app(path: &std::path::Path) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(not(target_os = "macos"))]
    let mut command = std::process::Command::new("xdg-open");
    command.arg(path).spawn()?;
    Ok(())
}

fn project_asset_directory(project_file: &std::path::Path) -> anyhow::Result<std::path::PathBuf> {
    let parent = project_file.parent()
        .ok_or_else(|| anyhow::anyhow!("Dossier du projet introuvable"))?;
    let stem = project_file.file_stem()
        .ok_or_else(|| anyhow::anyhow!("Nom de projet invalide"))?;
    Ok(parent.join(stem).join("assets"))
}

fn project_browser_directory(project_file: &std::path::Path) -> anyhow::Result<std::path::PathBuf> {
    let parent = project_file.parent()
        .ok_or_else(|| anyhow::anyhow!("Dossier du projet introuvable"))?;
    let stem = project_file.file_stem()
        .ok_or_else(|| anyhow::anyhow!("Nom de projet invalide"))?;
    Ok(parent.join(stem))
}

fn project_asset_prefix(project_file: &std::path::Path) -> anyhow::Result<std::path::PathBuf> {
    let stem = project_file.file_stem()
        .ok_or_else(|| anyhow::anyhow!("Nom de projet invalide"))?;
    Ok(std::path::PathBuf::from(stem).join("assets"))
}

fn project_script_directory(project_file: &std::path::Path) -> anyhow::Result<std::path::PathBuf> {
    let parent = project_file.parent()
        .ok_or_else(|| anyhow::anyhow!("Dossier du projet introuvable"))?;
    let stem = project_file.file_stem()
        .ok_or_else(|| anyhow::anyhow!("Nom de projet invalide"))?;
    Ok(parent.join(stem).join("scripts"))
}

fn project_script_prefix(project_file: &std::path::Path) -> anyhow::Result<std::path::PathBuf> {
    let stem = project_file.file_stem()
        .ok_or_else(|| anyhow::anyhow!("Nom de projet invalide"))?;
    Ok(std::path::PathBuf::from(stem).join("scripts"))
}

fn copy_asset_to_directory(
    source: &std::path::Path,
    destination_dir: &std::path::Path,
) -> anyhow::Result<std::path::PathBuf> {
    std::fs::create_dir_all(destination_dir)?;
    let file_name = source.file_name()
        .ok_or_else(|| anyhow::anyhow!("Nom de fichier invalide"))?;
    let mut destination = destination_dir.join(file_name);
    if destination.exists() {
        let stem = source.file_stem().and_then(|value| value.to_str()).unwrap_or("asset");
        let extension = source.extension().and_then(|value| value.to_str());
        let mut index = 1u32;
        loop {
            let candidate_name = match extension {
                Some(extension) => format!("{stem}_{index}.{extension}"),
                None => format!("{stem}_{index}"),
            };
            destination = destination_dir.join(candidate_name);
            if !destination.exists() { break; }
            index += 1;
        }
    }
    std::fs::copy(source, &destination)?;
    Ok(destination)
}

#[cfg(target_os = "windows")]
fn choose_texture_file() -> anyhow::Result<Option<std::path::PathBuf>> {
    let script = r#"
Add-Type -AssemblyName System.Windows.Forms
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$dialog = New-Object System.Windows.Forms.OpenFileDialog
$dialog.Title = 'Choisir une texture'
$dialog.Filter = 'Images PNG ou JPEG (*.png;*.jpg;*.jpeg)|*.png;*.jpg;*.jpeg'
$dialog.Multiselect = $false
$dialog.CheckFileExists = $true
$dialog.RestoreDirectory = $true
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
    [Console]::Out.Write($dialog.FileName)
}
"#;

    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-STA", "-WindowStyle", "Hidden", "-Command", &script])
        .output()?;
    if !output.status.success() {
        anyhow::bail!("Le sélecteur de texture Windows a échoué : {}",
            String::from_utf8_lossy(&output.stderr).trim());
    }
    let selected = String::from_utf8(output.stdout)?;
    let selected = selected.trim();
    if selected.is_empty() {
        Ok(None)
    } else {
        Ok(Some(std::path::PathBuf::from(selected)))
    }
}

#[cfg(not(target_os = "windows"))]
fn choose_texture_file() -> anyhow::Result<Option<std::path::PathBuf>> {
    anyhow::bail!("Le sélecteur natif de textures est uniquement disponible sous Windows")
}

#[cfg(target_os = "windows")]
fn choose_script_file() -> anyhow::Result<Option<std::path::PathBuf>> {
    let script = r#"
Add-Type -AssemblyName System.Windows.Forms
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$dialog = New-Object System.Windows.Forms.OpenFileDialog
$dialog.Title = 'Choisir un script Python'
$dialog.Filter = 'Scripts Python (*.py)|*.py'
$dialog.Multiselect = $false
$dialog.CheckFileExists = $true
$dialog.RestoreDirectory = $true
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
    [Console]::Out.Write($dialog.FileName)
}
"#;
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-STA", "-WindowStyle", "Hidden", "-Command", &script])
        .output()?;
    if !output.status.success() {
        anyhow::bail!("Le sélecteur de script Windows a échoué : {}",
            String::from_utf8_lossy(&output.stderr).trim());
    }
    let selected = String::from_utf8(output.stdout)?;
    let selected = selected.trim();
    if selected.is_empty() { Ok(None) } else { Ok(Some(std::path::PathBuf::from(selected))) }
}

#[cfg(not(target_os = "windows"))]
fn choose_script_file() -> anyhow::Result<Option<std::path::PathBuf>> {
    anyhow::bail!("Le sélecteur de scripts est uniquement disponible sous Windows")
}

#[cfg(target_os = "windows")]
fn choose_new_script_file(default_name: &str) -> anyhow::Result<Option<std::path::PathBuf>> {
    let script = r#"
Add-Type -AssemblyName System.Windows.Forms
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$dialog = New-Object System.Windows.Forms.SaveFileDialog
$dialog.Title = 'Créer un script Python'
$dialog.Filter = 'Scripts Python (*.py)|*.py'
$dialog.DefaultExt = 'py'
$dialog.AddExtension = $true
$dialog.OverwritePrompt = $false
$dialog.FileName = '{default_name}'
$dialog.RestoreDirectory = $true
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
    [Console]::Out.Write($dialog.FileName)
}
"#.replace("{default_name}", default_name);
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-STA", "-WindowStyle", "Hidden", "-Command", &script])
        .output()?;
    if !output.status.success() {
        anyhow::bail!("Le dialogue de création Windows a échoué : {}",
            String::from_utf8_lossy(&output.stderr).trim());
    }
    let selected = String::from_utf8(output.stdout)?;
    let selected = selected.trim();
    if selected.is_empty() { Ok(None) } else { Ok(Some(std::path::PathBuf::from(selected))) }
}

#[cfg(not(target_os = "windows"))]
fn choose_new_script_file(_default_name: &str) -> anyhow::Result<Option<std::path::PathBuf>> {
    anyhow::bail!("Le sélecteur de scripts est uniquement disponible sous Windows")
}

#[cfg(target_os = "windows")]
fn choose_project_file(save: bool) -> anyhow::Result<Option<std::path::PathBuf>> {
    let script = if save {
        r#"
Add-Type -AssemblyName System.Windows.Forms
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$dialog = New-Object System.Windows.Forms.SaveFileDialog
$dialog.Title = 'Enregistrer le projet IsoMaker'
$dialog.Filter = 'Projet IsoMaker (*.isomaker)|*.isomaker'
$dialog.DefaultExt = 'isomaker'
$dialog.AddExtension = $true
$dialog.OverwritePrompt = $true
$dialog.FileName = 'project.isomaker'
$dialog.RestoreDirectory = $true
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
    [Console]::Out.Write($dialog.FileName)
}
"#
    } else {
        r#"
Add-Type -AssemblyName System.Windows.Forms
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$dialog = New-Object System.Windows.Forms.OpenFileDialog
$dialog.Title = 'Ouvrir un projet IsoMaker'
$dialog.Filter = 'Projet IsoMaker (*.isomaker)|*.isomaker|Ancienne scène JSON (*.json)|*.json'
$dialog.Multiselect = $false
$dialog.CheckFileExists = $true
$dialog.RestoreDirectory = $true
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
    [Console]::Out.Write($dialog.FileName)
}
"#
    };

    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-STA", "-WindowStyle", "Hidden", "-Command", script])
        .output()?;
    if !output.status.success() {
        anyhow::bail!("Le sélecteur de fichiers Windows a échoué : {}",
            String::from_utf8_lossy(&output.stderr).trim());
    }
    let selected = String::from_utf8(output.stdout)?;
    let selected = selected.trim();
    if selected.is_empty() {
        Ok(None)
    } else {
        Ok(Some(std::path::PathBuf::from(selected)))
    }
}

#[cfg(not(target_os = "windows"))]
fn choose_project_file(_save: bool) -> anyhow::Result<Option<std::path::PathBuf>> {
    anyhow::bail!("Le sélecteur natif de fichiers est uniquement disponible sous Windows")
}

impl Default for EditorApp {
    fn default() -> Self { Self::new() }
}
