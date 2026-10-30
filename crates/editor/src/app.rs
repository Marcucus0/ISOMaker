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
use std::collections::{HashSet, VecDeque};
use std::sync::mpsc::{Receiver, TryRecvError};
use world::{
    object::{CustomObjectTypeDefinition, CustomPropertyKind, ObjectId, ObjectType, PropValue, SceneObject},
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
    /// Pile utilisée par game.load_scene()/game.unload_scene().
    scene_stack: Vec<usize>,
    /// Paires zone/objet qui se chevauchaient lors de la dernière frame.
    area_overlaps: HashSet<(ObjectId, ObjectId)>,
    /// Messages de log accumulés.
    pub log_messages: VecDeque<LogEntry>,
    /// Historique des états de scène pour annuler/rétablir les éditions.
    undo_history: Vec<String>,
    redo_history: Vec<String>,
    next_log_id: u64,
}

#[derive(Clone)]
pub struct LogEntry {
    pub level: log::Level,
    pub message: String,
    pub repetitions: usize,
    pub id: u64,
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
            scene_stack: Vec::new(),
            area_overlaps: HashSet::new(),
            log_messages: VecDeque::new(),
            undo_history: Vec::new(),
            redo_history: Vec::new(),
            next_log_id: 1,
        }
    }

    pub fn add_scene(&mut self) -> usize {
        let mut number = 1;
        while self.project.world.scenes.iter().any(|scene| scene.meta.name == format!("Scène {number}")) {
            number += 1;
        }
        self.project.world.add_scene(format!("Scène {number}"))
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
        self.scene_stack.clear();
        self.selected = None;
        self.scene_snapshot = None;
        self.undo_history.clear();
        self.redo_history.clear();
        true
    }

    fn load_scene_by_name(&mut self, name: &str) -> Result<bool, String> {
        let Some(index) = self.project.world.scenes.iter().position(|scene| scene.meta.name == name) else {
            return Err(format!("Aucune scène ne porte le nom « {name} »."));
        };
        let current = self.project.world.active_scene_index;
        if current == index { return Ok(false); }
        self.scene_stack.push(current);
        self.project.world.active_scene_index = index;
        self.selected = None;
        self.area_overlaps.clear();
        Ok(true)
    }

    fn unload_active_scene(&mut self) -> Result<bool, String> {
        let Some(previous) = self.scene_stack.pop() else {
            return Err("Aucune scène précédente à décharger.".to_owned());
        };
        self.project.world.active_scene_index = previous;
        self.selected = None;
        self.area_overlaps.clear();
        Ok(true)
    }

    /// Démarre la simulation.
    pub fn play(&mut self) {
        if self.game_loop.mode == EngineMode::Editor {
            self.game_loop.play();
        }
        // Le toolbar applique déjà la transition de mode. L'absence de snapshot
        // distingue un nouveau Play d'une reprise après pause.
        if self.game_loop.mode == EngineMode::Playing && self.scene_snapshot.is_none() {
            self.area_overlaps.clear();
            // Conserve aussi la scène sélectionnée dans l'éditeur pour Stop.
            self.scene_snapshot = self.project.to_json().ok();
            if self.project.world.main_scene_index < self.project.world.scenes.len() {
                self.project.world.active_scene_index = self.project.world.main_scene_index;
            }
            self.scene_stack.clear();
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
        self.area_overlaps.clear();
        self.scene_stack.clear();
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
            match Project::from_json(&snapshot) {
                Ok(restored) => self.project = restored,
                Err(e) => self.push_log(log::Level::Error, format!("Restauration : {e}")),
            }
        }
        self.selected = None;
    }

    /// Appelé chaque frame pendant Playing.
    pub fn update(&mut self, delta_time: f32) {
        if !self.game_loop.is_simulating() { return; }
        if self.script_runtime.has_legacy_update() {
            match self.script_runtime.call_update(delta_time) {
                Ok(cmds) => self.apply_commands(cmds),
                Err(e) => self.push_log(log::Level::Error, format!("on_update : {e}")),
            }
        }

        let has_world_update = self.script_runtime.has_world_update();
        let has_object_update = self.script_runtime.has_object_update();
        let mut objects = self.active_scene().objects.values().cloned().collect::<Vec<_>>();
        let mut world_changed_scene = false;
        if has_world_update {
            match self.script_runtime.call_world_update(delta_time, &self.input, &objects) {
                Ok(cmds) => {
                    world_changed_scene = !cmds.is_empty();
                    self.apply_commands(cmds);
                }
                Err(e) => self.push_log(log::Level::Error, format!("_on_update : {e}")),
            }
        }
        if has_object_update {
            // World commands are applied before object hooks, so refresh only
            // when they changed the scene. Otherwise reuse this frame snapshot.
            if world_changed_scene {
                objects = self.active_scene().objects.values().cloned().collect();
            }
            match self.script_runtime.call_object_update(delta_time, &objects, &self.input) {
                Ok(cmds) => self.apply_commands(cmds),
                Err(e) => self.push_log(log::Level::Error, format!("Script objet _on_update : {e}")),
            }
        }
        self.update_area_triggers(delta_time);
    }

    fn update_area_triggers(&mut self, delta_time: f32) {
        let objects = self.active_scene().objects.values().cloned().collect::<Vec<_>>();
        let areas = objects.iter()
            .filter(|object| self.script_runtime.object_has_area_hooks(&object.id.to_string()))
            .collect::<Vec<_>>();
        let mut overlaps = HashSet::new();
        for area in &areas {
            for other in &objects {
                if area.id == other.id || !object_footprints_overlap(area, other) { continue; }
                overlaps.insert((area.id, other.id));
            }
        }

        let mut events = Vec::new();
        events.extend(overlaps.difference(&self.area_overlaps).map(|pair| ("_on_area_entered", *pair)));
        events.extend(overlaps.intersection(&self.area_overlaps).map(|pair| ("_on_area_stayed", *pair)));
        events.extend(self.area_overlaps.difference(&overlaps).map(|pair| ("_on_area_exited", *pair)));
        events.sort_by_key(|(hook, (area, other))| (*hook, area.to_string(), other.to_string()));
        self.area_overlaps = overlaps;

        for (hook, (area_id, other_id)) in events {
            match self.script_runtime.call_object_area_event(
                hook, &area_id.to_string(), &other_id.to_string(), delta_time, &objects, &self.input,
            ) {
                Ok(commands) => self.apply_commands(commands),
                Err(error) => self.push_log(log::Level::Error, format!("{hook} : {error}")),
            }
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
                        let id = world::object::ObjectId(uuid);
                        let name = self.active_scene().get_object(id).map(|object| object.name.clone());
                        if self.active_scene_mut().remove_object(id).is_some() {
                            if let Some(name) = name {
                                self.push_log(log::Level::Info, format!("Objet « {name} » supprimé."));
                            }
                        }
                    }
                }
                GameCommand::Move { object_id, x, y } => {
                    if let Ok(uuid) = uuid::Uuid::parse_str(&object_id) {
                        let id = world::object::ObjectId(uuid);
                        let destination = self.active_scene().get_object(id)
                            .map(|object| TilePos::with_z(x, y, object.tile_pos.z));
                        if let Some(destination) = destination {
                            let result = move_object_with_collisions(
                                self.project.world.active_scene_mut(), &self.project.tile_definitions, id, destination,
                            );
                            let _ = result;
                        }
                    }
                }
                GameCommand::SetProperty { object_id, key, value } => {
                    if let Ok(uuid) = uuid::Uuid::parse_str(&object_id) {
                        let id = world::object::ObjectId(uuid);
                        let is_position = matches!(key.as_str(), "x" | "y" | "z");
                        let result = if is_position {
                            let coordinate = value.as_i64().and_then(|v| i32::try_from(v).ok())
                                .ok_or_else(|| "attend un entier".to_owned());
                            match (self.active_scene().get_object(id), coordinate) {
                                (Some(object), Ok(coordinate)) => {
                                    let mut destination = object.tile_pos;
                                    match key.as_str() {
                                        "x" => destination.x = coordinate,
                                        "y" => destination.y = coordinate,
                                        _ => destination.z = coordinate,
                                    }
                                    move_object_with_collisions(
                                        self.project.world.active_scene_mut(), &self.project.tile_definitions, id, destination,
                                    ).map(|_| ())
                                }
                                (None, _) => Err(format!("Objet introuvable : {object_id}")),
                                (_, Err(error)) => Err(error),
                            }
                        } else {
                            match self.active_scene_mut().get_object_mut(id) {
                                Some(obj) => apply_script_object_property(obj, &key, value),
                                None => Err(format!("Objet introuvable : {object_id}")),
                            }
                        };
                        if let Err(error) = result {
                            let level = if is_position { log::Level::Warn } else { log::Level::Error };
                            self.store_log(level, format!("Propriété objet `{key}` : {error}"));
                        }
                    }
                }
                GameCommand::SetGlobal { key, value } => {
                    self.push_log(
                        log::Level::Debug,
                        format!("SetGlobal: {} = {:?}", key, value),
                    );
                }
                GameCommand::LoadScene { name } => {
                    match self.load_scene_by_name(&name) {
                        Ok(true) => self.push_log(log::Level::Info, format!("Scène « {name} » chargée.")),
                        Ok(false) => {}
                        Err(error) => self.push_log(log::Level::Error, error),
                    }
                }
                GameCommand::UnloadScene => {
                    match self.unload_active_scene() {
                        Ok(true) => {
                            let name = self.active_scene().meta.name.clone();
                            self.push_log(log::Level::Info, format!("Retour à la scène « {name} »."));
                        }
                        Ok(false) => {}
                        Err(error) => self.push_log(log::Level::Warn, error),
                    }
                }
                GameCommand::TransferObject { object_id, scene_name, x, y } => {
                    if let Err(error) = self.transfer_object_to_scene(&object_id, &scene_name, x, y) {
                        self.push_log(log::Level::Warn, error);
                    }
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

    fn transfer_object_to_scene(
        &mut self,
        object_id: &str,
        scene_name: &str,
        x: i32,
        y: i32,
    ) -> Result<(), String> {
        let uuid = uuid::Uuid::parse_str(object_id)
            .map_err(|_| format!("Identifiant d'objet invalide : {object_id}"))?;
        let id = world::object::ObjectId(uuid);
        let current_index = self.project.world.active_scene_index;
        let target_index = self.project.world.scenes.iter()
            .position(|scene| scene.meta.name == scene_name)
            .ok_or_else(|| format!("Aucune scène ne porte le nom « {scene_name} »."))?;
        let mut destination = self.project.world.scenes[current_index].get_object(id)
            .cloned()
            .ok_or_else(|| format!("Objet introuvable dans la scène actuelle : {object_id}"))?;
        destination.tile_pos.x = x;
        destination.tile_pos.y = y;
        let destination_z = destination.tile_pos.z;
        let object_name = destination.name.clone();

        if current_index == target_index {
            let moved = move_object_with_collisions(
                &mut self.project.world.scenes[current_index],
                &self.project.tile_definitions,
                id,
                destination.tile_pos,
            )?;
            if !moved {
                return Err(format!("Transfert de « {object_name} » bloqué par une collision."));
            }
            return Ok(());
        }

        if self.project.world.scenes[target_index].get_object(id).is_some() {
            return Err(format!("L'objet {object_id} existe déjà dans la scène « {scene_name} »."));
        }

        self.project.world.scenes[target_index].add_object(destination);
        let moved = match move_object_with_collisions(
            &mut self.project.world.scenes[target_index],
            &self.project.tile_definitions,
            id,
            TilePos::with_z(x, y, destination_z),
        ) {
            Ok(moved) => moved,
            Err(error) => {
                self.project.world.scenes[target_index].remove_object(id);
                return Err(error);
            }
        };
        if !moved {
            self.project.world.scenes[target_index].remove_object(id);
            return Err(format!("Transfert de « {object_name} » vers « {scene_name} » bloqué par une collision."));
        }

        self.project.world.scenes[current_index].remove_object(id);
        self.push_log(log::Level::Info, format!(
            "Objet « {object_name} » transféré vers « {scene_name} » ; la scène active n'a pas changé."
        ));
        Ok(())
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
        let id = self.next_log_id;
        self.next_log_id = self.next_log_id.wrapping_add(1).max(1);
        self.log_messages.push_back(LogEntry { level, message: msg, repetitions: 1, id });
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

fn object_footprints_overlap(a: &world::object::SceneObject, b: &world::object::SceneObject) -> bool {
    if a.tile_pos.z != b.tile_pos.z { return false; }
    let (ax1, ay1) = (i64::from(a.tile_pos.x), i64::from(a.tile_pos.y));
    let (bx1, by1) = (i64::from(b.tile_pos.x), i64::from(b.tile_pos.y));
    let ax2 = ax1 + i64::from(a.footprint.width.max(1));
    let ay2 = ay1 + i64::from(a.footprint.height.max(1));
    let bx2 = bx1 + i64::from(b.footprint.width.max(1));
    let by2 = by1 + i64::from(b.footprint.height.max(1));
    ax1 < bx2 && bx1 < ax2 && ay1 < by2 && by1 < ay2
}

fn move_object_with_collisions(
    scene: &mut Scene,
    tile_definitions: &[world::grid::TileDefinition],
    object_id: world::object::ObjectId,
    destination: TilePos,
) -> Result<bool, String> {
    let object = scene.get_object(object_id)
        .ok_or_else(|| format!("Objet introuvable : {object_id}"))?
        .clone();
    if object.has_collisions {
        let width = object.footprint.width.max(1) as i64;
        let height = object.footprint.height.max(1) as i64;
        let left = destination.x as i64;
        let top = destination.y as i64;
        let right = left + width;
        let bottom = top + height;

        for y in top..bottom {
            for x in left..right {
                if x < i32::MIN as i64 || x > i32::MAX as i64
                    || y < i32::MIN as i64 || y > i32::MAX as i64 {
                    return Ok(false);
                }
                let tile = scene.get_tile(TilePos::new(x as i32, y as i32));
                if tile_definitions.iter().any(|definition| {
                    definition.id == tile.tile_type && definition.has_collisions
                }) {
                    return Ok(false);
                }
            }
        }

        let overlaps_object = scene.objects.values().any(|other| {
            if other.id == object_id || !other.has_collisions || other.tile_pos.z != destination.z {
                return false;
            }
            let other_left = other.tile_pos.x as i64;
            let other_top = other.tile_pos.y as i64;
            let other_right = other_left + other.footprint.width.max(1) as i64;
            let other_bottom = other_top + other.footprint.height.max(1) as i64;
            left < other_right && right > other_left && top < other_bottom && bottom > other_top
        });
        if overlaps_object {
            return Ok(false);
        }
    }

    if let Some(object) = scene.get_object_mut(object_id) {
        object.tile_pos = destination;
    }
    Ok(true)
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

fn synchronize_custom_object_instances(
    scenes: &mut [Scene],
    definitions: &[CustomObjectTypeDefinition],
    scene_names: &[String],
) {
    for scene in scenes {
        for object in scene.objects.values_mut() {
            let Some(type_id) = object.custom_type_id else { continue };
            let Some(definition) = definitions.iter().find(|definition| definition.id == type_id) else { continue };
            object.object_type = ObjectType::Custom(definition.name.clone());
            let valid_keys: HashSet<&str> = definition.properties.iter()
                .map(|property| property.key.as_str()).collect();
            object.properties.retain(|key, _| key == "texture" || valid_keys.contains(key.as_str()));

            for property in &definition.properties {
                let mut default = property.default_value.clone();
                if property.kind == CustomPropertyKind::Scene {
                    if let PropValue::Str(value) = &default {
                        if !scene_names.iter().any(|name| name == value) {
                            default = PropValue::Str(scene_names.first().cloned().unwrap_or_default());
                        }
                    }
                }
                let valid = object.properties.get(&property.key).is_some_and(|value| match property.kind {
                    CustomPropertyKind::Text | CustomPropertyKind::Scene => matches!(value, PropValue::Str(_)),
                    CustomPropertyKind::Integer => matches!(value, PropValue::Int(_)),
                    CustomPropertyKind::Float => matches!(value, PropValue::Float(_)),
                    CustomPropertyKind::Boolean => matches!(value, PropValue::Bool(_)),
                });
                if !valid {
                    let value = match (property.kind, object.properties.get(&property.key)) {
                        (CustomPropertyKind::Float, Some(PropValue::Int(value))) => PropValue::Float(*value as f64),
                        _ => default,
                    };
                    object.properties.insert(property.key.clone(), value);
                }
            }
        }
    }
}

fn remap_custom_scene_properties(project: &mut Project, old_name: &str, new_name: Option<&str>) {
    let fallback_scene = project.world.scenes.first().map(|scene| scene.meta.name.clone()).unwrap_or_default();
    let mut scene_keys = std::collections::HashMap::<uuid::Uuid, HashSet<String>>::new();
    for definition in &mut project.object_types {
        let keys: HashSet<String> = definition.properties.iter()
            .filter(|property| property.kind == CustomPropertyKind::Scene)
            .map(|property| property.key.clone()).collect();
        if keys.is_empty() { continue; }
        for property in &mut definition.properties {
            if property.kind == CustomPropertyKind::Scene
                && property.default_value.as_str() == Some(old_name) {
                property.default_value = PropValue::Str(new_name.unwrap_or(&fallback_scene).to_owned());
            }
        }
        scene_keys.insert(definition.id, keys);
    }
    for scene in &mut project.world.scenes {
        for object in scene.objects.values_mut() {
            let Some(type_id) = object.custom_type_id else { continue };
            let Some(keys) = scene_keys.get(&type_id) else { continue };
            for key in keys {
                if object.properties.get(key).and_then(PropValue::as_str) == Some(old_name) {
                    object.properties.insert(key.clone(), PropValue::Str(new_name.unwrap_or(&fallback_scene).to_owned()));
                }
            }
        }
    }
}

fn format_log_entry(entry: &LogEntry) -> String {
    if entry.repetitions > 1 {
        format!("{}  (répété {} fois)", entry.message, entry.repetitions)
    } else {
        entry.message.clone()
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
    pending_tile_definition_deletion: Option<u16>,
    pending_scene_rename: Option<usize>,
    scene_rename_buffer: String,
    scene_rename_error: Option<String>,
    pending_scene_deletion: Option<usize>,
    selected_log_ids: HashSet<u64>,
    log_selection_anchor: Option<u64>,
    log_selection_focused: bool,
    log_panel_rect: Option<egui::Rect>,
    pending_log_copy: bool,
    /// Chemin du projet courant pour la sauvegarde.
    project_path: Option<std::path::PathBuf>,
    /// Le player charge un projet directement en lecture et masque l'UI d'édition.
    player_mode: bool,
    export_receiver: Option<Receiver<Result<std::path::PathBuf, String>>>,
    export_status: Option<String>,
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
            pending_tile_definition_deletion: None,
            pending_scene_rename: None,
            scene_rename_buffer: String::new(),
            scene_rename_error: None,
            pending_scene_deletion: None,
            selected_log_ids: HashSet::new(),
            log_selection_anchor: None,
            log_selection_focused: false,
            log_panel_rect: None,
            pending_log_copy: false,
            project_path: None,
            player_mode: false,
            export_receiver: None,
            export_status: None,
        }
    }

    /// Charge un projet et démarre directement sa simulation sans interface d'édition.
    pub fn new_player(path: &std::path::Path) -> anyhow::Result<Self> {
        let path = path.canonicalize()?;
        let mut app = Self::new();
        app.player_mode = true;
        app.show_log = false;
        app.load_scene(&path)?;
        app.load_world_script()?;
        app.load_object_scripts()?;
        app.state.play();
        app.runtime_camera = app.state.active_scene().main_camera;
        Ok(app)
    }

    pub fn is_player_mode(&self) -> bool {
        self.player_mode
    }

    pub fn shutdown_player(&mut self) {
        if self.player_mode {
            self.state.stop();
        }
    }

    /// Appelé à chaque frame par la boucle winit.
    pub fn update(&mut self, ctx: &Context, delta_time: f32) {
        self.poll_export_job();
        if delta_time.is_finite() && delta_time > 0.0 {
            let instantaneous = 1.0 / delta_time;
            self.fps_estimate = if self.fps_estimate == 0.0 {
                instantaneous
            } else {
                self.fps_estimate * 0.9 + instantaneous * 0.1
            };
        }
        let scene_before_update = self.state.project.world.active_scene_index;
        // Met à jour la simulation si en cours.
        self.state.update(delta_time);
        if scene_before_update != self.state.project.world.active_scene_index {
            self.runtime_camera = self.state.active_scene().main_camera;
            self.runtime_camera_position = None;
            self.runtime_camera_zoom = None;
            self.scene_view.selected_object = None;
            self.scene_view.selected_objects.clear();
            self.scene_view.selected_tiles.clear();
        }

        if self.player_mode {
            self.update_player_view(ctx, delta_time);
            return;
        }

        // Synchronise la sélection avec la vue scène.
        self.scene_view.selected_object = self.state.selected;
        // Synchronise le type de tuile depuis la palette.
        self.scene_view.paint_tile_type = self.left_panel.selected_tile_type;

        self.draw_ui(ctx, delta_time);

        // Récupère la sélection de la vue scène.
        self.state.selected = self.scene_view.selected_object;
    }

    fn poll_export_job(&mut self) {
        let result = match self.export_receiver.as_ref().map(Receiver::try_recv) {
            Some(Ok(result)) => Some(result),
            Some(Err(TryRecvError::Disconnected)) => Some(Err("Le processus d'export s'est interrompu.".into())),
            Some(Err(TryRecvError::Empty)) | None => None,
        };
        let Some(result) = result else { return };
        self.export_receiver = None;
        self.show_log = true;
        match result {
            Ok(path) => {
                let size = std::fs::metadata(&path).map(|metadata| metadata.len()).unwrap_or(0);
                let message = format!("Export correctement terminé : {} ({size} octets)", path.display());
                self.export_status = Some(message.clone());
                self.state.push_log(log::Level::Info, message);
            }
            Err(error) => {
                let message = format!("Échec de l'export : {error}");
                self.export_status = Some(message.clone());
                self.state.push_log(log::Level::Error, message);
            }
        }
    }

    fn update_player_view(&mut self, ctx: &Context, delta_time: f32) {
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
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
        }

        let asset_root = self.project_path.as_ref()
            .and_then(|path| path.parent())
            .map(std::path::Path::to_path_buf)
            .unwrap_or_default();
        egui::CentralPanel::default().frame(egui::Frame::default()).show(ctx, |ui| {
            self.scene_view.show(
                ui,
                self.state.project.world.active_scene_mut(),
                &self.state.project.tile_definitions,
                crate::toolbar::EditorTool::Select,
                &asset_root,
                delta_time,
                None,
                false,
                true,
                false,
                false,
                &mut self.runtime_camera,
            );
        });
    }

    /// Capture Ctrl+C/Ctrl+V avant qu'egui-winit les traite comme des raccourcis
    /// du presse-papiers texte.
    pub fn queue_clipboard_shortcut(&mut self, copy: bool) -> bool {
        let has_selected_logs = self.show_log && self.log_selection_focused && self.state.log_messages.iter()
            .any(|entry| self.selected_log_ids.contains(&entry.id));
        if copy && has_selected_logs {
            self.pending_log_copy = true;
            return true;
        }
        if !self.state.game_loop.is_editor()
            || self.toolbar.active_tool != crate::toolbar::EditorTool::Select
        {
            return false;
        }
        self.scene_view.request_clipboard_shortcut(copy);
        true
    }

    fn selected_logs_as_text(&self) -> String {
        self.state.log_messages.iter()
            .filter(|entry| self.selected_log_ids.contains(&entry.id))
            .map(format_log_entry)
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn draw_ui(&mut self, ctx: &Context, delta_time: f32) {
        self.selected_log_ids.retain(|id| self.state.log_messages.iter().any(|entry| entry.id == *id));
        let pointer_click = ctx.input(|input| {
            (input.pointer.any_click(), input.pointer.latest_pos())
        });
        if let (true, Some(position)) = pointer_click {
            if !self.log_panel_rect.is_some_and(|rect| rect.contains(position)) {
                self.log_selection_focused = false;
            }
        }
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
        let export_in_progress = self.export_receiver.is_some();
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
                export_in_progress,
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
                } else if let Some(index) = self.toolbar.scene_rename_requested {
                    if let Some(scene) = self.state.project.world.scenes.get(index) {
                        self.scene_rename_buffer = scene.meta.name.clone();
                        self.pending_scene_rename = Some(index);
                        self.scene_rename_error = None;
                    }
                } else if let Some(index) = self.toolbar.scene_delete_requested {
                    if self.state.project.world.scenes.len() > 1 {
                        self.pending_scene_deletion = Some(index);
                    }
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
                                    let _ = self.load_object_scripts();
                                    self.show_world_settings = false;
                                    self.runtime_camera = self.state.active_scene().main_camera;
                                    self.runtime_camera_position = None;
                                    self.runtime_camera_zoom = None;
                                    self.state.play();
                                    self.runtime_camera = self.state.active_scene().main_camera;
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

        if self.toolbar.export_requested && self.state.game_loop.is_editor() {
            self.export_current_project();
        }

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
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
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
            let main_scene_names = self.state.project.world.scenes.iter()
                .map(|scene| scene.meta.name.clone()).collect::<Vec<_>>();
            let mut main_scene_index = self.state.project.world.main_scene_index;
            let mut main_scene_changed = false;
            let mut choose_script = false;
            let mut create_script = false;
            egui::Window::new("Paramètres du monde")
                .open(&mut self.show_world_settings)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!("Monde : {}", self.state.project.world.name));
                    ui.horizontal(|ui| {
                        ui.label("Scène principale (Main) :");
                        let selected_name = main_scene_names.get(main_scene_index)
                            .map(String::as_str).unwrap_or("Aucune");
                        egui::ComboBox::from_id_salt("world_main_scene")
                            .selected_text(selected_name)
                            .show_ui(ui, |ui| {
                                for (index, name) in main_scene_names.iter().enumerate() {
                                    if ui.selectable_value(&mut main_scene_index, index, name).changed() {
                                        main_scene_changed = true;
                                    }
                                }
                            });
                    });
                    ui.separator();
                    ui.label(egui::RichText::new("Script Python du monde").strong());
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

            if main_scene_changed {
                self.state.project.world.main_scene_index = main_scene_index;
            }

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
                if let Some(status) = &self.export_status {
                    ui.separator();
                    ui.label(status);
                }

                // Toggle log.
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.toggle_value(&mut self.show_log, "📋 Log");
                });
            });
        });

        // ── Log en bas si activé ──────────────────────────────────────────────
        if self.show_log {
            let response = egui::TopBottomPanel::bottom("log_panel")
                .resizable(true)
                .default_height(120.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading("Log");
                        let has_selected_logs = !self.selected_log_ids.is_empty();
                        if ui.add_enabled(has_selected_logs, egui::Button::new("📋 Copier sélection"))
                            .clicked() {
                            self.pending_log_copy = true;
                            self.log_selection_focused = true;
                        }
                        if ui.small_button("🗑 Effacer").clicked() {
                            self.state.log_messages.clear();
                            self.selected_log_ids.clear();
                            self.log_selection_anchor = None;
                            self.log_selection_focused = false;
                        }
                    });
                    ui.separator();
                    let entries: Vec<LogEntry> = self.state.log_messages.iter().cloned().collect();
                    egui::ScrollArea::vertical()
                        .auto_shrink(false)
                        .stick_to_bottom(true)
                        .show_rows(ui, 18.0, entries.len(), |ui, rows| {
                            for row in rows {
                                let entry = &entries[row];
                                let color = match entry.level {
                                    log::Level::Error => egui::Color32::from_rgb(255, 100, 100),
                                    log::Level::Warn  => egui::Color32::from_rgb(255, 200, 80),
                                    log::Level::Info  => egui::Color32::WHITE,
                                    _                 => egui::Color32::GRAY,
                                };
                                let text = format_log_entry(entry);
                                let selected = self.selected_log_ids.contains(&entry.id);
                                let response = ui.selectable_label(
                                    selected,
                                    egui::RichText::new(text).color(color).monospace(),
                                ).on_hover_text("Cliquer pour sélectionner ; Ctrl+C copie les lignes sélectionnées");
                                if response.clicked() {
                                    self.log_selection_focused = true;
                                    let modifiers = ui.input(|input| input.modifiers);
                                    if modifiers.shift {
                                        if let Some(anchor) = self.log_selection_anchor
                                            .and_then(|id| entries.iter().position(|candidate| candidate.id == id)) {
                                            self.selected_log_ids.clear();
                                            for index in anchor.min(row)..=anchor.max(row) {
                                                self.selected_log_ids.insert(entries[index].id);
                                            }
                                        } else {
                                            self.selected_log_ids.clear();
                                            self.selected_log_ids.insert(entry.id);
                                            self.log_selection_anchor = Some(entry.id);
                                        }
                                    } else if modifiers.ctrl || modifiers.command {
                                        if !self.selected_log_ids.remove(&entry.id) {
                                            self.selected_log_ids.insert(entry.id);
                                        }
                                        self.log_selection_anchor = Some(entry.id);
                                    } else {
                                        self.selected_log_ids.clear();
                                        self.selected_log_ids.insert(entry.id);
                                        self.log_selection_anchor = Some(entry.id);
                                    }
                                }
                            }
                        });
                });
            self.log_panel_rect = Some(response.response.rect);
        } else {
            self.log_panel_rect = None;
            self.log_selection_focused = false;
        }
        if std::mem::take(&mut self.pending_log_copy) {
            let text = self.selected_logs_as_text();
            if !text.is_empty() { ctx.copy_text(text); }
        }

        // ── Panneau gauche ────────────────────────────────────────────────────
        let editing_enabled = self.state.game_loop.is_editor();
        let project_root = self.project_path.as_deref()
            .and_then(|path| project_browser_directory(path).ok())
            .and_then(|path| path.canonicalize().ok());
        let active_project_file = self.project_path.as_deref()
            .and_then(|path| path.canonicalize().ok());
        let scene_names: Vec<String> = self.state.project.world.scenes.iter()
            .map(|scene| scene.meta.name.clone()).collect();
        let mut tile_texture_picker = None;
        let mut object_type_texture_picker = None;
        let mut object_type_script_picker = None;
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(220.0)
            .show(ctx, |ui| {
                ui.add_enabled_ui(editing_enabled, |ui| {
                let selection_before_panel = self.state.selected;
                let project = &mut self.state.project;
                let action = self.left_panel.show(
                    ui,
                    project.world.active_scene_mut(),
                    &mut self.state.selected,
                    project_root.as_deref(),
                    active_project_file.as_deref(),
                    &mut project.tile_definitions,
                    &mut project.object_types,
                    &scene_names,
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
                    PanelAction::DeletePath(path) => {
                        match self.delete_project_path(&path) {
                            Ok(()) => self.state.push_log(log::Level::Info, format!("Fichier ou dossier supprimé : {}", path.display())),
                            Err(error) => self.state.push_log(log::Level::Error, format!("Suppression : {error}")),
                        }
                    }
                    PanelAction::DeleteObject(id) => {
                        self.pending_deletion = Some((vec![id], Vec::new()));
                    }
                    PanelAction::ChooseTileTexture(id) => tile_texture_picker = Some(id),
                    PanelAction::DeleteTileDefinition(id) if id > 6 => {
                        self.pending_tile_definition_deletion = Some(id);
                    }
                    PanelAction::AddObject => {
                        let id = self.state.active_scene_mut().spawn(
                            "Nouvel objet",
                            world::object::ObjectType::Custom("custom".to_string()),
                            TilePos::new(0, 0),
                        );
                        self.state.selected = Some(id);
                    }
                    PanelAction::ObjectTypesChanged => {
                        synchronize_custom_object_instances(
                            &mut self.state.project.world.scenes,
                            &self.state.project.object_types,
                            &scene_names,
                        );
                    }
                    PanelAction::RenameCustomProperty(type_id, old_key, new_key) => {
                        for scene in &mut self.state.project.world.scenes {
                            for object in scene.objects.values_mut() {
                                if object.custom_type_id == Some(type_id) {
                                    if let Some(value) = object.properties.remove(&old_key) {
                                        object.properties.insert(new_key.clone(), value);
                                    }
                                }
                            }
                        }
                        synchronize_custom_object_instances(
                            &mut self.state.project.world.scenes,
                            &self.state.project.object_types,
                            &scene_names,
                        );
                    }
                    PanelAction::AddCustomObject(type_id) => {
                        if let Some(definition) = self.state.project.object_types.iter()
                            .find(|definition| definition.id == type_id).cloned() {
                            let position = self.scene_view.hovered_tile.unwrap_or_else(|| TilePos::new(0, 0));
                            let mut object = SceneObject::new(
                                definition.name.clone(),
                                ObjectType::Custom(definition.name.clone()),
                                position,
                            );
                            object.custom_type_id = Some(type_id);
                            object.has_collisions = definition.defaults.has_collisions;
                            object.hidden = definition.defaults.hidden;
                            object.fit_texture_to_diamond = definition.defaults.fit_texture_to_diamond;
                            object.footprint = definition.defaults.footprint;
                            object.rotation = definition.defaults.rotation;
                            object.layer = definition.defaults.layer;
                            object.script = definition.defaults.script.clone();
                            object.asset_path = definition.defaults.texture.clone();
                            if let Some(texture) = &definition.defaults.texture {
                                object.properties.insert("texture".to_owned(), PropValue::Str(texture.clone()));
                            }
                            for property in definition.properties {
                                object.properties.insert(property.key, property.default_value);
                            }
                            let id = self.state.active_scene_mut().add_object(object);
                            self.state.selected = Some(id);
                            self.scene_view.selected_object = Some(id);
                            self.scene_view.selected_objects.clear();
                            self.scene_view.selected_objects.insert(id);
                        }
                    }
                    PanelAction::DeleteCustomObjectType(type_id) => {
                        self.state.project.object_types.retain(|definition| definition.id != type_id);
                        for scene in &mut self.state.project.world.scenes {
                            for object in scene.objects.values_mut() {
                                if object.custom_type_id == Some(type_id) {
                                    object.custom_type_id = None;
                                    object.object_type = ObjectType::Custom("custom".to_owned());
                                }
                            }
                        }
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
                    _ => {}
                }
                });
            });

        if let Some(tile_id) = tile_texture_picker {
            match choose_tile_texture_file() {
                Ok(Some(path)) => match self.import_texture(&path) {
                    Ok(stored_path) => {
                        if let Some(tile) = self.state.project.tile_definitions.iter_mut()
                            .find(|tile| tile.id == tile_id) {
                            tile.texture = Some(stored_path);
                        }
                    }
                    Err(error) => self.state.push_log(log::Level::Error,
                        format!("Import de texture de tuile : {error}")),
                },
                Ok(None) => {}
                Err(error) => self.state.push_log(log::Level::Error,
                    format!("Sélecteur de texture : {error}")),
            }
        }

        // ── Inspecteur à droite ───────────────────────────────────────────────
        let mut texture_picker_object = None;
        let mut script_picker_object = None;
        let mut script_create_object = None;
        let mut type_definition_edit = None;
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
                    if let Some(type_id) = self.left_panel.selected_object_type_id() {
                        let edit = Inspector::show_type_definition(
                            ui,
                            type_id,
                            &mut self.state.project.object_types,
                            &scene_names,
                            &mut request_texture_picker,
                            &mut request_script_picker,
                        );
                        if request_texture_picker { object_type_texture_picker = Some(type_id); }
                        if request_script_picker { object_type_script_picker = Some(type_id); }
                        type_definition_edit = Some(edit);
                    } else {
                        Inspector::show(
                            ui,
                            self.state.project.world.active_scene_mut(),
                            selected,
                            self.scene_view.selected_objects.len(),
                            &self.state.project.object_types,
                            &scene_names,
                            &mut request_texture_picker,
                            &mut request_script_picker,
                            &mut request_script_create,
                        );
                        if request_texture_picker { texture_picker_object = selected; }
                        if request_script_picker { script_picker_object = selected; }
                        if request_script_create { script_create_object = selected; }
                    }
                  });
                });
            });

        if let Some(edit) = type_definition_edit {
            if let Some((type_id, old_key, new_key)) = edit.renamed_property {
                for scene in &mut self.state.project.world.scenes {
                    for object in scene.objects.values_mut() {
                        if object.custom_type_id == Some(type_id) {
                            if let Some(value) = object.properties.remove(&old_key) {
                                object.properties.insert(new_key.clone(), value);
                            }
                        }
                    }
                }
            }
            if edit.changed {
                synchronize_custom_object_instances(
                    &mut self.state.project.world.scenes,
                    &self.state.project.object_types,
                    &scene_names,
                );
            }
        }

        if let Some(type_id) = object_type_texture_picker {
            match choose_texture_file() {
                Ok(Some(path)) => match self.import_texture(&path) {
                    Ok(stored_path) => {
                        if let Some(definition) = self.state.project.object_types.iter_mut()
                            .find(|definition| definition.id == type_id) {
                            definition.defaults.texture = Some(stored_path);
                        }
                    }
                    Err(error) => self.state.push_log(log::Level::Error,
                        format!("Import de texture du type : {error}")),
                },
                Ok(None) => {}
                Err(error) => self.state.push_log(log::Level::Error,
                    format!("Sélecteur de texture du type : {error}")),
            }
        }

        if let Some(type_id) = object_type_script_picker {
            match choose_script_file() {
                Ok(Some(path)) => match self.import_object_script(&path) {
                    Ok(stored_path) => {
                        if let Some(definition) = self.state.project.object_types.iter_mut()
                            .find(|definition| definition.id == type_id) {
                            definition.defaults.script = Some(stored_path);
                        }
                    }
                    Err(error) => self.state.push_log(log::Level::Error,
                        format!("Import du script par défaut du type : {error}")),
                },
                Ok(None) => {}
                Err(error) => self.state.push_log(log::Level::Error,
                    format!("Sélecteur de script du type : {error}")),
            }
        }

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
                &self.state.project.tile_definitions,
                self.toolbar.active_tool,
                &asset_root,
                delta_time,
                fps_overlay,
                editing_enabled,
                !editing_enabled,
                grid_visible,
                true,
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
                    let name = self.state.active_scene().get_object(id).map(|object| object.name.clone());
                    let removed = self.state.active_scene_mut().remove_object(id).is_some();
                    self.scene_view.selected_objects.remove(&id);
                    if self.state.selected == Some(id) { self.state.selected = None; }
                    if self.scene_view.selected_object == Some(id) { self.scene_view.selected_object = None; }
                    if removed {
                        if let Some(name) = name {
                            self.state.push_log(log::Level::Info, format!("Objet « {name} » supprimé."));
                        }
                    }
                }
                for tile in &tiles {
                    self.state.active_scene_mut().clear_tile(*tile);
                    self.scene_view.selected_tiles.remove(tile);
                }
            }
        }
        let mut confirm_scene_rename = false;
        let mut cancel_scene_rename = false;
        if self.pending_scene_rename.is_some() {
            egui::Window::new("Renommer la scène")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                        cancel_scene_rename = true;
                    }
                    if ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                        confirm_scene_rename = true;
                    }
                    ui.text_edit_singleline(&mut self.scene_rename_buffer);
                    if let Some(error) = &self.scene_rename_error {
                        ui.colored_label(egui::Color32::RED, error);
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Annuler").clicked() { cancel_scene_rename = true; }
                        if ui.button("Renommer").clicked() { confirm_scene_rename = true; }
                    });
                });
        }
        if cancel_scene_rename {
            self.pending_scene_rename = None;
            self.scene_rename_error = None;
        } else if confirm_scene_rename {
            if let Some(index) = self.pending_scene_rename {
                let name = self.scene_rename_buffer.trim().to_owned();
                let duplicate = self.state.project.world.scenes.iter().enumerate()
                    .any(|(other_index, scene)| other_index != index && scene.meta.name.eq_ignore_ascii_case(&name));
                if name.is_empty() {
                    self.scene_rename_error = Some("Le nom ne peut pas être vide.".to_owned());
                } else if duplicate {
                    self.scene_rename_error = Some("Une scène porte déjà ce nom.".to_owned());
                } else if let Some(scene) = self.state.project.world.scenes.get_mut(index) {
                    let old_name = scene.meta.name.clone();
                    scene.meta.name = name.clone();
                    remap_custom_scene_properties(&mut self.state.project, &old_name, Some(&name));
                    self.pending_scene_rename = None;
                    self.scene_rename_error = None;
                    history_action = true;
                } else {
                    self.pending_scene_rename = None;
                }
            }
        }

        let mut confirm_scene_deletion = false;
        let mut cancel_scene_deletion = false;
        if let Some(index) = self.pending_scene_deletion {
            let name = self.state.project.world.scenes.get(index)
                .map(|scene| scene.meta.name.clone()).unwrap_or_default();
            egui::Window::new("Supprimer la scène ?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                        cancel_scene_deletion = true;
                    }
                    if ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                        confirm_scene_deletion = true;
                    }
                    ui.label(format!("Supprimer la scène « {name} » ?"));
                    ui.horizontal(|ui| {
                        if ui.button("Annuler").clicked() { cancel_scene_deletion = true; }
                        if ui.button("Supprimer").clicked() { confirm_scene_deletion = true; }
                    });
                });
        }
        if cancel_scene_deletion {
            self.pending_scene_deletion = None;
        } else if confirm_scene_deletion {
            if let Some(index) = self.pending_scene_deletion.take() {
                if self.state.project.world.scenes.len() > 1 {
                    let was_active = self.state.project.world.active_scene_index == index;
                    if let Some(removed) = self.state.project.world.remove_scene(index) {
                        remap_custom_scene_properties(&mut self.state.project, &removed.meta.name, None);
                        let names: Vec<String> = self.state.project.world.scenes.iter()
                            .map(|scene| scene.meta.name.clone()).collect();
                        synchronize_custom_object_instances(
                            &mut self.state.project.world.scenes,
                            &self.state.project.object_types,
                            &names,
                        );
                        self.state.scene_stack.clear();
                        if was_active {
                            self.state.selected = None;
                            self.scene_view.selected_object = None;
                            self.scene_view.selected_objects.clear();
                            self.scene_view.selected_tiles.clear();
                        }
                        self.state.push_log(log::Level::Info,
                            format!("Scène « {} » supprimée.", removed.meta.name));
                        history_action = true;
                    }
                }
            }
        }

        let mut confirm_tile_definition_deletion = false;
        let mut cancel_tile_definition_deletion = false;
        if let Some(tile_id) = self.pending_tile_definition_deletion {
            let tile_name = self.state.project.tile_definitions.iter()
                .find(|tile| tile.id == tile_id)
                .map(|tile| tile.name.clone())
                .unwrap_or_else(|| format!("tuile {tile_id}"));
            egui::Window::new("Supprimer une tuile personnalisée ?")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                        cancel_tile_definition_deletion = true;
                    }
                    if ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                        confirm_tile_definition_deletion = true;
                    }
                    ui.label(format!("Supprimer « {tile_name} » et effacer ses cases peintes dans toutes les scènes ?"));
                    ui.horizontal(|ui| {
                        if ui.button("Annuler").clicked() { cancel_tile_definition_deletion = true; }
                        if ui.button("Supprimer").clicked() { confirm_tile_definition_deletion = true; }
                    });
                });
        }
        if cancel_tile_definition_deletion {
            self.pending_tile_definition_deletion = None;
        } else if confirm_tile_definition_deletion {
            if let Some(tile_id) = self.pending_tile_definition_deletion.take() {
                if tile_id > 6 {
                    let tile_name = self.state.project.tile_definitions.iter()
                        .find(|tile| tile.id == tile_id)
                        .map(|tile| tile.name.clone())
                        .unwrap_or_else(|| format!("tuile {tile_id}"));
                    self.state.project.tile_definitions.retain(|tile| tile.id != tile_id);
                    let active_scene = self.state.project.world.active_scene_index;
                    let mut removed_active_tiles = Vec::new();
                    for (index, scene) in self.state.project.world.scenes.iter_mut().enumerate() {
                        let removed = remove_tile_definition_from_scene(scene, tile_id);
                        if index == active_scene { removed_active_tiles = removed; }
                    }
                    for tile in removed_active_tiles {
                        self.scene_view.selected_tiles.remove(&tile);
                    }
                    self.left_panel.selected_tile_type = self.state.project.tile_definitions
                        .iter().map(|tile| tile.id).min().unwrap_or(1);
                    self.state.push_log(log::Level::Info,
                        format!("Tuile personnalisée « {tile_name} » supprimée du projet."));
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

        let tile_texture_references: Vec<(u16, String)> = self.state.project.tile_definitions.iter()
            .filter_map(|tile| tile.texture.as_ref().filter(|path| !path.trim().is_empty())
                .map(|path| (tile.id, path.clone())))
            .collect();
        let mut tile_texture_updates = Vec::new();
        for (tile_id, stored_path) in tile_texture_references {
            let stored = std::path::Path::new(&stored_path);
            let source_path = if stored.is_absolute() { stored.to_owned() } else { source_root.join(stored) };
            let source = match source_path.canonicalize() {
                Ok(source) => source,
                Err(error) => {
                    self.state.push_log(log::Level::Warn,
                        format!("Texture de tuile manquante `{stored_path}` : {error}"));
                    continue;
                }
            };
            let relative = if let Some(relative) = copied_sources.get(&source) {
                relative.clone()
            } else if let Ok(within_assets) = source.strip_prefix(&target_asset_dir) {
                let relative = asset_prefix.join(within_assets).to_string_lossy().replace('\\', "/");
                copied_sources.insert(source, relative.clone());
                relative
            } else {
                let destination = copy_asset_to_directory(&source, &target_asset_dir)?;
                let relative = asset_prefix.join(destination.file_name().unwrap_or_default())
                    .to_string_lossy().replace('\\', "/");
                copied_sources.insert(source, relative.clone());
                relative
            };
            tile_texture_updates.push((tile_id, relative));
        }
        for (tile_id, relative) in tile_texture_updates {
            if let Some(tile) = self.state.project.tile_definitions.iter_mut().find(|tile| tile.id == tile_id) {
                tile.texture = Some(relative);
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
        write_tile_definition_files(&path, &self.state.project.tile_definitions)?;
        self.project_path = Some(path.clone());
        self.remember_recent_project(&path);
        self.state.push_log(log::Level::Info, format!("Projet sauvegardé : {}", path.display()));
        Ok(())
    }

    fn export_current_project(&mut self) {
        if self.export_receiver.is_some() {
            return;
        }
        if !self.state.project.world.script.as_ref().is_some_and(|script| !script.trim().is_empty()) {
            self.state.push_log(log::Level::Error,
                "Export impossible : définis d'abord le script du monde dans le menu Monde.".into());
            return;
        }

        let project_path = if let Some(path) = self.project_path.clone() {
            path
        } else {
            match choose_project_file(true) {
                Ok(Some(path)) => path,
                Ok(None) => return,
                Err(error) => {
                    self.state.push_log(log::Level::Error, format!("Sélecteur de projet : {error}"));
                    return;
                }
            }
        };
        let stem = project_path.file_stem().and_then(|name| name.to_str()).unwrap_or("jeu");
        let default_zip_name = format!("{stem}-windows.zip");
        let output_path = match choose_export_file(&default_zip_name) {
            Ok(Some(path)) => path,
            Ok(None) => return,
            Err(error) => {
                self.state.push_log(log::Level::Error, format!("Sélecteur d'export : {error}"));
                return;
            }
        };
        if let Err(error) = self.save_scene(&project_path) {
            self.state.push_log(log::Level::Error, format!("Sauvegarde avant export : {error}"));
            return;
        }
        let project_path = self.project_path.clone().unwrap_or(project_path);
        let script_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../scripts/export_game.py");
        if !script_path.is_file() {
            self.state.push_log(log::Level::Error, format!(
                "Script d'export introuvable : {}", script_path.display()));
            return;
        }

        let (sender, receiver) = std::sync::mpsc::channel();
        self.export_receiver = Some(receiver);
        self.export_status = Some("Export du jeu en cours…".into());
        self.show_log = true;
        self.state.push_log(log::Level::Info, format!(
            "Export du jeu démarré : {} → {}", project_path.display(), output_path.display()));
        std::thread::spawn(move || {
            let result = std::process::Command::new("python")
                .arg(script_path)
                .arg(&project_path)
                .arg("--output")
                .arg(&output_path)
                .env("PYTHONIOENCODING", "utf-8")
                .output()
                .map_err(|error| format!("Impossible de lancer Python : {error}"))
                .and_then(|output| {
                    if output.status.success() {
                        match std::fs::metadata(&output_path) {
                            Ok(metadata) if metadata.is_file() && metadata.len() > 0 => Ok(output_path),
                            Ok(_) => Err(format!("L'exporteur n'a pas produit d'archive valide : {}", output_path.display())),
                            Err(error) => Err(format!("Archive annoncée comme exportée mais introuvable : {error}")),
                        }
                    } else {
                        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
                        let mut details = Vec::new();
                        if !stdout.is_empty() { details.push(format!("stdout: {stdout}")); }
                        if !stderr.is_empty() { details.push(format!("stderr: {stderr}")); }
                        if details.is_empty() { details.push("aucun détail fourni par l'exporteur".into()); }
                        Err(format!("{} (code {:?})", details.join(" | "), output.status.code()))
                    }
                });
            let _ = sender.send(result);
        });
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
        let tile_texture_updates: Vec<(u16, String)> = self.state.project.tile_definitions.iter()
            .filter_map(|tile| tile.texture.as_deref()
                .and_then(|path| remap_project_reference(path, &root, &old, &new))
                .map(|path| (tile.id, path)))
            .collect();

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
        for (tile_id, texture) in tile_texture_updates {
            if let Some(tile) = self.state.project.tile_definitions.iter_mut().find(|tile| tile.id == tile_id) {
                tile.texture = Some(texture);
            }
        }
        self.left_panel.refresh_file_browser();
        if let Some(project_file) = self.project_path.clone() {
            let json = self.state.project.to_json()?;
            std::fs::write(&project_file, json)?;
            write_tile_definition_files(&project_file, &self.state.project.tile_definitions)?;
            self.state.push_log(log::Level::Info,
                format!("Références mises à jour après renommage : {}", project_file.display()));
        }
        Ok(())
    }

    fn delete_project_path(&mut self, requested: &std::path::Path) -> anyhow::Result<()> {
        let project_file = self.project_path.as_deref()
            .ok_or_else(|| anyhow::anyhow!("Aucun projet n'est ouvert"))?;
        let browser_root = project_browser_directory(project_file)?.canonicalize()?;
        let target = resolve_project_browser_path(Some(project_file), requested)?;
        anyhow::ensure!(target != browser_root, "Impossible de supprimer la racine du projet");
        anyhow::ensure!(target.exists(), "Le fichier ou dossier n'existe plus");

        let deleted_path = target.canonicalize()?;
        let project_root = project_file.parent()
            .ok_or_else(|| anyhow::anyhow!("Dossier du projet introuvable"))?.canonicalize()?;
        let referenced = |stored: &str| {
            let path = std::path::Path::new(stored);
            let resolved = if path.is_absolute() { path.to_path_buf() } else { project_root.join(path) };
            resolved.canonicalize().is_ok_and(|resolved| resolved.starts_with(&deleted_path))
        };

        for scene in &mut self.state.project.world.scenes {
            for object in scene.objects.values_mut() {
                if object.asset_path.as_deref().is_some_and(referenced) {
                    object.asset_path = None;
                }
                if object.properties.get("texture").and_then(world::object::PropValue::as_str).is_some_and(referenced) {
                    object.properties.remove("texture");
                }
                if object.script.as_deref().is_some_and(referenced) {
                    object.script = None;
                }
            }
        }
        if self.state.project.world.script.as_deref().is_some_and(referenced) {
            self.state.project.world.script = None;
        }
        for tile in &mut self.state.project.tile_definitions {
            if tile.texture.as_deref().is_some_and(referenced) {
                tile.texture = None;
            }
        }

        if target.is_dir() {
            std::fs::remove_dir_all(&target)?;
        } else {
            std::fs::remove_file(&target)?;
        }
        let json = self.state.project.to_json()?;
        std::fs::write(project_file, json)?;
        write_tile_definition_files(project_file, &self.state.project.tile_definitions)?;
        self.left_panel.refresh_file_browser();
        Ok(())
    }

    fn import_texture(&self, selected: &std::path::Path) -> anyhow::Result<String> {
        let extension = selected.extension().and_then(|value| value.to_str())
            .unwrap_or_default().to_ascii_lowercase();
        if !matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "gif") {
            anyhow::bail!("Formats acceptés : PNG, JPEG et GIF");
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
        let mut project = match Project::from_json(&json) {
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
        if let Err(error) = load_tile_definition_files(path, &mut project) {
            anyhow::bail!("Impossible de charger les définitions de tuiles du projet : {error}");
        }
        let scene_names: Vec<String> = project.world.scenes.iter()
            .map(|scene| scene.meta.name.clone()).collect();
        synchronize_custom_object_instances(
            &mut project.world.scenes,
            &project.object_types,
            &scene_names,
        );
        self.state.project = project;
        self.state.selected = None;
        self.scene_view.selected_object = None;
        self.scene_view.selected_tiles.clear();
        self.scene_view.selected_objects.clear();
        self.scene_view.clear_clipboard();
        self.state.undo_history.clear();
        self.state.redo_history.clear();
        self.project_path = Some(path.to_owned());
        if !self.player_mode {
            self.remember_recent_project(path);
        }
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

    fn load_object_scripts(&mut self) -> anyhow::Result<()> {
        let root = self.project_path.as_ref()
            .and_then(|path| path.parent())
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let scripts: Vec<(String, String)> = self.state.project.world.scenes.iter()
            .flat_map(|scene| scene.objects.values())
            .filter_map(|object| object.script.as_ref().filter(|path| !path.trim().is_empty())
                .map(|path| (object.id.to_string(), path.clone())))
            .collect();
        let mut errors = Vec::new();
        for (object_id, script_path) in scripts {
            let path = std::path::Path::new(&script_path);
            let resolved = if path.is_absolute() { path.to_owned() } else { root.join(path) };
            log::info!("Chargement du script objet {} : {}", object_id, resolved.display());
            if let Err(error) = self.state.script_runtime.load_object_script(&resolved, object_id) {
                self.state.push_log(log::Level::Error,
                    format!("Chargement du script d'objet `{script_path}` : {error}"));
                errors.push(format!("{script_path} : {error}"));
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            anyhow::bail!("Impossible de charger les scripts d'objet : {}", errors.join("; "))
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
const OBJECT_SCRIPT_TEMPLATE: &str = "from __future__ import annotations\nfrom typing import TYPE_CHECKING\n\nif TYPE_CHECKING:\n    from engine_api import GameContext, GameObject\n\n\ndef _on_game_launch(game: GameContext) -> None:\n    \"\"\"Called once when the game starts.\"\"\"\n    # Object id: game.object_id; tile position: game.x, game.y, game.z\n    pass\n\n\ndef _on_update(game: GameContext, delta_time: float) -> None:\n    \"\"\"Called on each game update.\"\"\"\n    # Use game.move_self(x, y), game.destroy_self(), or game.set_self_property(...).\n    pass\n";

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

fn project_tile_directory(project_file: &std::path::Path) -> anyhow::Result<std::path::PathBuf> {
    let parent = project_file.parent()
        .ok_or_else(|| anyhow::anyhow!("Dossier du projet introuvable"))?;
    let stem = project_file.file_stem()
        .ok_or_else(|| anyhow::anyhow!("Nom de projet invalide"))?;
    Ok(parent.join(stem).join("tiles"))
}

fn write_tile_definition_files(
    project_file: &std::path::Path,
    definitions: &[world::grid::TileDefinition],
) -> anyhow::Result<()> {
    let directory = project_tile_directory(project_file)?;
    std::fs::create_dir_all(&directory)?;
    let active_ids: HashSet<u16> = definitions.iter().map(|definition| definition.id).collect();
    for entry in std::fs::read_dir(&directory)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() || !path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("json")) {
            continue;
        }
        let Some(file_stem) = path.file_stem().and_then(|stem| stem.to_str()) else { continue };
        let Some(id) = file_stem.strip_prefix("tile-").and_then(|id| id.parse::<u16>().ok()) else { continue };
        if !active_ids.contains(&id) {
            std::fs::remove_file(path)?;
        }
    }
    for definition in definitions {
        let path = directory.join(format!("tile-{:05}.json", definition.id));
        std::fs::write(path, serde_json::to_vec_pretty(definition)?)?;
    }
    Ok(())
}

fn remove_tile_definition_from_scene(scene: &mut Scene, tile_id: u16) -> Vec<TilePos> {
    let chunk_size = world::grid::CHUNK_SIZE;
    let positions = scene.grid.chunks.iter()
        .flat_map(|(&(chunk_x, chunk_y), chunk)| {
            chunk.tiles.iter().enumerate().filter_map(move |(index, data)| {
                (data.tile_type == tile_id || data.decoration == tile_id).then(|| {
                    TilePos::new(
                        chunk_x * chunk_size + index as i32 % chunk_size,
                        chunk_y * chunk_size + index as i32 / chunk_size,
                    )
                })
            })
        })
        .collect::<Vec<_>>();

    for &position in &positions {
        let mut data = scene.get_tile(position);
        if data.tile_type == tile_id { data.tile_type = 0; }
        if data.decoration == tile_id { data.decoration = 0; }
        if data.is_empty() { scene.clear_tile(position); }
        else { scene.set_tile(position, data); }
    }
    positions
}

fn load_tile_definition_files(
    project_file: &std::path::Path,
    project: &mut Project,
) -> anyhow::Result<()> {
    let directory = project_tile_directory(project_file)?;
    if !directory.is_dir() {
        return Ok(());
    }
    let mut paths = std::fs::read_dir(directory)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("json")))
        .collect::<Vec<_>>();
    paths.sort();
    for path in paths {
        let definition: world::grid::TileDefinition = serde_json::from_slice(&std::fs::read(&path)?)
            .map_err(|error| anyhow::anyhow!("{} : {error}", path.display()))?;
        anyhow::ensure!(definition.id != 0, "L'identifiant 0 est réservé à la tuile vide");
        if let Some(existing) = project.tile_definitions.iter_mut().find(|tile| tile.id == definition.id) {
            *existing = definition;
        } else {
            project.tile_definitions.push(definition);
        }
    }
    project.tile_definitions.sort_by_key(|tile| tile.id);
    Ok(())
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

#[cfg(target_os = "windows")]
fn choose_tile_texture_file() -> anyhow::Result<Option<std::path::PathBuf>> {
    let script = r#"
Add-Type -AssemblyName System.Windows.Forms
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$dialog = New-Object System.Windows.Forms.OpenFileDialog
$dialog.Title = 'Choisir une texture de tuile'
$dialog.Filter = 'Images PNG, JPEG ou GIF (*.png;*.jpg;*.jpeg;*.gif)|*.png;*.jpg;*.jpeg;*.gif'
$dialog.Multiselect = $false
$dialog.CheckFileExists = $true
$dialog.RestoreDirectory = $true
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
    [Console]::Out.Write($dialog.FileName)
}
"#;
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-STA", "-WindowStyle", "Hidden", "-Command", script])
        .output()?;
    if !output.status.success() {
        anyhow::bail!("Le sélecteur de texture Windows a échoué : {}",
            String::from_utf8_lossy(&output.stderr).trim());
    }
    let selected = String::from_utf8(output.stdout)?;
    let selected = selected.trim();
    if selected.is_empty() { Ok(None) } else { Ok(Some(std::path::PathBuf::from(selected))) }
}

#[cfg(not(target_os = "windows"))]
fn choose_tile_texture_file() -> anyhow::Result<Option<std::path::PathBuf>> {
    anyhow::bail!("Le sélecteur natif de textures est uniquement disponible sous Windows")
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

#[cfg(target_os = "windows")]
fn choose_export_file(default_name: &str) -> anyhow::Result<Option<std::path::PathBuf>> {
    let script = r#"
Add-Type -AssemblyName System.Windows.Forms
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$dialog = New-Object System.Windows.Forms.SaveFileDialog
$dialog.Title = 'Exporter le jeu'
$dialog.Filter = 'Archive ZIP (*.zip)|*.zip'
$dialog.DefaultExt = 'zip'
$dialog.AddExtension = $true
$dialog.OverwritePrompt = $true
$dialog.FileName = '{default_name}'
$dialog.RestoreDirectory = $true
if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
    [Console]::Out.Write($dialog.FileName)
}
"#.replace("{default_name}", &default_name.replace('\'', "''"));
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-STA", "-WindowStyle", "Hidden", "-Command", &script])
        .output()?;
    if !output.status.success() {
        anyhow::bail!("Le sélecteur d'export Windows a échoué : {}",
            String::from_utf8_lossy(&output.stderr).trim());
    }
    let selected = String::from_utf8(output.stdout)?;
    let selected = selected.trim();
    if selected.is_empty() { Ok(None) } else { Ok(Some(std::path::PathBuf::from(selected))) }
}

#[cfg(not(target_os = "windows"))]
fn choose_export_file(_default_name: &str) -> anyhow::Result<Option<std::path::PathBuf>> {
    anyhow::bail!("Le sélecteur d'export Windows est uniquement disponible sous Windows")
}

impl Default for EditorApp {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod collision_and_tile_file_tests {
    use super::*;
    use world::{
        coords::TilePos,
        grid::{TileData, TileDefinition},
        object::{ObjectType, TileSize},
    };

    #[test]
    fn collidable_movement_is_blocked_by_solid_tile() {
        let mut scene = Scene::new("Tile collision");
        let id = scene.spawn("Player", ObjectType::Character, TilePos::new(0, 0));
        scene.get_object_mut(id).unwrap().has_collisions = true;
        scene.set_tile(TilePos::new(1, 0), TileData::terrain(7));
        let mut definition = TileDefinition::new(7, "Wall", [100, 100, 100, 255]);
        definition.has_collisions = true;

        let moved = move_object_with_collisions(&mut scene, &[definition], id, TilePos::new(1, 0)).unwrap();
        assert!(!moved);
        assert_eq!(scene.get_object(id).unwrap().tile_pos, TilePos::new(0, 0));
    }

    #[test]
    fn collidable_movement_respects_object_footprints_and_elevation() {
        let mut scene = Scene::new("Object collision");
        let player_id = scene.spawn("Player", ObjectType::Character, TilePos::new(0, 0));
        let player = scene.get_object_mut(player_id).unwrap();
        player.has_collisions = true;
        player.footprint = TileSize::new(2, 1);
        let obstacle_id = scene.spawn("Crate", ObjectType::Building, TilePos::new(3, 0));
        scene.get_object_mut(obstacle_id).unwrap().has_collisions = true;

        assert!(!move_object_with_collisions(
            &mut scene, &[], player_id, TilePos::new(2, 0),
        ).unwrap());
        assert!(move_object_with_collisions(
            &mut scene, &[], player_id, TilePos::with_z(2, 0, 1),
        ).unwrap());
        assert_eq!(scene.get_object(player_id).unwrap().tile_pos, TilePos::with_z(2, 0, 1));
    }

    #[test]
    fn tile_definition_files_are_project_scoped_and_loaded_over_embedded_defaults() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("isomaker_tiles_{}_{}", std::process::id(), nonce));
        std::fs::create_dir_all(&root).unwrap();
        let project_file = root.join("sample.isomaker");
        let mut source = Project::new("Sample");
        let mut definition = TileDefinition::new(7, "Animated wall", [10, 20, 30, 255]);
        definition.texture = Some("sample/assets/wall.gif".to_owned());
        definition.has_collisions = true;
        definition.randomize_texture_rotation = true;
        source.tile_definitions.push(definition.clone());

        write_tile_definition_files(&project_file, &source.tile_definitions).unwrap();
        let mut loaded = Project::new("Sample");
        load_tile_definition_files(&project_file, &mut loaded).unwrap();
        assert_eq!(loaded.tile_definitions.iter().find(|tile| tile.id == 7), Some(&definition));
        assert!(project_tile_directory(&project_file).unwrap().join("tile-00007.json").exists());

        std::fs::remove_dir_all(root).unwrap();
    }
}
