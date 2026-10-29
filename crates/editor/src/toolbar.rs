//! Barre d'outils supérieure de l'éditeur.

use egui::Ui;
use engine::game_loop::{EngineMode, GameLoop};

/// Outil actif dans l'éditeur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorTool {
    #[default]
    Select,
    Move,
    Rotate,
    TilePaint,
    TileErase,
    TileFill,
    Build,
}

impl EditorTool {
    pub fn label(&self) -> &'static str {
        match self {
            EditorTool::Select    => "⬡ Sélection",
            EditorTool::Move      => "✥ Déplacer",
            EditorTool::Rotate    => "↻ Rotation",
            EditorTool::TilePaint => "🖌 Peindre",
            EditorTool::TileErase => "⌫ Gomme",
            EditorTool::TileFill  => "⬛ Remplir",
            EditorTool::Build     => "🏗 Construire",
        }
    }
}

/// État de la barre d'outils.
pub struct Toolbar {
    pub active_tool: EditorTool,
    pub undo_requested: bool,
    pub redo_requested: bool,
    pub new_requested: bool,
    pub open_requested: bool,
    pub save_requested: bool,
    pub save_as_requested: bool,
    pub scene_switch_requested: Option<usize>,
    pub new_scene_requested: bool,
    pub world_settings_requested: bool,
}

impl Default for Toolbar {
    fn default() -> Self {
        Self {
            active_tool: EditorTool::Select,
            undo_requested: false,
            redo_requested: false,
            new_requested: false,
            open_requested: false,
            save_requested: false,
            save_as_requested: false,
            scene_switch_requested: None,
            new_scene_requested: false,
            world_settings_requested: false,
        }
    }
}

impl Toolbar {
    pub fn new() -> Self { Self::default() }

    /// Dessine la barre d'outils et retourne la commande éventuelle (Play/Pause/Stop).
    pub fn show(
        &mut self,
        ui: &mut Ui,
        game_loop: &mut GameLoop,
        scene_names: &[String],
        active_scene: usize,
    ) {
        self.undo_requested = false;
        self.redo_requested = false;
        self.new_requested = false;
        self.open_requested = false;
        self.save_requested = false;
        self.save_as_requested = false;
        self.scene_switch_requested = None;
        self.new_scene_requested = false;
        self.world_settings_requested = false;
        let editor_enabled = game_loop.mode == EngineMode::Editor;
        egui::menu::bar(ui, |ui| {
            // ── Fichier ──────────────────────────────────────────────────────
            ui.menu_button("Fichier", |ui| {
                ui.add_enabled_ui(editor_enabled, |ui| {
                if ui.button("Nouveau projet").clicked() {
                    self.new_requested = true;
                    ui.close_menu();
                }
                if ui.button("Ouvrir…").clicked() {
                    self.open_requested = true;
                    ui.close_menu();
                }
                if ui.button("Enregistrer").clicked() {
                    self.save_requested = true;
                    ui.close_menu();
                }
                if ui.button("Enregistrer sous…").clicked() {
                    self.save_as_requested = true;
                    ui.close_menu();
                }
                });
                ui.separator();
                if ui.button("Quitter").clicked()        { std::process::exit(0); }
            });

            ui.add_enabled_ui(editor_enabled, |ui| ui.menu_button("Monde", |ui| {
                if ui.button("Script du monde…").clicked() {
                    self.world_settings_requested = true;
                    ui.close_menu();
                }
            }));

            ui.separator();

            // ── Outils ───────────────────────────────────────────────────────
            ui.add_enabled_ui(editor_enabled, |ui| {
            for tool in [
                EditorTool::Select,
                EditorTool::Move,
                EditorTool::Rotate,
                EditorTool::TilePaint,
                EditorTool::TileErase,
                EditorTool::TileFill,
                EditorTool::Build,
            ] {
                let selected = self.active_tool == tool;
                if ui.selectable_label(selected, tool.label()).clicked() {
                    self.active_tool = tool;
                }
            }
            });

            ui.add_enabled_ui(editor_enabled, |ui| ui.menu_button("Scènes", |ui| {
                for (index, name) in scene_names.iter().enumerate() {
                    if ui.selectable_label(index == active_scene, name).clicked() {
                        self.scene_switch_requested = Some(index);
                        ui.close_menu();
                    }
                }
                ui.separator();
                if ui.button("+ Nouvelle scène").clicked() {
                    self.new_scene_requested = true;
                    ui.close_menu();
                }
            }));

            ui.separator();

            // ── Annuler / Rétablir ────────────────────────────────────────────
            if ui.add_enabled(editor_enabled, egui::Button::new("↩ Annuler")).clicked()   { self.undo_requested = true; }
            if ui.add_enabled(editor_enabled, egui::Button::new("↪ Rétablir")).clicked()  { self.redo_requested = true; }

            ui.separator();

            // ── Contrôles de simulation ───────────────────────────────────────
            if ui.input(|input| input.key_pressed(egui::Key::F9)) {
                game_loop.play();
            } else if ui.input(|input| input.key_pressed(egui::Key::F10)) {
                game_loop.stop();
            } else if ui.input(|input| input.key_pressed(egui::Key::F11)) {
                game_loop.pause();
            }

            let mode = game_loop.mode;
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Affichage du mode
                let mode_text = format!("  {}  ", mode);
                ui.label(egui::RichText::new(mode_text).monospace());

                ui.separator();

                // Bouton Stop
                if ui.add_enabled(
                    mode != EngineMode::Editor,
                    egui::Button::new("⏹ Stop"),
                ).clicked() {
                    game_loop.stop();
                }

                // Bouton Pause / Reprendre
                if mode == EngineMode::Playing {
                    if ui.button("⏸ Pause").clicked() {
                        game_loop.pause();
                    }
                } else if mode == EngineMode::Paused {
                    if ui.button("▶ Reprendre").clicked() {
                        game_loop.resume();
                    }
                }

                // Bouton Play
                if ui.add_enabled(
                    mode != EngineMode::Playing,
                    egui::Button::new("▶ Lecture"),
                ).clicked() {
                    game_loop.play();
                }
            });
        });
    }
}
