//! Barre d'outils supérieure de l'éditeur.

use egui::{Color32, Pos2, Rect, Sense, Shape, Stroke, Ui, Vec2, Widget};
use engine::game_loop::{EngineMode, GameLoop};
use std::path::PathBuf;

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
            EditorTool::Select    => "Sélection",
            EditorTool::Move      => "Déplacer",
            EditorTool::Rotate    => "Rotation",
            EditorTool::TilePaint => "Peindre",
            EditorTool::TileErase => "Gomme",
            EditorTool::TileFill  => "Remplir",
            EditorTool::Build     => "Construire",
        }
    }
}

struct ToolButton {
    tool: EditorTool,
    selected: bool,
}

impl Widget for ToolButton {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let font_id = egui::TextStyle::Button.resolve(ui.style());
        let text_color = ui.visuals().text_color();
        let galley = ui
            .painter()
            .layout_no_wrap(self.tool.label().to_owned(), font_id, text_color);
        let icon_size = 16.0;
        let padding = Vec2::new(7.0, 3.0);
        let size = Vec2::new(
            padding.x * 2.0 + icon_size + 6.0 + galley.size().x,
            (icon_size + padding.y * 2.0).max(ui.spacing().interact_size.y),
        );
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        let visuals = ui.style().interact_selectable(&response, self.selected);

        ui.painter().rect(
            rect,
            visuals.rounding,
            visuals.bg_fill,
            visuals.bg_stroke,
        );

        let icon_rect = Rect::from_min_size(
            Pos2::new(rect.left() + padding.x, rect.center().y - icon_size * 0.5),
            Vec2::splat(icon_size),
        );
        draw_tool_icon(ui.painter(), icon_rect, self.tool, visuals.fg_stroke.color);

        let text_pos = Pos2::new(
            icon_rect.right() + 6.0,
            rect.center().y - galley.size().y * 0.5,
        );
        ui.painter().galley(text_pos, galley, visuals.text_color());
        response
    }
}

fn draw_tool_icon(painter: &egui::Painter, rect: Rect, tool: EditorTool, color: Color32) {
    let point = |x: f32, y: f32| {
        Pos2::new(
            rect.left() + rect.width() * x,
            rect.top() + rect.height() * y,
        )
    };
    let stroke = Stroke::new(1.7_f32, color);
    let line = |a, b| painter.line_segment([a, b], stroke);
    let arrow_head = |tip: Pos2, direction: Vec2| {
        let direction = direction.normalized() * 4.0;
        let side = Vec2::new(-direction.y, direction.x) * 0.65;
        painter.add(Shape::convex_polygon(
            vec![tip, tip - direction + side, tip - direction - side],
            color,
            Stroke::NONE,
        ));
    };

    match tool {
        EditorTool::Select => {
            let cursor = vec![
                point(0.16, 0.07),
                point(0.27, 0.78),
                point(0.43, 0.59),
                point(0.61, 0.91),
                point(0.75, 0.83),
                point(0.57, 0.51),
                point(0.86, 0.49),
            ];
            for index in 0..cursor.len() {
                line(cursor[index], cursor[(index + 1) % cursor.len()]);
            }
        }
        EditorTool::Move => {
            let center = rect.center();
            line(point(0.5, 0.25), point(0.5, 0.75));
            line(point(0.25, 0.5), point(0.75, 0.5));
            arrow_head(point(0.5, 0.08), Vec2::new(0.0, -1.0));
            arrow_head(point(0.5, 0.92), Vec2::new(0.0, 1.0));
            arrow_head(point(0.08, 0.5), Vec2::new(-1.0, 0.0));
            arrow_head(point(0.92, 0.5), Vec2::new(1.0, 0.0));
            painter.circle_filled(center, 1.7, color);
        }
        EditorTool::Rotate => {
            let center = rect.center();
            let radius = rect.width() * 0.31;
            painter.circle_stroke(center, radius, stroke);
            let tip = point(0.78, 0.27);
            arrow_head(tip, Vec2::new(0.8, -0.6));
        }
        EditorTool::TilePaint => {
            line(point(0.29, 0.72), point(0.69, 0.32));
            line(point(0.23, 0.78), point(0.31, 0.86));
            line(point(0.31, 0.86), point(0.43, 0.74));
            line(point(0.43, 0.74), point(0.35, 0.66));
            painter.add(Shape::convex_polygon(
                vec![point(0.62, 0.25), point(0.76, 0.39), point(0.84, 0.31), point(0.70, 0.17)],
                color,
                Stroke::NONE,
            ));
            line(point(0.15, 0.91), point(0.83, 0.91));
        }
        EditorTool::TileErase => {
            let eraser = vec![
                point(0.17, 0.63),
                point(0.58, 0.18),
                point(0.83, 0.40),
                point(0.42, 0.85),
            ];
            painter.add(Shape::convex_polygon(
                eraser,
                Color32::TRANSPARENT,
                stroke,
            ));
            line(point(0.36, 0.42), point(0.62, 0.65));
            line(point(0.22, 0.88), point(0.80, 0.88));
        }
        EditorTool::TileFill => {
            line(point(0.31, 0.19), point(0.59, 0.46));
            line(point(0.24, 0.27), point(0.52, 0.55));
            line(point(0.49, 0.47), point(0.73, 0.69));
            line(point(0.73, 0.69), point(0.64, 0.83));
            line(point(0.64, 0.83), point(0.28, 0.83));
            line(point(0.28, 0.83), point(0.20, 0.70));
            line(point(0.20, 0.70), point(0.49, 0.47));
            painter.circle_filled(point(0.77, 0.33), 1.5, color);
            painter.circle_filled(point(0.86, 0.48), 1.2, color);
        }
        EditorTool::Build => {
            line(point(0.35, 0.83), point(0.70, 0.48));
            line(point(0.28, 0.76), point(0.42, 0.90));
            line(point(0.18, 0.28), point(0.39, 0.10));
            line(point(0.39, 0.10), point(0.63, 0.35));
            line(point(0.63, 0.35), point(0.48, 0.50));
            line(point(0.48, 0.50), point(0.18, 0.28));
            line(point(0.61, 0.30), point(0.78, 0.14));
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
    pub export_requested: bool,
    pub scene_switch_requested: Option<usize>,
    pub new_scene_requested: bool,
    pub world_settings_requested: bool,
    pub editor_settings_requested: bool,
    pub recent_project_requested: Option<PathBuf>,
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
            export_requested: false,
            scene_switch_requested: None,
            new_scene_requested: false,
            world_settings_requested: false,
            editor_settings_requested: false,
            recent_project_requested: None,
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
        recent_projects: &[PathBuf],
        export_in_progress: bool,
    ) {
        self.undo_requested = false;
        self.redo_requested = false;
        self.new_requested = false;
        self.open_requested = false;
        self.save_requested = false;
        self.save_as_requested = false;
        self.export_requested = false;
        self.scene_switch_requested = None;
        self.new_scene_requested = false;
        self.world_settings_requested = false;
        self.editor_settings_requested = false;
        self.recent_project_requested = None;
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
                ui.menu_button("Projets récents", |ui| {
                    if recent_projects.is_empty() {
                        ui.label("Aucun projet récent");
                    } else {
                        for path in recent_projects {
                            let label = path.file_name().and_then(|name| name.to_str())
                                .unwrap_or_else(|| path.to_str().unwrap_or("Projet"));
                            if ui.button(label).on_hover_text(path.display().to_string()).clicked() {
                                self.recent_project_requested = Some(path.clone());
                                ui.close_menu();
                            }
                        }
                    }
                });
                if ui.button("Enregistrer").clicked() {
                    self.save_requested = true;
                    ui.close_menu();
                }
                if ui.button("Enregistrer sous…").clicked() {
                    self.save_as_requested = true;
                    ui.close_menu();
                }
                if ui.add_enabled(!export_in_progress, egui::Button::new(
                    if export_in_progress { "Export en cours…" } else { "Exporter le jeu…" },
                )).clicked() {
                    self.export_requested = true;
                    ui.close_menu();
                }
                });
                ui.separator();
                if ui.button("Paramètres de l'éditeur…").clicked() {
                    self.editor_settings_requested = true;
                    ui.close_menu();
                }
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
                if ui.add(ToolButton { tool, selected }).clicked() {
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
