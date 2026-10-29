//! Vue centrale de la scène isométrique dessinée via egui Painter.

use egui::{Color32, Painter, Rect, Sense, Stroke, Ui, Vec2};
use image::AnimationDecoder;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::BufReader,
    path::Path,
};
use world::{
    coords::{IsoProjection, ScreenPos, TilePos},
    grid::{TileData, TileDefinition},
    object::{ObjectId, ObjectType, Rotation4, SceneObject},
    prefab::{BuildingModuleKind, Prefab},
    scene::Scene,
};
use crate::{
    camera::Camera,
    panels::BuildDragPayload,
    toolbar::EditorTool,
};

/// Options d'affichage de la grille.
pub struct GridOptions {
    pub show_grid: bool,
    pub grid_color: Color32,
    pub selection_color: Color32,
    pub hover_color: Color32,
}

impl Default for GridOptions {
    fn default() -> Self {
        Self {
            show_grid: true,
            grid_color: Color32::from_rgba_unmultiplied(180, 200, 220, 80),
            selection_color: Color32::from_rgb(100, 180, 255),
            hover_color: Color32::from_rgba_unmultiplied(255, 255, 255, 60),
        }
    }
}

/// État de la vue scène.
pub struct SceneView {
    pub camera: Camera,
    pub options: GridOptions,
    /// Tuile actuellement survolée.
    pub hovered_tile: Option<TilePos>,
    /// Objet sélectionné.
    pub selected_object: Option<ObjectId>,
    /// Objets inclus dans la sélection multiple.
    pub selected_objects: HashSet<ObjectId>,
    /// Tuiles incluses dans la sélection courante.
    pub selected_tiles: HashSet<TilePos>,
    delete_requested: Option<(Vec<ObjectId>, Vec<TilePos>)>,
    /// Données de tuile copiées dans l'éditeur.
    copied_tile: Option<TileData>,
    copy_tile_requested: bool,
    paste_tile_requested: bool,
    /// Paint tile type courant.
    pub paint_tile_type: u16,
    /// Dragging de caméra en cours.
    is_panning: bool,
    pan_start: Vec2,
    pan_cam_start: Vec2,
    selection_start: Option<TilePos>,
    selection_objects_only: bool,
    move_start: Option<TilePos>,
    move_tiles: Vec<(TilePos, TileData)>,
    move_objects: Vec<(ObjectId, TilePos)>,
    rotate_last_angle: Option<f32>,
    rotate_accumulated_angle: f32,
    rotate_pivot: Vec2,
    rotate_steps: i32,
    rotate_bounds: Option<(i32, i32, i32, i32)>,
    rotate_tiles: Vec<(TilePos, TileData)>,
    texture_cache: HashMap<String, Option<egui::TextureHandle>>,
    animated_tile_textures: HashMap<String, AnimatedTileTexture>,
    failed_animated_tile_textures: HashSet<String>,
    object_hit_rects: HashMap<ObjectId, Rect>,
}

struct AnimatedTileFrame {
    image: egui::ColorImage,
    duration_seconds: f32,
}

struct AnimatedTileTexture {
    frames: Vec<AnimatedTileFrame>,
    texture: egui::TextureHandle,
    current_frame: usize,
    elapsed_seconds: f32,
}

impl AnimatedTileTexture {
    fn advance(&mut self, delta_time: f32) {
        self.elapsed_seconds += delta_time.max(0.0);
        let mut changed = false;
        while self.elapsed_seconds >= self.frames[self.current_frame].duration_seconds {
            self.elapsed_seconds -= self.frames[self.current_frame].duration_seconds;
            self.current_frame = (self.current_frame + 1) % self.frames.len();
            changed = true;
        }
        if changed {
            self.texture.set(
                self.frames[self.current_frame].image.clone(),
                egui::TextureOptions::NEAREST,
            );
        }
    }
}

impl Default for SceneView {
    fn default() -> Self {
        Self {
            camera: Camera::default(),
            options: GridOptions::default(),
            hovered_tile: None,
            selected_object: None,
            selected_objects: HashSet::new(),
            selected_tiles: HashSet::new(),
            delete_requested: None,
            copied_tile: None,
            copy_tile_requested: false,
            paste_tile_requested: false,
            paint_tile_type: 1,
            is_panning: false,
            pan_start: Vec2::ZERO,
            pan_cam_start: Vec2::ZERO,
            selection_start: None,
            selection_objects_only: false,
            move_start: None,
            move_tiles: Vec::new(),
            move_objects: Vec::new(),
            rotate_last_angle: None,
            rotate_accumulated_angle: 0.0,
            rotate_pivot: Vec2::ZERO,
            rotate_steps: 0,
            rotate_bounds: None,
            rotate_tiles: Vec::new(),
            texture_cache: HashMap::new(),
            animated_tile_textures: HashMap::new(),
            failed_animated_tile_textures: HashSet::new(),
            object_hit_rects: HashMap::new(),
        }
    }
}

impl SceneView {
    pub fn new() -> Self { Self::default() }

    pub fn take_delete_request(&mut self) -> Option<(Vec<ObjectId>, Vec<TilePos>)> {
        self.delete_requested.take()
    }

    pub fn request_tile_clipboard_shortcut(&mut self, copy: bool) {
        if copy { self.copy_tile_requested = true; }
        else { self.paste_tile_requested = true; }
    }

    /// Affiche la vue scène dans le `Ui` donné.
    ///
    /// Retourne `true` si la scène a été modifiée (tuile peinte, objet déplacé…).
    pub fn show(
        &mut self,
        ui: &mut Ui,
        scene: &mut Scene,
        tile_definitions: &[TileDefinition],
        active_tool: EditorTool,
        asset_root: &Path,
        delta_time: f32,
        fps: Option<f32>,
        interactive: bool,
        show_grid: bool,
        runtime_camera: &mut Option<ObjectId>,
    ) -> bool {
        let mut modified = false;
        self.update_animated_tile_textures(ui.ctx(), asset_root, tile_definitions, delta_time);
        let proj = scene.projection();
        let active_tool = if interactive { active_tool } else { EditorTool::Select };

        // Zone disponible pour la scène.
        let (response, painter) = ui.allocate_painter(
            ui.available_size(),
            if interactive { Sense::click_and_drag() } else { Sense::hover() },
        );
        let rect = response.rect;
        let center = rect.center().to_vec2();

        if interactive && ui.input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::D)) {
            self.selected_tiles.clear();
            self.selected_object = None;
            self.selected_objects.clear();
        }
        if interactive && !ui.ctx().wants_keyboard_input()
            && ui.input(|input| input.key_pressed(egui::Key::Delete))
        {
            if !self.selected_objects.is_empty() {
                self.delete_requested = Some((self.selected_objects.iter().copied().collect(), Vec::new()));
            } else if let Some(id) = self.selected_object {
                self.delete_requested = Some((vec![id], Vec::new()));
            } else if !self.selected_tiles.is_empty() {
                let tiles: Vec<_> = self.selected_tiles.iter().copied().collect();
                let selected_xy = &tiles;
                let object_ids: Vec<_> = scene.objects.values()
                    .filter(|object| selected_xy.iter().any(|tile| {
                        tile.x == object.tile_pos.x && tile.y == object.tile_pos.y
                    }))
                    .map(|object| object.id)
                    .collect();
                if object_ids.is_empty() {
                    for tile in &tiles {
                        if !scene.get_tile(*tile).is_empty() {
                            scene.clear_tile(*tile);
                            modified = true;
                        }
                    }
                } else {
                    self.delete_requested = Some((object_ids, tiles));
                }
            }
        }

        // ── Gestion des entrées ───────────────────────────────────────────────

        // Scroll = zoom.
        if let Some(hover_pos) = response.hover_pos() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.camera.zoom_at(hover_pos.to_vec2() - center, scroll);
            }
        }

        // Clic milieu ou Alt+clic gauche = pan.
        if response.drag_started_by(egui::PointerButton::Middle)
            || (response.drag_started_by(egui::PointerButton::Primary)
                && ui.input(|i| i.modifiers.alt))
        {
            self.is_panning = true;
            self.pan_start = response.interact_pointer_pos()
                .map(|p| p.to_vec2())
                .unwrap_or_default();
            self.pan_cam_start = self.camera.position;
        }

        if self.is_panning {
            if let Some(pos) = response.interact_pointer_pos() {
                let delta = self.pan_start - pos.to_vec2();
                self.camera.position = self.pan_cam_start + delta / self.camera.zoom;
            }
            if !response.dragged() {
                self.is_panning = false;
            }
        }

        // Tuile survolée.
        self.hovered_tile = response.hover_pos().or_else(|| response.interact_pointer_pos()).map(|p| {
            let world = self.camera.screen_to_world(p.to_vec2() - center, Vec2::ZERO);
            proj.screen_to_tile(ScreenPos::new(world.x, world.y))
        });

        // Copier/coller une tuile avec le focus dans le viewport et l'outil Sélection.
        if interactive && active_tool == EditorTool::Select {
            let copy = std::mem::take(&mut self.copy_tile_requested);
            let paste = std::mem::take(&mut self.paste_tile_requested);
            if copy && self.selected_tiles.len() == 1 {
                if let Some(tile) = self.selected_tiles.iter().next().copied() {
                    self.copied_tile = Some(scene.get_tile(tile));
                }
            }
            if paste {
                let destination = (self.selected_tiles.len() == 1)
                    .then(|| self.selected_tiles.iter().next().copied())
                    .flatten()
                    .or(self.hovered_tile);
                if let (Some(tile), Some(data)) = (destination, self.copied_tile) {
                    if scene.get_tile(tile) != data {
                        if data.is_empty() { scene.clear_tile(tile); }
                        else { scene.set_tile(tile, data); }
                        modified = true;
                    }
                    self.selected_tiles.clear();
                    self.selected_tiles.insert(tile);
                    self.selected_object = None;
                    self.selected_objects.clear();
                }
            }
        }

        let dropped_build_payload = if interactive {
            response.dnd_release_payload::<BuildDragPayload>()
        } else {
            None
        };
        if let (Some(payload), Some(tile)) = (&dropped_build_payload, self.hovered_tile) {
            match payload.as_ref() {
                BuildDragPayload::Prefab(prefab) => {
                    let ids = scene.instantiate_prefab(prefab, tile);
                    self.selected_object = ids.first().copied();
                    modified |= !ids.is_empty();
                }
                BuildDragPayload::Module(kind) => {
                    let object = module_object(*kind, tile);
                    self.selected_object = Some(scene.add_object(object));
                    modified = true;
                }
            }
            self.selected_tiles.clear();
        }
        let build_drag_hover = if interactive {
            response.dnd_hover_payload::<BuildDragPayload>()
        } else {
            None
        };

        // Sélection par clic ou rectangle de tuiles.
        if active_tool == EditorTool::Select && !self.is_panning {
            if response.drag_started_by(egui::PointerButton::Primary) {
                self.selection_start = self.hovered_tile;
                self.selection_objects_only = ui.input(|input| input.modifiers.shift);
                self.selected_object = None;
                self.selected_tiles.clear();
                if !self.selection_objects_only {
                    self.selected_objects.clear();
                }
            }
            if response.dragged_by(egui::PointerButton::Primary) {
                if let (Some(start), Some(end)) = (self.selection_start, self.hovered_tile) {
                    if self.selection_objects_only {
                        self.selected_tiles.clear();
                        self.selected_objects = objects_in_tile_rectangle(scene, start, end);
                        self.selected_object = sole_object(&self.selected_objects);
                    } else {
                        self.selected_tiles = tile_rectangle(start, end);
                    }
                }
            }
            if response.clicked_by(egui::PointerButton::Primary) {
                let hit_object = response.interact_pointer_pos().and_then(|pointer| {
                    self.find_object_at_screen(scene, pointer, &proj, center)
                });
                if ui.input(|input| input.modifiers.shift) {
                    self.selected_tiles.clear();
                    if let Some(id) = hit_object {
                        if !self.selected_objects.insert(id) {
                            self.selected_objects.remove(&id);
                        }
                    }
                    self.selected_object = sole_object(&self.selected_objects);
                } else {
                    self.selected_tiles.clear();
                    self.selected_objects.clear();
                    if let Some(tile) = self.hovered_tile {
                        self.selected_tiles.insert(tile);
                    }
                    self.selected_object = hit_object;
                    if let Some(id) = hit_object {
                        self.selected_objects.insert(id);
                    }
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                if let (Some(start), Some(end)) = (self.selection_start, self.hovered_tile) {
                    if self.selection_objects_only {
                        self.selected_tiles.clear();
                        self.selected_objects = objects_in_tile_rectangle(scene, start, end);
                        self.selected_object = sole_object(&self.selected_objects);
                    } else {
                        self.selected_tiles = tile_rectangle(start, end);
                        self.selected_objects.clear();
                    }
                }
                self.selection_start = None;
                self.selection_objects_only = false;
            }
        }

        // Déplacement : la sélection entière, ou la tuile sous le curseur.
        if active_tool == EditorTool::Move && !self.is_panning {
            if response.drag_started_by(egui::PointerButton::Primary) {
                self.move_start = self.hovered_tile;
                self.move_tiles.clear();
                self.move_objects.clear();
                let hit_object = response.interact_pointer_pos().and_then(|pointer| {
                    self.find_object_at_screen(scene, pointer, &proj, center)
                });
                if let Some(id) = hit_object {
                    if !self.selected_objects.contains(&id) {
                        self.selected_objects.clear();
                        self.selected_objects.insert(id);
                    }
                    self.selected_object = sole_object(&self.selected_objects);
                    self.selected_tiles.clear();
                    self.move_objects.extend(self.selected_objects.iter().filter_map(|id| {
                        scene.get_object(*id).map(|object| (*id, object.tile_pos))
                    }));
                } else if !self.selected_tiles.is_empty() {
                    self.move_tiles.extend(self.selected_tiles.iter().map(|&tile| (tile, scene.get_tile(tile))));
                } else if let Some(tile) = self.hovered_tile {
                    self.move_tiles.push((tile, scene.get_tile(tile)));
                }
            }
            if response.drag_stopped_by(egui::PointerButton::Primary) {
                if let (Some(start), Some(end)) = (self.move_start, self.hovered_tile) {
                    let dx = end.x - start.x;
                    let dy = end.y - start.y;
                    if dx != 0 || dy != 0 {
                        for &(id, original_pos) in &self.move_objects {
                            if let Some(object) = scene.get_object_mut(id) {
                                object.tile_pos = TilePos::with_z(
                                    original_pos.x + dx,
                                    original_pos.y + dy,
                                    original_pos.z,
                                );
                                modified = true;
                            }
                        }
                        for &(tile, data) in &self.move_tiles {
                            if !data.is_empty() {
                                scene.clear_tile(tile);
                            }
                        }
                        for &(tile, data) in &self.move_tiles {
                            if !data.is_empty() {
                                scene.set_tile(TilePos::with_z(tile.x + dx, tile.y + dy, tile.z), data);
                            }
                        }
                        if !self.selected_tiles.is_empty() {
                            self.selected_tiles = self.selected_tiles.iter()
                                .map(|tile| TilePos::with_z(tile.x + dx, tile.y + dy, tile.z))
                                .collect();
                        }
                        modified |= self.move_tiles.iter().any(|(_, data)| !data.is_empty());
                    }
                }
                self.move_start = None;
                self.move_tiles.clear();
                self.move_objects.clear();
            }
        }

        // Rotation temporaire de la sélection autour de son centre.
        if active_tool == EditorTool::Rotate && !self.is_panning {
            if response.drag_started_by(egui::PointerButton::Primary)
                && !self.selected_tiles.is_empty()
            {
                let min_x = self.selected_tiles.iter().map(|tile| tile.x).min().unwrap_or(0);
                let max_x = self.selected_tiles.iter().map(|tile| tile.x).max().unwrap_or(0);
                let min_y = self.selected_tiles.iter().map(|tile| tile.y).min().unwrap_or(0);
                let max_y = self.selected_tiles.iter().map(|tile| tile.y).max().unwrap_or(0);
                let logical_center = Vec2::new(
                    (min_x + max_x) as f32 * 0.5,
                    (min_y + max_y) as f32 * 0.5,
                );
                let world_center = Vec2::new(
                    (logical_center.x - logical_center.y) * proj.half_width(),
                    (logical_center.x + logical_center.y) * proj.half_height(),
                );
                self.rotate_pivot = self.camera.world_to_screen(world_center, Vec2::ZERO) + center;
                self.rotate_last_angle = response.interact_pointer_pos().map(|pointer| {
                    let delta = pointer.to_vec2() - self.rotate_pivot;
                    delta.y.atan2(delta.x)
                });
                self.rotate_accumulated_angle = 0.0;
                self.rotate_steps = 0;
                self.rotate_bounds = Some((min_x, min_y, max_x, max_y));
                self.rotate_tiles = self.selected_tiles.iter()
                    .map(|&tile| (tile, scene.get_tile(tile)))
                    .collect();
            }

            if response.dragged_by(egui::PointerButton::Primary)
                || response.drag_stopped_by(egui::PointerButton::Primary)
            {
                if let (Some(last_angle), Some(pointer)) =
                    (self.rotate_last_angle, response.interact_pointer_pos())
                {
                    let delta = pointer.to_vec2() - self.rotate_pivot;
                    let angle = delta.y.atan2(delta.x);
                    let mut difference = angle - last_angle;
                    while difference > std::f32::consts::PI { difference -= std::f32::consts::TAU; }
                    while difference < -std::f32::consts::PI { difference += std::f32::consts::TAU; }
                    self.rotate_accumulated_angle += difference;
                    self.rotate_last_angle = Some(angle);
                    self.rotate_steps = (self.rotate_accumulated_angle
                        / std::f32::consts::FRAC_PI_2).round() as i32;
                }
            }

            if response.drag_stopped_by(egui::PointerButton::Primary)
                && self.rotate_last_angle.is_some()
            {
                let steps = self.rotate_steps.rem_euclid(4);
                if steps != 0 {
                    if let Some(bounds) = self.rotate_bounds {
                        for &(tile, data) in &self.rotate_tiles {
                            if !data.is_empty() { scene.clear_tile(tile); }
                        }
                        for &(tile, data) in &self.rotate_tiles {
                            let rotated = rotate_tile_in_bounds(tile, bounds, steps);
                            if !data.is_empty() { scene.set_tile(rotated, data); }
                        }
                        self.selected_tiles = self.rotate_tiles.iter()
                            .map(|(tile, _)| rotate_tile_in_bounds(*tile, bounds, steps))
                            .collect();

                        let object_ids: Vec<_> = scene.objects.values()
                            .filter(|object| self.rotate_tiles.iter().any(|(tile, _)| *tile == object.tile_pos))
                            .map(|object| object.id)
                            .collect();
                        for &id in &object_ids {
                            if let Some(object) = scene.get_object_mut(id) {
                                object.tile_pos = rotate_tile_in_bounds(object.tile_pos, bounds, steps);
                                for _ in 0..steps { object.rotation = object.rotation.next(); }
                            }
                        }
                        modified = self.rotate_tiles.iter().any(|(_, data)| !data.is_empty())
                            || !object_ids.is_empty();
                    }
                }
                self.rotate_last_angle = None;
                self.rotate_accumulated_angle = 0.0;
                self.rotate_bounds = None;
                self.rotate_tiles.clear();
                self.rotate_steps = 0;
            }
        }

        // ── Dessin du fond ────────────────────────────────────────────────────
        painter.rect_filled(rect, 0.0, Color32::from_rgb(30, 35, 40));

        // ── Dessin de la grille et des tuiles ─────────────────────────────────
        self.draw_grid(ui, scene, tile_definitions, asset_root, &painter, &proj, center, rect, show_grid);

        // Les données réelles restent en place jusqu'au relâchement du clic.
        if response.dragged_by(egui::PointerButton::Primary)
            && active_tool == EditorTool::Move
        {
            self.draw_move_preview(scene, &painter, &proj, center);
        }
        if response.dragged_by(egui::PointerButton::Primary)
            && active_tool == EditorTool::Rotate
        {
            self.draw_rotate_preview(&painter, &proj, center);
        }
        if active_tool == EditorTool::Select && self.selection_objects_only {
            self.draw_object_selection_bounds(&painter, &proj, center);
        }
        if let Some(payload) = build_drag_hover {
            if let Some(tile) = self.hovered_tile {
                match payload.as_ref() {
                    BuildDragPayload::Prefab(prefab) => {
                        self.draw_build_preview(prefab, tile, &painter, &proj, center);
                    }
                    BuildDragPayload::Module(kind) => {
                        self.draw_module_preview(*kind, tile, &painter, &proj, center);
                    }
                }
            }
        } else if active_tool == EditorTool::Build {
            if let Some(tile) = self.hovered_tile {
                self.draw_build_preview(&Prefab::preset_house(), tile, &painter, &proj, center);
            }
        }

        // ── Dessin des objets ─────────────────────────────────────────────────
        self.draw_objects(ui.ctx(), scene, &painter, &proj, center, rect, asset_root, interactive);

        // ── Interactions (paint, sélection) ──────────────────────────────────
        if response.clicked_by(egui::PointerButton::Primary)
            && !self.is_panning
            && dropped_build_payload.is_none()
        {
            if let Some(tile) = self.hovered_tile {
                match active_tool {
                    EditorTool::TilePaint => {
                        scene.set_tile(tile, TileData::terrain(self.paint_tile_type));
                        modified = true;
                    }
                    EditorTool::TileErase => {
                        modified |= erase_tile(scene, tile);
                    }
                    EditorTool::TileFill => {
                        if self.selected_tiles.is_empty() {
                            let region = connected_tile_region(scene, tile);
                            modified |= fill_tiles(scene, region, self.paint_tile_type);
                        } else {
                            modified |= fill_tiles(
                                scene,
                                self.selected_tiles.iter().copied(),
                                self.paint_tile_type,
                            );
                        }
                    }
                    EditorTool::Rotate => {
                        if self.selected_tiles.is_empty() {
                            let target = self.selected_object
                                .filter(|&id| scene.get_object(id).is_some())
                                .or_else(|| self.find_object_at(scene, tile));
                            if let Some(object) = target.and_then(|id| scene.get_object_mut(id)) {
                                object.rotation = object.rotation.next();
                                modified = true;
                            }
                        }
                    }
                    EditorTool::Build => {
                        let ids = scene.instantiate_prefab(&Prefab::preset_house(), tile);
                        self.selected_object = ids.first().copied();
                        modified = !ids.is_empty();
                    }
                    _ => {}
                }
            }
        }

        // Peindre et gommer en continu pendant le glisser.
        if response.dragged_by(egui::PointerButton::Primary)
            && matches!(active_tool, EditorTool::TilePaint | EditorTool::TileErase)
        {
            if let Some(tile) = self.hovered_tile {
                match active_tool {
                    EditorTool::TilePaint => {
                        let data = TileData::terrain(self.paint_tile_type);
                        if scene.get_tile(tile) != data {
                            scene.set_tile(tile, data);
                            modified = true;
                        }
                    }
                    EditorTool::TileErase => modified |= erase_tile(scene, tile),
                    _ => unreachable!(),
                }
            }
        }

        if let Some(fps) = fps {
            let label = format!("FPS: {:.0}", fps);
            let font = egui::FontId::proportional(13.0);
            let galley = painter.layout_no_wrap(label.clone(), font.clone(), Color32::WHITE);
            let padding = Vec2::new(8.0, 5.0);
            let size = galley.size() + padding * 2.0;
            let badge = Rect::from_min_size(
                egui::pos2(rect.right() - size.x - 12.0, rect.top() + 12.0),
                size,
            );
            painter.rect_filled(badge, 4.0, Color32::from_black_alpha(190));
            painter.text(
                badge.min + padding,
                egui::Align2::LEFT_TOP,
                label,
                font,
                Color32::WHITE,
            );
        }

        if !interactive {
            show_runtime_camera_selector(ui.ctx(), scene, rect, runtime_camera);
        }

        modified
    }

    /// Dessine la grille isométrique et les tuiles peintes.
    fn draw_grid(
        &mut self,
        ui: &Ui,
        scene: &Scene,
        tile_definitions: &[TileDefinition],
        asset_root: &Path,
        painter: &Painter,
        proj: &IsoProjection,
        center: Vec2,
        rect: Rect,
        show_grid: bool,
    ) {
        let zoom = self.camera.zoom;
        let cam_tile = proj.screen_to_tile(
            world::coords::ScreenPos::new(self.camera.position.x, self.camera.position.y),
        );
        let tile_range = (rect.width().hypot(rect.height())
            / (proj.tile_height * zoom).max(1.0))
            .ceil() as i32;
        let tile_range = tile_range.saturating_add(4).clamp(4, 2048);
        // Once an individual tile is only a few pixels wide, render a coarser
        // lattice and chunk summaries instead of thousands of tiny diamonds.
        let detailed = zoom >= 0.85;
        let step = if detailed { 1 } else { (2.0 / zoom).ceil() as i32 };

        if !detailed {
            self.draw_low_zoom_tiles(scene, tile_definitions, painter, proj, center, rect, cam_tile, tile_range);
        }

        // Anchor coarse grid vertices to world coordinates, not to the camera
        // or the current zoom, so the lattice never slides over painted tiles.
        let min_x = (cam_tile.x - tile_range).div_euclid(step) * step;
        let max_x = (cam_tile.x + tile_range).div_euclid(step) * step;
        let min_y = (cam_tile.y - tile_range).div_euclid(step) * step;
        let max_y = (cam_tile.y + tile_range).div_euclid(step) * step;
        let half_width = proj.half_width() * zoom * step as f32;
        let half_height = proj.half_height() * zoom * step as f32;

        for y in (min_y..=max_y).step_by(step as usize) {
            for x in (min_x..=max_x).step_by(step as usize) {
                let tile = TilePos::new(x, y);
                let screen = self.tile_to_screen(tile, proj, center);

                // Calcule les 4 coins du losange.
                let corners = [
                    egui::pos2(screen.x, screen.y - half_height),
                    egui::pos2(screen.x + half_width, screen.y),
                    egui::pos2(screen.x, screen.y + half_height),
                    egui::pos2(screen.x - half_width, screen.y),
                ];

                // Ignore si hors de la vue.
                let min_x = corners.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
                let max_x = corners.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
                let min_y = corners.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
                let max_y = corners.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
                if max_x < rect.left() || min_x > rect.right()
                    || max_y < rect.top()  || min_y > rect.bottom() {
                    continue;
                }

                if detailed {
                    // Full tile fill and selection are only useful at readable sizes.
                    let tile_data = scene.get_tile(tile);
                    if !tile_data.is_empty() {
                        let definition = tile_definitions.iter().find(|definition| definition.id == tile_data.tile_type);
                        let texture = definition.and_then(|definition| definition.texture.as_deref())
                            .and_then(|path| self.load_tile_texture_id(ui.ctx(), asset_root, path));
                        if let Some(texture_id) = texture {
                            let base_uvs = [
                                egui::pos2(0.5, 0.0), egui::pos2(1.0, 0.5),
                                egui::pos2(0.5, 1.0), egui::pos2(0.0, 0.5),
                            ];
                            let rotation = definition
                                .filter(|definition| definition.randomize_texture_rotation)
                                .map(|_| tile_texture_rotation(tile, tile_data.tile_type))
                                .unwrap_or(0);
                            let uvs: [egui::Pos2; 4] =
                                std::array::from_fn(|index| base_uvs[(index + rotation) % 4]);
                            let mut mesh = egui::Mesh::with_texture(texture_id);
                            let first = mesh.vertices.len() as u32;
                            for (position, uv) in corners.into_iter().zip(uvs) {
                                mesh.vertices.push(egui::epaint::Vertex { pos: position, uv, color: Color32::WHITE });
                            }
                            mesh.add_triangle(first, first + 1, first + 2);
                            mesh.add_triangle(first, first + 2, first + 3);
                            painter.add(egui::Shape::mesh(mesh));
                        } else {
                            painter.add(egui::Shape::convex_polygon(
                                corners.to_vec(), tile_type_color(tile_data.tile_type, tile_definitions), Stroke::NONE,
                            ));
                        }
                    }
                    if self.selected_tiles.contains(&tile) {
                        painter.add(egui::Shape::convex_polygon(
                            corners.to_vec(),
                            Color32::from_rgba_unmultiplied(70, 150, 255, 34),
                            Stroke::new(2.0_f32, self.options.selection_color),
                        ));
                    }
                }

                // Grille.
                if show_grid {
                    // Survol.
                    let is_hovered = detailed && self.hovered_tile == Some(tile);
                    if is_hovered {
                        painter.add(egui::Shape::convex_polygon(
                            corners.to_vec(),
                            self.options.hover_color,
                            Stroke::NONE,
                        ));
                    }

                    // Contour.
                    for i in 0..4 {
                        painter.line_segment(
                            [corners[i], corners[(i + 1) % 4]],
                            Stroke::new(1.0_f32, self.options.grid_color),
                        );
                    }
                }
            }
        }
    }

    /// Dessine les objets de la scène (placeholder rectangulaire coloré).
    fn draw_low_zoom_tiles(
        &self,
        scene: &Scene,
        tile_definitions: &[TileDefinition],
        painter: &Painter,
        proj: &IsoProjection,
        center: Vec2,
        rect: Rect,
        camera_tile: TilePos,
        tile_range: i32,
    ) {
        let chunk_size = world::grid::CHUNK_SIZE;
        let min_chunk_x = (camera_tile.x - tile_range).div_euclid(chunk_size) - 1;
        let max_chunk_x = (camera_tile.x + tile_range).div_euclid(chunk_size) + 1;
        let min_chunk_y = (camera_tile.y - tile_range).div_euclid(chunk_size) - 1;
        let max_chunk_y = (camera_tile.y + tile_range).div_euclid(chunk_size) + 1;
        let zoom = self.camera.zoom;
        let half_width = proj.half_width() * zoom;
        let half_height = proj.half_height() * zoom;
        let mut mesh = egui::Mesh::default();

        for chunk_y in min_chunk_y..=max_chunk_y {
            for chunk_x in min_chunk_x..=max_chunk_x {
                let Some(chunk) = scene.grid.chunks.get(&(chunk_x, chunk_y)) else { continue };
                let chunk_center = TilePos::new(
                    chunk_x * chunk_size + chunk_size / 2,
                    chunk_y * chunk_size + chunk_size / 2,
                );
                let screen = self.tile_to_screen(chunk_center, proj, center);
                let bounds = Rect::from_min_max(
                    egui::pos2(
                        screen.x - half_width * chunk_size as f32,
                        screen.y - half_height * chunk_size as f32,
                    ),
                    egui::pos2(
                        screen.x + half_width * chunk_size as f32,
                        screen.y + half_height * chunk_size as f32,
                    ),
                );
                if !bounds.intersects(rect) { continue; }

                for (index, data) in chunk.tiles.iter().copied().enumerate() {
                    if data.is_empty() { continue; }
                    let tile = TilePos::new(
                        chunk_x * chunk_size + (index as i32 % chunk_size),
                        chunk_y * chunk_size + (index as i32 / chunk_size),
                    );
                    let screen = self.tile_to_screen(tile, proj, center);
                    let tile_bounds = Rect::from_min_max(
                        egui::pos2(screen.x - half_width, screen.y - half_height),
                        egui::pos2(screen.x + half_width, screen.y + half_height),
                    );
                    if !tile_bounds.intersects(rect) { continue; }

                    let color = tile_type_color(data.tile_type, tile_definitions);
                    let first = mesh.vertices.len() as u32;
                    mesh.colored_vertex(egui::pos2(screen.x, screen.y - half_height), color);
                    mesh.colored_vertex(egui::pos2(screen.x + half_width, screen.y), color);
                    mesh.colored_vertex(egui::pos2(screen.x, screen.y + half_height), color);
                    mesh.colored_vertex(egui::pos2(screen.x - half_width, screen.y), color);
                    mesh.add_triangle(first, first + 1, first + 2);
                    mesh.add_triangle(first, first + 2, first + 3);
                }
            }
        }
        if !mesh.is_empty() {
            painter.add(egui::Shape::mesh(mesh));
        }
    }

    fn draw_objects(
        &mut self,
        ctx: &egui::Context,
        scene: &Scene,
        painter: &Painter,
        proj: &IsoProjection,
        center: Vec2,
        viewport: Rect,
        asset_root: &Path,
        show_cameras: bool,
    ) {
        let zoom = self.camera.zoom;
        self.object_hit_rects.clear();
        for obj in scene.objects_sorted_for_render() {
            if obj.object_type == ObjectType::Camera {
                if show_cameras && !obj.hidden {
                    let screen = self.tile_to_screen(obj.tile_pos, proj, center);
                    let icon_size = (16.0 * zoom).clamp(11.0, 22.0);
                    let icon_rect = Rect::from_center_size(
                        egui::pos2(screen.x, screen.y - 8.0 * zoom),
                        Vec2::new(icon_size * 1.25, icon_size),
                    );
                    self.object_hit_rects.insert(obj.id, icon_rect);
                    if icon_rect.intersects(viewport) {
                        let selected = self.selected_object == Some(obj.id)
                            || self.selected_objects.contains(&obj.id);
                        let color = if scene.main_camera == Some(obj.id) {
                            Color32::from_rgb(75, 180, 190)
                        } else {
                            Color32::from_rgb(105, 125, 145)
                        };
                        painter.rect_filled(icon_rect, 2.0, color);
                        painter.rect_stroke(icon_rect, 2.0, Stroke::new(
                            if selected { 2.0_f32 } else { 1.0_f32 },
                            if selected { self.options.selection_color } else { Color32::WHITE },
                        ));
                        painter.circle_stroke(icon_rect.center(), icon_size * 0.22,
                            Stroke::new(1.5_f32, Color32::WHITE));
                        painter.line_segment([
                            egui::pos2(icon_rect.left() + icon_size * 0.2, icon_rect.bottom()),
                            egui::pos2(icon_rect.left(), icon_rect.bottom() + icon_size * 0.2),
                        ], Stroke::new(1.0_f32, color));
                    }
                }
                continue;
            }
            if obj.hidden {
                continue;
            }
            if self.move_objects.iter().any(|(id, _)| *id == obj.id) {
                continue;
            }
            let screen = self.tile_to_screen(obj.tile_pos, proj, center);
            let texture_path = obj.asset_path.as_deref().filter(|path| !path.is_empty())
                .or_else(|| obj.properties.get("texture").and_then(|value| value.as_str()).filter(|path| !path.is_empty()));
            let texture = texture_path.and_then(|path| self.load_texture(ctx, asset_root, path));
            let size = Vec2::new(proj.tile_width * zoom, proj.tile_height * zoom);
            let tile_rect = Rect::from_min_size(
                egui::pos2(screen.x - size.x * 0.5, screen.y - size.y),
                size,
            );
            let texture_id = texture.as_ref().map(|(texture_id, _)| *texture_id);
            let r = match (texture.as_ref(), obj.fit_texture_to_diamond) {
                (Some((_, texture_size)), false) => contain_rect(tile_rect, *texture_size),
                _ => tile_rect,
            };
            self.object_hit_rects.insert(obj.id, r);
            if !r.intersects(viewport) { continue; }

            let selected = self.selected_object == Some(obj.id)
                || self.selected_objects.contains(&obj.id);
            let size = r.size();
            if let Some(texture_id) = texture_id {
                if obj.fit_texture_to_diamond {
                    let half_width = proj.half_width() * zoom;
                    let half_height = proj.half_height() * zoom;
                    let corners = [
                        egui::pos2(screen.x, screen.y - half_height),
                        egui::pos2(screen.x + half_width, screen.y),
                        egui::pos2(screen.x, screen.y + half_height),
                        egui::pos2(screen.x - half_width, screen.y),
                    ];
                    let uvs = [
                        egui::pos2(0.5, 0.0),
                        egui::pos2(1.0, 0.5),
                        egui::pos2(0.5, 1.0),
                        egui::pos2(0.0, 0.5),
                    ];
                    let mut mesh = egui::Mesh::with_texture(texture_id);
                    let first = mesh.vertices.len() as u32;
                    for (position, uv) in corners.into_iter().zip(uvs) {
                        mesh.vertices.push(egui::epaint::Vertex {
                            pos: position,
                            uv,
                            color: Color32::WHITE,
                        });
                    }
                    mesh.add_triangle(first, first + 1, first + 2);
                    mesh.add_triangle(first, first + 2, first + 3);
                    painter.add(egui::Shape::mesh(mesh));
                    if selected {
                        for index in 0..corners.len() {
                            painter.line_segment(
                                [corners[index], corners[(index + 1) % corners.len()]],
                                Stroke::new(2.0_f32, self.options.selection_color),
                            );
                        }
                    }
                } else {
                    painter.image(
                        texture_id,
                        r,
                        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    if selected {
                        painter.rect_stroke(r, 0.0, Stroke::new(2.0_f32, self.options.selection_color));
                    }
                }
            } else {
                let color = if selected {
                    Color32::from_rgb(100, 180, 255)
                } else {
                    Color32::from_rgb(180, 120, 80)
                };
                painter.rect(r, 4.0, color, Stroke::new(2.0_f32, Color32::WHITE));
            }

            // Orientation marker makes the four rotation states visible even
            // while objects still use placeholder rectangles.
            let direction = match obj.rotation {
                Rotation4::North => Vec2::new(0.0, -1.0),
                Rotation4::East => Vec2::new(1.0, 0.0),
                Rotation4::South => Vec2::new(0.0, 1.0),
                Rotation4::West => Vec2::new(-1.0, 0.0),
            };
            if texture_id.is_none() {
                painter.line_segment(
                    [r.center(), r.center() + direction * (size.y.min(size.x) * 0.28)],
                    Stroke::new(2.0_f32, Color32::WHITE),
                );
            }

            // Nom de l'objet.
            if texture_id.is_none() && zoom >= 0.5 {
                painter.text(
                    r.center(),
                    egui::Align2::CENTER_CENTER,
                    &obj.name,
                    egui::FontId::proportional(10.0 * zoom),
                    Color32::WHITE,
                );
            }
        }
    }

    fn load_texture(
        &mut self,
        ctx: &egui::Context,
        asset_root: &Path,
        asset_path: &str,
    ) -> Option<(egui::TextureId, Vec2)> {
        let cache_key = format!("{}|{}", asset_root.display(), asset_path);
        if !self.texture_cache.contains_key(&cache_key) {
            let path = Path::new(asset_path);
            let path = if path.is_absolute() { path.to_owned() } else { asset_root.join(path) };
            let texture = image::open(path).ok().map(|image| {
                let rgba = image.into_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
                ctx.load_texture(cache_key.clone(), color_image, egui::TextureOptions::NEAREST)
            });
            self.texture_cache.insert(cache_key.clone(), texture);
        }
        self.texture_cache.get(&cache_key)
            .and_then(Option::as_ref)
            .map(|texture| (texture.id(), texture.size_vec2()))
    }

    fn load_tile_texture_id(
        &mut self,
        ctx: &egui::Context,
        asset_root: &Path,
        asset_path: &str,
    ) -> Option<egui::TextureId> {
        let path = Path::new(asset_path);
        if path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("gif")) {
            let cache_key = format!("{}|{}", asset_root.display(), asset_path);
            return self.animated_tile_textures.get(&cache_key).map(|animation| animation.texture.id());
        }
        self.load_texture(ctx, asset_root, asset_path).map(|(id, _)| id)
    }

    fn update_animated_tile_textures(
        &mut self,
        ctx: &egui::Context,
        asset_root: &Path,
        definitions: &[TileDefinition],
        delta_time: f32,
    ) {
        for path in definitions.iter().filter_map(|definition| definition.texture.as_deref()) {
            let relative = Path::new(path);
            if !relative.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("gif")) {
                continue;
            }
            let cache_key = format!("{}|{}", asset_root.display(), path);
            if self.animated_tile_textures.contains_key(&cache_key)
                || self.failed_animated_tile_textures.contains(&cache_key) {
                continue;
            }
            let full_path = if relative.is_absolute() { relative.to_owned() } else { asset_root.join(relative) };
            if let Some(animation) = load_animated_tile_texture(ctx, &full_path, &cache_key) {
                self.animated_tile_textures.insert(cache_key, animation);
            } else {
                self.failed_animated_tile_textures.insert(cache_key);
            }
        }
        for animation in self.animated_tile_textures.values_mut() {
            animation.advance(delta_time);
        }
    }

    fn draw_move_preview(&self, scene: &Scene, painter: &Painter, proj: &IsoProjection, center: Vec2) {
        let (Some(start), Some(cursor)) = (self.move_start, self.hovered_tile) else {
            return;
        };
        let dx = cursor.x - start.x;
        let dy = cursor.y - start.y;
        if dx == 0 && dy == 0 {
            return;
        }

        let zoom = self.camera.zoom;
        let half_width = proj.half_width() * zoom;
        let half_height = proj.half_height() * zoom;
        let fill = Color32::from_rgba_unmultiplied(190, 200, 210, 105);
        let outline = Stroke::new(1.5_f32, Color32::from_rgb(215, 220, 225));

        for &(tile, _) in &self.move_tiles {
            let destination = TilePos::with_z(tile.x + dx, tile.y + dy, tile.z);
            let screen = self.tile_to_screen(destination, proj, center);
            let corners = vec![
                egui::pos2(screen.x, screen.y - half_height),
                egui::pos2(screen.x + half_width, screen.y),
                egui::pos2(screen.x, screen.y + half_height),
                egui::pos2(screen.x - half_width, screen.y),
            ];
            painter.add(egui::Shape::convex_polygon(corners.to_vec(), fill, outline));
        }

        let object_size = Vec2::new(proj.tile_width * zoom, proj.tile_height * zoom);
        for &(id, original_pos) in &self.move_objects {
            let Some(object) = scene.get_object(id) else { continue };
            if object.hidden { continue; }
            let destination = TilePos::with_z(original_pos.x + dx, original_pos.y + dy, original_pos.z);
            let screen = self.tile_to_screen(destination, proj, center);
            let rect = Rect::from_min_size(
                egui::pos2(screen.x - object_size.x * 0.5, screen.y - object_size.y),
                object_size,
            );
            painter.rect_filled(rect, 3.0, Color32::from_rgba_unmultiplied(190, 200, 210, 95));
            painter.rect_stroke(rect, 3.0, outline);
            if zoom >= 0.5 {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &object.name,
                    egui::FontId::proportional(10.0 * zoom),
                    Color32::WHITE,
                );
            }
        }
    }

    fn draw_rotate_preview(&self, painter: &Painter, proj: &IsoProjection, center: Vec2) {
        let Some(bounds) = self.rotate_bounds else { return };
        let steps = self.rotate_steps.rem_euclid(4);
        let zoom = self.camera.zoom;
        let half_width = proj.half_width() * zoom;
        let half_height = proj.half_height() * zoom;
        let fill = Color32::from_rgba_unmultiplied(190, 200, 210, 105);
        let outline = Stroke::new(1.5_f32, Color32::from_rgb(215, 220, 225));
        for &(tile, _) in &self.rotate_tiles {
            let destination = rotate_tile_in_bounds(tile, bounds, steps);
            let screen = self.tile_to_screen(destination, proj, center);
            let corners = vec![
                egui::pos2(screen.x, screen.y - half_height),
                egui::pos2(screen.x + half_width, screen.y),
                egui::pos2(screen.x, screen.y + half_height),
                egui::pos2(screen.x - half_width, screen.y),
            ];
            painter.add(egui::Shape::convex_polygon(corners, fill, outline));
        }
        painter.text(
            egui::pos2(self.rotate_pivot.x + 20.0, self.rotate_pivot.y - 20.0),
            egui::Align2::LEFT_BOTTOM,
            format!("{}°", self.rotate_steps * 90),
            egui::FontId::proportional(14.0),
            Color32::WHITE,
        );
    }

    fn draw_build_preview(
        &self,
        prefab: &Prefab,
        base: TilePos,
        painter: &Painter,
        proj: &IsoProjection,
        center: Vec2,
    ) {
        let zoom = self.camera.zoom;
        let half_width = proj.half_width() * zoom;
        let half_height = proj.half_height() * zoom;
        let fill = Color32::from_rgba_unmultiplied(100, 180, 255, 75);
        let outline = Stroke::new(1.5_f32, Color32::from_rgb(190, 220, 255));

        for element in &prefab.elements {
            let tile = TilePos::with_z(
                base.x + element.relative_pos.x,
                base.y + element.relative_pos.y,
                base.z + element.relative_pos.z,
            );
            let screen = self.tile_to_screen(tile, proj, center);
            let corners = [
                egui::pos2(screen.x, screen.y - half_height),
                egui::pos2(screen.x + half_width, screen.y),
                egui::pos2(screen.x, screen.y + half_height),
                egui::pos2(screen.x - half_width, screen.y),
            ];
            painter.add(egui::Shape::convex_polygon(corners.to_vec(), fill, outline));
            if zoom >= 0.7 {
                painter.text(
                    screen,
                    egui::Align2::CENTER_CENTER,
                    element.module_kind.label(),
                    egui::FontId::proportional(9.0 * zoom),
                    Color32::WHITE,
                );
            }
        }
    }

    fn draw_module_preview(
        &self,
        kind: BuildingModuleKind,
        tile: TilePos,
        painter: &Painter,
        proj: &IsoProjection,
        center: Vec2,
    ) {
        let screen = self.tile_to_screen(tile, proj, center);
        let half_width = proj.half_width() * self.camera.zoom;
        let half_height = proj.half_height() * self.camera.zoom;
        let corners = [
            egui::pos2(screen.x, screen.y - half_height),
            egui::pos2(screen.x + half_width, screen.y),
            egui::pos2(screen.x, screen.y + half_height),
            egui::pos2(screen.x - half_width, screen.y),
        ];
        painter.add(egui::Shape::convex_polygon(
            corners.to_vec(),
            Color32::from_rgba_unmultiplied(100, 180, 255, 90),
            Stroke::new(1.5_f32, Color32::from_rgb(190, 220, 255)),
        ));
        painter.text(
            screen,
            egui::Align2::CENTER_CENTER,
            kind.label(),
            egui::FontId::proportional(9.0 * self.camera.zoom),
            Color32::WHITE,
        );
    }

    fn draw_object_selection_bounds(
        &self,
        painter: &Painter,
        proj: &IsoProjection,
        center: Vec2,
    ) {
        let (Some(start), Some(end)) = (self.selection_start, self.hovered_tile) else {
            return;
        };
        let min_x = start.x.min(end.x) as f32 - 0.5;
        let max_x = start.x.max(end.x) as f32 + 0.5;
        let min_y = start.y.min(end.y) as f32 - 0.5;
        let max_y = start.y.max(end.y) as f32 + 0.5;
        let corners = [
            self.logical_to_screen(min_x, min_y, 0.0, proj, center),
            self.logical_to_screen(max_x, min_y, 0.0, proj, center),
            self.logical_to_screen(max_x, max_y, 0.0, proj, center),
            self.logical_to_screen(min_x, max_y, 0.0, proj, center),
        ];
        for index in 0..corners.len() {
            painter.line_segment(
                [corners[index], corners[(index + 1) % corners.len()]],
                Stroke::new(2.0_f32, self.options.selection_color),
            );
        }
    }

    /// Convertit une TilePos en position écran (dans le repère du painter).
    fn tile_to_screen(&self, tile: TilePos, proj: &IsoProjection, center: Vec2) -> egui::Pos2 {
        self.logical_to_screen(tile.x as f32, tile.y as f32, tile.z as f32, proj, center)
    }

    fn logical_to_screen(&self, x: f32, y: f32, z: f32, proj: &IsoProjection, center: Vec2) -> egui::Pos2 {
        let world = Vec2::new(
            (x - y) * proj.half_width(),
            (x + y) * proj.half_height() - z * proj.tile_height,
        );
        // world_to_screen already adds its viewport center. Use zero here,
        // then translate into the painter's local viewport coordinates once.
        let screen = self.camera.world_to_screen(world, Vec2::ZERO);
        egui::pos2(screen.x + center.x, screen.y + center.y)
    }

    /// Trouve l'objet de la scène à la position de tuile donnée.
    fn find_object_at(&self, scene: &Scene, tile: TilePos) -> Option<ObjectId> {
        scene.objects.values()
            .find(|o| !o.hidden && o.tile_pos == tile)
            .map(|o| o.id)
    }

    fn find_object_at_screen(
        &self,
        scene: &Scene,
        pointer: egui::Pos2,
        proj: &IsoProjection,
        center: Vec2,
    ) -> Option<ObjectId> {
        let default_size = Vec2::new(
            proj.tile_width * self.camera.zoom,
            proj.tile_height * self.camera.zoom,
        );
        scene.objects_sorted_for_render().into_iter().rev().find_map(|object| {
            if object.hidden { return None; }
            let bounds = self.object_hit_rects.get(&object.id).copied().unwrap_or_else(|| {
                let screen = self.tile_to_screen(object.tile_pos, proj, center);
                Rect::from_min_size(
                    egui::pos2(screen.x - default_size.x * 0.5, screen.y - default_size.y),
                    default_size,
                )
            });
            bounds.contains(pointer).then_some(object.id)
        })
    }
}

fn load_animated_tile_texture(
    ctx: &egui::Context,
    path: &Path,
    cache_key: &str,
) -> Option<AnimatedTileTexture> {
    let file = File::open(path).ok()?;
    let decoder = image::codecs::gif::GifDecoder::new(BufReader::new(file)).ok()?;
    let mut frame_iterator = decoder.into_frames();
    let mut decoded_frames = Vec::new();
    for _ in 0..512 {
        match frame_iterator.next() {
            Some(Ok(frame)) => decoded_frames.push(frame),
            Some(Err(_)) => return None,
            None => break,
        }
    }
    let mut frames = Vec::with_capacity(decoded_frames.len());
    for frame in decoded_frames {
        let (numerator, denominator) = frame.delay().numer_denom_ms();
        let duration_seconds = (numerator as f32 / denominator.max(1) as f32 / 1000.0).max(0.02);
        let rgba = frame.into_buffer();
        let size = [rgba.width() as usize, rgba.height() as usize];
        frames.push(AnimatedTileFrame {
            image: egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()),
            duration_seconds,
        });
    }
    let first = frames.first()?.image.clone();
    let texture = ctx.load_texture(cache_key.to_owned(), first, egui::TextureOptions::NEAREST);
    Some(AnimatedTileTexture { frames, texture, current_frame: 0, elapsed_seconds: 0.0 })
}

fn objects_in_tile_rectangle(scene: &Scene, start: TilePos, end: TilePos) -> HashSet<ObjectId> {
    let min_x = start.x.min(end.x);
    let max_x = start.x.max(end.x);
    let min_y = start.y.min(end.y);
    let max_y = start.y.max(end.y);
    scene.objects.values()
        .filter(|object| {
            !object.hidden
                && (min_x..=max_x).contains(&object.tile_pos.x)
                && (min_y..=max_y).contains(&object.tile_pos.y)
        })
        .map(|object| object.id)
        .collect()
}

fn sole_object(objects: &HashSet<ObjectId>) -> Option<ObjectId> {
    (objects.len() == 1).then(|| objects.iter().next().copied()).flatten()
}

fn contain_rect(bounds: Rect, image_size: Vec2) -> Rect {
    let image_size = image_size.max(Vec2::splat(1.0));
    let scale = (bounds.width() / image_size.x).min(bounds.height() / image_size.y);
    Rect::from_center_size(bounds.center(), image_size * scale)
}

fn show_runtime_camera_selector(
    ctx: &egui::Context,
    scene: &Scene,
    viewport: Rect,
    selected_camera: &mut Option<ObjectId>,
) {
    let mut cameras: Vec<_> = scene.objects.values()
        .filter(|object| object.object_type == ObjectType::Camera)
        .map(|object| (object.id, object.name.clone()))
        .collect();
    cameras.sort_by(|a, b| a.1.cmp(&b.1));
    if selected_camera.is_some_and(|id| !cameras.iter().any(|(camera_id, _)| *camera_id == id)) {
        *selected_camera = None;
    }
    let selected_name = selected_camera
        .and_then(|id| cameras.iter().find(|(camera_id, _)| *camera_id == id).map(|(_, name)| name.as_str()))
        .unwrap_or("Vue libre");
    let position = egui::pos2(viewport.left() + 8.0, viewport.top() + 8.0);

    egui::Area::new(egui::Id::new("runtime_camera_selector"))
        .order(egui::Order::Foreground)
        .fixed_pos(position)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Caméra");
                    egui::ComboBox::from_id_salt("runtime_camera_choice")
                        .selected_text(selected_name)
                        .width(145.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(selected_camera, None, "Vue libre");
                            for (id, name) in &cameras {
                                ui.selectable_value(selected_camera, Some(*id), name);
                            }
                        });
                });
            });
        });
}

fn module_object(kind: BuildingModuleKind, mut tile: TilePos) -> SceneObject {
    if kind == BuildingModuleKind::Roof {
        tile.z += 1;
    }
    let mut object = SceneObject::new(kind.label(), ObjectType::Building, tile);
    if kind == BuildingModuleKind::Roof {
        object.layer = world::Layer::Foreground;
    } else {
        object.layer = world::Layer::Buildings;
    }
    object
}

fn erase_tile(scene: &mut Scene, tile: TilePos) -> bool {
    if scene.get_tile(tile).is_empty() {
        return false;
    }
    scene.clear_tile(tile);
    true
}

fn fill_tiles(
    scene: &mut Scene,
    tiles: impl IntoIterator<Item = TilePos>,
    tile_type: u16,
) -> bool {
    let mut modified = false;
    for tile in tiles {
        let old = scene.get_tile(tile);
        if old.tile_type != tile_type {
            scene.set_tile(tile, TileData { tile_type, ..old });
            modified = true;
        }
    }
    modified
}

/// Récupère le composant contigu de tuiles ayant les mêmes données que la tuile
/// de départ. Les tuiles vides restent limitées à la tuile cliquée pour éviter
/// de remplir une région infinie.
fn connected_tile_region(scene: &Scene, start: TilePos) -> Vec<TilePos> {
    let target = scene.get_tile(start);
    if target.is_empty() {
        return vec![start];
    }

    let mut region = HashSet::from([TilePos::new(start.x, start.y)]);
    let mut queue = std::collections::VecDeque::from([TilePos::new(start.x, start.y)]);
    while let Some(tile) = queue.pop_front() {
        let neighbors = [
            tile.x.checked_add(1).map(|x| TilePos::new(x, tile.y)),
            tile.x.checked_sub(1).map(|x| TilePos::new(x, tile.y)),
            tile.y.checked_add(1).map(|y| TilePos::new(tile.x, y)),
            tile.y.checked_sub(1).map(|y| TilePos::new(tile.x, y)),
        ];
        for neighbor in neighbors.into_iter().flatten() {
            if !region.contains(&neighbor) && scene.get_tile(neighbor) == target {
                region.insert(neighbor);
                queue.push_back(neighbor);
            }
        }
    }
    region.into_iter().collect()
}

fn rotate_tile_in_bounds(tile: TilePos, bounds: (i32, i32, i32, i32), steps: i32) -> TilePos {
    let (min_x, min_y, mut max_x, mut max_y) = bounds;
    let mut x = tile.x;
    let mut y = tile.y;
    for _ in 0..steps.rem_euclid(4) {
        let width = max_x - min_x;
        let height = max_y - min_y;
        let rotated_x = min_x + (max_y - y);
        let rotated_y = min_y + (x - min_x);
        x = rotated_x;
        y = rotated_y;
        max_x = min_x + height;
        max_y = min_y + width;
    }
    TilePos::with_z(x, y, tile.z)
}

fn tile_rectangle(start: TilePos, end: TilePos) -> HashSet<TilePos> {
    let min_x = start.x.min(end.x);
    let max_x = start.x.max(end.x);
    let min_y = start.y.min(end.y);
    let max_y = start.y.max(end.y);
    (min_y..=max_y)
        .flat_map(|y| (min_x..=max_x).map(move |x| TilePos::new(x, y)))
        .collect()
}

/// Couleur de prévisualisation selon le type de tuile (1-16 définis arbitrairement).
fn tile_type_color(tile_type: u16, definitions: &[TileDefinition]) -> Color32 {
    definitions.iter().find(|definition| definition.id == tile_type)
        .map(|definition| Color32::from_rgba_unmultiplied(
            definition.color[0], definition.color[1], definition.color[2], definition.color[3],
        ))
        .unwrap_or_else(|| Color32::from_rgb(60, 60, 60))
}

fn tile_texture_rotation(tile: TilePos, tile_type: u16) -> usize {
    let mut hash = (tile.x as u32).wrapping_mul(0x9E37_79B9)
        ^ (tile.y as u32).wrapping_mul(0x85EB_CA6B)
        ^ (tile_type as u32).wrapping_mul(0xC2B2_AE35);
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x7FEB_352D);
    hash ^= hash >> 15;
    hash = hash.wrapping_mul(0x846C_A68B);
    hash ^= hash >> 16;
    (hash & 3) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connected_fill_changes_only_four_connected_matching_tiles() {
        let mut scene = Scene::new("Fill component");
        let matching = TileData { tile_type: 1, decoration: 7 };
        scene.set_tile(TilePos::new(15, 0), matching);
        scene.set_tile(TilePos::new(16, 0), matching); // Across a chunk boundary.
        scene.set_tile(TilePos::new(16, 1), TileData::terrain(2)); // Blocks the route.
        scene.set_tile(TilePos::new(17, 0), TileData::terrain(2)); // Blocks the route.
        scene.set_tile(TilePos::new(17, 1), matching); // Diagonal, not connected.

        let region = connected_tile_region(&scene, TilePos::new(15, 0));
        assert_eq!(region.len(), 2);
        assert!(fill_tiles(&mut scene, region, 3));

        assert_eq!(scene.get_tile(TilePos::new(15, 0)), TileData { tile_type: 3, decoration: 7 });
        assert_eq!(scene.get_tile(TilePos::new(16, 0)), TileData { tile_type: 3, decoration: 7 });
        assert_eq!(scene.get_tile(TilePos::new(16, 1)), TileData::terrain(2));
        assert_eq!(scene.get_tile(TilePos::new(17, 0)), TileData::terrain(2));
        assert_eq!(scene.get_tile(TilePos::new(17, 1)), matching);
    }

    #[test]
    fn connected_fill_on_empty_tile_is_bounded_to_the_clicked_tile() {
        let scene = Scene::new("Empty fill");
        assert_eq!(connected_tile_region(&scene, TilePos::new(-4, 8)), vec![TilePos::new(-4, 8)]);
    }

    #[test]
    fn tile_texture_rotation_is_stable_and_spans_multiple_quarter_turns() {
        let tile = TilePos::new(-17, 42);
        let rotation = tile_texture_rotation(tile, 7);
        assert_eq!(tile_texture_rotation(tile, 7), rotation);
        assert!(rotation < 4);

        let rotations = (-50..50)
            .map(|x| tile_texture_rotation(TilePos::new(x, x * 3), 7))
            .collect::<HashSet<_>>();
        assert!(rotations.len() >= 3, "les emplacements doivent varier les orientations");
    }

    #[test]
    fn animated_gif_tile_texture_decodes_frames_and_advances() {
        use image::{codecs::gif::GifEncoder, Delay, Frame, Rgba, RgbaImage};
        use std::{fs::File, time::{SystemTime, UNIX_EPOCH}};

        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("isomaker_tile_animation_{}_{}.gif", std::process::id(), nonce));
        let frames = [
            Frame::from_parts(RgbaImage::from_pixel(2, 2, Rgba([255, 0, 0, 255])), 0, 0,
                Delay::from_numer_denom_ms(40, 1)),
            Frame::from_parts(RgbaImage::from_pixel(2, 2, Rgba([0, 255, 0, 255])), 0, 0,
                Delay::from_numer_denom_ms(40, 1)),
        ];
        GifEncoder::new(File::create(&path).unwrap())
            .encode_frames(frames)
            .unwrap();

        let context = egui::Context::default();
        let mut animation = load_animated_tile_texture(&context, &path, "test-gif")
            .expect("GIF animé valide");
        assert_eq!(animation.frames.len(), 2);
        assert_eq!(animation.current_frame, 0);
        animation.advance(0.05);
        assert_eq!(animation.current_frame, 1);
        animation.advance(0.05);
        assert_eq!(animation.current_frame, 0, "l'animation boucle");

        drop(animation);
        std::fs::remove_file(path).unwrap();
    }
}
