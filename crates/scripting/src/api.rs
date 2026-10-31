//! Objet Python `GameContext` exposé aux scripts via PyO3.

use crate::command::{GameCommand, HudAnchor, HudText, LogLevel};
use engine::{InputState, Key};
use pyo3::prelude::*;
use pyo3::types::PyModule;
use std::{collections::HashMap, sync::{Arc, Mutex}};
use world::object::{PropValue, SceneObject};

/// File de commandes partagée entre le contexte Python et le moteur Rust.
pub type CommandQueue = Arc<Mutex<Vec<GameCommand>>>;
pub type GlobalStore = Arc<Mutex<HashMap<String, serde_json::Value>>>;

/// Constante de touche typée, utilisée à la place d'une chaîne fragile.
#[pyclass(name = "Key", frozen)]
#[derive(Clone)]
pub struct PyKey {
    key: Key,
}

/// Espace de noms des touches disponibles sur `game.keys`.
#[pyclass(name = "KeySet", frozen)]
pub struct PyKeySet;

#[pymethods]
impl PyKeySet {
    #[getter(A)]
    fn a(&self) -> PyKey {
        PyKey { key: Key::A }
    }
    #[getter(B)]
    fn b(&self) -> PyKey {
        PyKey { key: Key::B }
    }
    #[getter(C)]
    fn c(&self) -> PyKey {
        PyKey { key: Key::C }
    }
    #[getter(D)]
    fn d(&self) -> PyKey {
        PyKey { key: Key::D }
    }
    #[getter(E)]
    fn e(&self) -> PyKey {
        PyKey { key: Key::E }
    }
    #[getter(F)]
    fn f(&self) -> PyKey {
        PyKey { key: Key::F }
    }
    #[getter(G)]
    fn g(&self) -> PyKey {
        PyKey { key: Key::G }
    }
    #[getter(H)]
    fn h(&self) -> PyKey {
        PyKey { key: Key::H }
    }
    #[getter(I)]
    fn i(&self) -> PyKey {
        PyKey { key: Key::I }
    }
    #[getter(J)]
    fn j(&self) -> PyKey {
        PyKey { key: Key::J }
    }
    #[getter(K)]
    fn k(&self) -> PyKey {
        PyKey { key: Key::K }
    }
    #[getter(L)]
    fn l(&self) -> PyKey {
        PyKey { key: Key::L }
    }
    #[getter(M)]
    fn m(&self) -> PyKey {
        PyKey { key: Key::M }
    }
    #[getter(N)]
    fn n(&self) -> PyKey {
        PyKey { key: Key::N }
    }
    #[getter(O)]
    fn o(&self) -> PyKey {
        PyKey { key: Key::O }
    }
    #[getter(P)]
    fn p(&self) -> PyKey {
        PyKey { key: Key::P }
    }
    #[getter(Q)]
    fn q(&self) -> PyKey {
        PyKey { key: Key::Q }
    }
    #[getter(R)]
    fn r(&self) -> PyKey {
        PyKey { key: Key::R }
    }
    #[getter(S)]
    fn s(&self) -> PyKey {
        PyKey { key: Key::S }
    }
    #[getter(T)]
    fn t(&self) -> PyKey {
        PyKey { key: Key::T }
    }
    #[getter(U)]
    fn u(&self) -> PyKey {
        PyKey { key: Key::U }
    }
    #[getter(V)]
    fn v(&self) -> PyKey {
        PyKey { key: Key::V }
    }
    #[getter(W)]
    fn w(&self) -> PyKey {
        PyKey { key: Key::W }
    }
    #[getter(X)]
    fn x(&self) -> PyKey {
        PyKey { key: Key::X }
    }
    #[getter(Y)]
    fn y(&self) -> PyKey {
        PyKey { key: Key::Y }
    }
    #[getter(Z)]
    fn z(&self) -> PyKey {
        PyKey { key: Key::Z }
    }
    #[getter(DIGIT_0)]
    fn digit_0(&self) -> PyKey {
        PyKey { key: Key::Key0 }
    }
    #[getter(DIGIT_1)]
    fn digit_1(&self) -> PyKey {
        PyKey { key: Key::Key1 }
    }
    #[getter(DIGIT_2)]
    fn digit_2(&self) -> PyKey {
        PyKey { key: Key::Key2 }
    }
    #[getter(DIGIT_3)]
    fn digit_3(&self) -> PyKey {
        PyKey { key: Key::Key3 }
    }
    #[getter(DIGIT_4)]
    fn digit_4(&self) -> PyKey {
        PyKey { key: Key::Key4 }
    }
    #[getter(DIGIT_5)]
    fn digit_5(&self) -> PyKey {
        PyKey { key: Key::Key5 }
    }
    #[getter(DIGIT_6)]
    fn digit_6(&self) -> PyKey {
        PyKey { key: Key::Key6 }
    }
    #[getter(DIGIT_7)]
    fn digit_7(&self) -> PyKey {
        PyKey { key: Key::Key7 }
    }
    #[getter(DIGIT_8)]
    fn digit_8(&self) -> PyKey {
        PyKey { key: Key::Key8 }
    }
    #[getter(DIGIT_9)]
    fn digit_9(&self) -> PyKey {
        PyKey { key: Key::Key9 }
    }
    #[getter(SPACE)]
    fn space(&self) -> PyKey {
        PyKey { key: Key::Space }
    }
    #[getter(ENTER)]
    fn enter(&self) -> PyKey {
        PyKey { key: Key::Enter }
    }
    #[getter(ESCAPE)]
    fn escape(&self) -> PyKey {
        PyKey { key: Key::Escape }
    }
    #[getter(BACKSPACE)]
    fn backspace(&self) -> PyKey {
        PyKey {
            key: Key::Backspace,
        }
    }
    #[getter(DELETE)]
    fn delete(&self) -> PyKey {
        PyKey { key: Key::Delete }
    }
    #[getter(TAB)]
    fn tab(&self) -> PyKey {
        PyKey { key: Key::Tab }
    }
    #[getter(LEFT)]
    fn left(&self) -> PyKey {
        PyKey { key: Key::Left }
    }
    #[getter(RIGHT)]
    fn right(&self) -> PyKey {
        PyKey { key: Key::Right }
    }
    #[getter(UP)]
    fn up(&self) -> PyKey {
        PyKey { key: Key::Up }
    }
    #[getter(DOWN)]
    fn down(&self) -> PyKey {
        PyKey { key: Key::Down }
    }
    #[getter(SHIFT)]
    fn shift(&self) -> PyKey {
        PyKey { key: Key::Shift }
    }
    #[getter(CTRL)]
    fn ctrl(&self) -> PyKey {
        PyKey { key: Key::Control }
    }
    #[getter(ALT)]
    fn alt(&self) -> PyKey {
        PyKey { key: Key::Alt }
    }
    #[getter(F1)]
    fn f1(&self) -> PyKey {
        PyKey { key: Key::F1 }
    }
    #[getter(F2)]
    fn f2(&self) -> PyKey {
        PyKey { key: Key::F2 }
    }
    #[getter(F3)]
    fn f3(&self) -> PyKey {
        PyKey { key: Key::F3 }
    }
    #[getter(F4)]
    fn f4(&self) -> PyKey {
        PyKey { key: Key::F4 }
    }
    #[getter(F5)]
    fn f5(&self) -> PyKey {
        PyKey { key: Key::F5 }
    }
    #[getter(F6)]
    fn f6(&self) -> PyKey {
        PyKey { key: Key::F6 }
    }
    #[getter(F7)]
    fn f7(&self) -> PyKey {
        PyKey { key: Key::F7 }
    }
    #[getter(F8)]
    fn f8(&self) -> PyKey {
        PyKey { key: Key::F8 }
    }
    #[getter(F9)]
    fn f9(&self) -> PyKey {
        PyKey { key: Key::F9 }
    }
    #[getter(F10)]
    fn f10(&self) -> PyKey {
        PyKey { key: Key::F10 }
    }
    #[getter(F11)]
    fn f11(&self) -> PyKey {
        PyKey { key: Key::F11 }
    }
    #[getter(F12)]
    fn f12(&self) -> PyKey {
        PyKey { key: Key::F12 }
    }
}

#[pyclass(name = "UIAnchor", frozen)]
#[derive(Clone)]
pub struct PyUIAnchor {
    anchor: HudAnchor,
}

#[pyclass(name = "UIAnchorSet", frozen)]
pub struct PyUIAnchorSet;

#[pymethods]
impl PyUIAnchorSet {
    #[getter(TOP_LEFT)] fn top_left(&self) -> PyUIAnchor { PyUIAnchor { anchor: HudAnchor::TopLeft } }
    #[getter(TOP_CENTER)] fn top_center(&self) -> PyUIAnchor { PyUIAnchor { anchor: HudAnchor::TopCenter } }
    #[getter(TOP_RIGHT)] fn top_right(&self) -> PyUIAnchor { PyUIAnchor { anchor: HudAnchor::TopRight } }
    #[getter(CENTER_LEFT)] fn center_left(&self) -> PyUIAnchor { PyUIAnchor { anchor: HudAnchor::CenterLeft } }
    #[getter(CENTER)] fn center(&self) -> PyUIAnchor { PyUIAnchor { anchor: HudAnchor::Center } }
    #[getter(CENTER_RIGHT)] fn center_right(&self) -> PyUIAnchor { PyUIAnchor { anchor: HudAnchor::CenterRight } }
    #[getter(BOTTOM_LEFT)] fn bottom_left(&self) -> PyUIAnchor { PyUIAnchor { anchor: HudAnchor::BottomLeft } }
    #[getter(BOTTOM_CENTER)] fn bottom_center(&self) -> PyUIAnchor { PyUIAnchor { anchor: HudAnchor::BottomCenter } }
    #[getter(BOTTOM_RIGHT)] fn bottom_right(&self) -> PyUIAnchor { PyUIAnchor { anchor: HudAnchor::BottomRight } }
}

#[pyclass(name = "GameUI")]
pub struct PyGameUI {
    queue: CommandQueue,
}

#[pymethods]
impl PyGameUI {
    /// Crée ou met à jour un texte identifié dans le HUD.
    #[pyo3(signature = (id, text, x=0.0, y=0.0, anchor=None, font_size=24.0, color=None, visible=true))]
    fn set_text(
        &self,
        id: &str,
        text: &str,
        x: f32,
        y: f32,
        anchor: Option<PyRef<'_, PyUIAnchor>>,
        font_size: f32,
        color: Option<(u8, u8, u8, u8)>,
        visible: bool,
    ) -> PyResult<()> {
        if id.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err("L'identifiant du texte HUD ne peut pas être vide."));
        }
        if !x.is_finite() || !y.is_finite() || !font_size.is_finite() {
            return Err(pyo3::exceptions::PyValueError::new_err("La position et la taille du texte doivent être des nombres finis."));
        }
        let anchor = anchor.map(|value| value.anchor).unwrap_or(HudAnchor::TopLeft);
        let color = color.unwrap_or((255, 255, 255, 255));
        self.queue.lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Verrou de commande inaccessible"))?
            .push(GameCommand::SetHudText(HudText {
                id: id.to_owned(),
                text: text.to_owned(),
                x,
                y,
                anchor,
                font_size: font_size.clamp(6.0, 128.0),
                color: [color.0, color.1, color.2, color.3],
                visible,
            }));
        Ok(())
    }

    /// Met à jour uniquement le contenu d'un texte HUD existant.
    fn update_text(&self, id: &str, text: &str) -> PyResult<()> {
        self.queue.lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Verrou de commande inaccessible"))?
            .push(GameCommand::UpdateHudText { id: id.to_owned(), text: text.to_owned() });
        Ok(())
    }

    /// Met à jour uniquement la position d'un texte HUD existant.
    fn update_position(&self, id: &str, x: f32, y: f32) -> PyResult<()> {
        if !x.is_finite() || !y.is_finite() {
            return Err(pyo3::exceptions::PyValueError::new_err("La position du texte doit être composée de nombres finis."));
        }
        self.queue.lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Verrou de commande inaccessible"))?
            .push(GameCommand::UpdateHudPosition { id: id.to_owned(), x, y });
        Ok(())
    }

    /// Met à jour uniquement l'ancrage d'un texte HUD existant.
    fn update_anchor(&self, id: &str, anchor: PyRef<'_, PyUIAnchor>) -> PyResult<()> {
        self.queue.lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Verrou de commande inaccessible"))?
            .push(GameCommand::UpdateHudAnchor { id: id.to_owned(), anchor: anchor.anchor });
        Ok(())
    }

    /// Met à jour uniquement la taille de police d'un texte HUD existant.
    fn update_font_size(&self, id: &str, font_size: f32) -> PyResult<()> {
        if !font_size.is_finite() {
            return Err(pyo3::exceptions::PyValueError::new_err("La taille du texte doit être un nombre fini."));
        }
        self.queue.lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Verrou de commande inaccessible"))?
            .push(GameCommand::UpdateHudFontSize { id: id.to_owned(), font_size: font_size.clamp(6.0, 128.0) });
        Ok(())
    }

    /// Met à jour uniquement la couleur RGBA d'un texte HUD existant.
    fn update_color(&self, id: &str, color: (u8, u8, u8, u8)) -> PyResult<()> {
        self.queue.lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Verrou de commande inaccessible"))?
            .push(GameCommand::UpdateHudColor { id: id.to_owned(), color: [color.0, color.1, color.2, color.3] });
        Ok(())
    }

    /// Fait diminuer progressivement l'opacité d'un texte, puis le supprime.
    fn fade_out(&self, id: &str, duration: f32) -> PyResult<()> {
        if !duration.is_finite() || duration <= 0.0 {
            return Err(pyo3::exceptions::PyValueError::new_err("La durée du fondu doit être un nombre positif."));
        }
        self.queue.lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Verrou de commande inaccessible"))?
            .push(GameCommand::FadeOutHudText { id: id.to_owned(), duration });
        Ok(())
    }

    /// Fait apparaître progressivement un texte HUD existant.
    fn fade_in(&self, id: &str, duration: f32) -> PyResult<()> {
        if !duration.is_finite() || duration <= 0.0 {
            return Err(pyo3::exceptions::PyValueError::new_err("La durée du fondu doit être un nombre positif."));
        }
        self.queue.lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Verrou de commande inaccessible"))?
            .push(GameCommand::FadeInHudText { id: id.to_owned(), duration });
        Ok(())
    }

    /// Supprime un texte HUD par son identifiant.
    fn remove_text(&self, id: &str) -> PyResult<()> {
        self.queue.lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Verrou de commande inaccessible"))?
            .push(GameCommand::RemoveHudText { id: id.to_owned() });
        Ok(())
    }
}

/// Instantané en lecture seule d'un objet de scène.
#[pyclass(name = "GameObject", frozen)]
#[derive(Clone)]
pub struct PyGameObject {
    object: SceneObject,
}

impl PyGameObject {
    pub(crate) fn from_object(object: SceneObject) -> Self {
        Self { object }
    }
}

/// Données immuables d'une scène transmises aux scripts Python.
#[derive(Clone)]
pub struct SceneSnapshot {
    pub name: String,
    pub objects: Vec<SceneObject>,
}

/// Instantané en lecture seule d'une scène du projet.
#[pyclass(name = "GameScene", frozen)]
#[derive(Clone)]
pub struct PyGameScene {
    scene: SceneSnapshot,
}

#[pymethods]
impl PyGameScene {
    #[getter]
    fn name(&self) -> String {
        self.scene.name.clone()
    }

    #[getter]
    fn objects(&self) -> Vec<PyGameObject> {
        self.scene.objects.iter().cloned().map(PyGameObject::from_object).collect()
    }

    /// Cherche un objet par UUID dans cette scène.
    fn get_object(&self, object_id: &str) -> Option<PyGameObject> {
        self.scene.objects.iter()
            .find(|object| object.id.to_string().eq_ignore_ascii_case(object_id))
            .cloned()
            .map(PyGameObject::from_object)
    }

    /// Retourne le premier objet dont le nom correspond exactement.
    fn find_object(&self, name: &str) -> Option<PyGameObject> {
        self.scene.objects.iter()
            .filter(|object| object.name == name)
            .min_by_key(|object| object.id.to_string())
            .cloned()
            .map(PyGameObject::from_object)
    }
}

#[pymethods]
impl PyGameObject {
    #[getter]
    fn id(&self) -> String {
        self.object.id.to_string()
    }
    #[getter]
    fn object_id(&self) -> String {
        self.object.id.to_string()
    }
    #[getter]
    fn name(&self) -> String {
        self.object.name.clone()
    }
    #[getter]
    fn object_type(&self) -> String {
        object_type_name(&self.object.object_type)
    }
    #[getter]
    fn x(&self) -> i32 {
        self.object.tile_pos.x
    }
    #[getter]
    fn y(&self) -> i32 {
        self.object.tile_pos.y
    }
    #[getter]
    fn z(&self) -> i32 {
        self.object.tile_pos.z
    }
    #[getter]
    fn has_collisions(&self) -> bool {
        self.object.has_collisions
    }
    #[getter]
    fn hidden(&self) -> bool {
        self.object.hidden
    }
    #[getter]
    fn fit_texture_to_diamond(&self) -> bool {
        self.object.fit_texture_to_diamond
    }
    #[getter]
    fn camera_zoom(&self) -> f32 {
        self.object.camera_zoom
    }
    #[getter]
    fn camera_interpolation(&self) -> bool {
        self.object.camera_interpolation
    }
    #[getter]
    fn camera_interpolation_speed(&self) -> f32 {
        self.object.camera_interpolation_speed
    }
    #[getter]
    fn rotation(&self) -> String {
        format!("{:?}", self.object.rotation)
    }
    #[getter]
    fn layer(&self) -> String {
        format!("{:?}", self.object.layer)
    }
    #[getter]
    fn texture(&self) -> Option<String> {
        self.object
            .asset_path
            .clone()
            .filter(|path| !path.is_empty())
            .or_else(|| {
                self.object
                    .properties
                    .get("texture")
                    .and_then(PropValue::as_str)
                    .filter(|path| !path.is_empty())
                    .map(str::to_owned)
            })
    }
    #[getter]
    fn script(&self) -> Option<String> {
        self.object.script.clone()
    }
    #[getter]
    fn footprint_width(&self) -> u32 {
        self.object.footprint.width
    }
    #[getter]
    fn footprint_height(&self) -> u32 {
        self.object.footprint.height
    }

    /// Lit un champ intégré ou une propriété personnalisée de l'instantané.
    fn get_property(&self, py: Python<'_>, key: &str) -> PyResult<PyObject> {
        json_value_to_py(py, object_to_json_value(&self.object, key))
    }
}

/// Contexte de jeu exposé aux scripts Python.
///
/// Les méthodes de cet objet empilent des `GameCommand` dans la file partagée.
/// Le moteur les dépile après le retour du script.
#[pyclass(name = "GameContext")]
pub struct PyGameContext {
    queue: CommandQueue,
    pub delta_time: f32,
    object_id: Option<String>,
    tile_position: Option<(i32, i32, i32)>,
    self_object: Option<SceneObject>,
    scene_objects: Arc<Vec<SceneObject>>,
    scenes: Arc<Vec<SceneSnapshot>>,
    globals: GlobalStore,
    input: InputState,
    script_source: String,
}

impl PyGameContext {
    pub fn new(
        queue: CommandQueue,
        delta_time: f32,
        object_id: Option<String>,
        tile_position: Option<(i32, i32, i32)>,
        self_object: Option<SceneObject>,
        scene_objects: Arc<Vec<SceneObject>>,
        scenes: Arc<Vec<SceneSnapshot>>,
        globals: GlobalStore,
        input: InputState,
        script_source: String,
    ) -> Self {
        Self {
            queue,
            delta_time,
            object_id,
            tile_position,
            self_object,
            scene_objects,
            scenes,
            globals,
            input,
            script_source,
        }
    }

    fn push(&self, cmd: GameCommand) -> PyResult<()> {
        self.queue
            .lock()
            .map_err(|e| {
                pyo3::exceptions::PyRuntimeError::new_err(format!(
                    "Verrou de commande inaccessible : {e}"
                ))
            })?
            .push(cmd);
        Ok(())
    }

    fn require_self_object(&self) -> PyResult<&SceneObject> {
        self.self_object.as_ref().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "Cette propriété est disponible uniquement dans un script attaché à un objet",
            )
        })
    }
}

#[pymethods]
impl PyGameContext {
    #[getter]
    fn keys(&self) -> PyKeySet {
        PyKeySet
    }

    #[getter]
    fn ui(&self) -> PyGameUI {
        PyGameUI { queue: Arc::clone(&self.queue) }
    }

    #[getter]
    fn ui_anchor(&self) -> PyUIAnchorSet {
        PyUIAnchorSet
    }

    /// True tant que la touche est enfoncée.
    fn key_pressed(&self, key: PyRef<'_, PyKey>) -> bool {
        self.input.is_key_pressed(key.key)
    }

    /// True uniquement pendant la frame où la touche vient d'être enfoncée.
    fn key_just_pressed(&self, key: PyRef<'_, PyKey>) -> bool {
        self.input.is_key_just_pressed(key.key)
    }

    /// True uniquement pendant la frame où la touche vient d'être relâchée.
    fn key_just_released(&self, key: PyRef<'_, PyKey>) -> bool {
        self.input.is_key_just_released(key.key)
    }

    /// Alias de `key_just_released`, pour une API clavier symétrique.
    fn key_released(&self, key: PyRef<'_, PyKey>) -> bool {
        self.input.is_key_just_released(key.key)
    }

    /// Temps écoulé depuis le dernier frame, en secondes.
    #[getter]
    fn delta_time(&self) -> f32 {
        self.delta_time
    }

    /// Identifiant de l'objet auquel ce script est attaché (None pour le monde).
    #[getter]
    fn object_id(&self) -> Option<String> {
        self.object_id.clone()
    }

    /// Retourne un instantané en lecture seule de l'objet portant cet UUID.
    fn get_object(&self, object_id: &str) -> Option<PyGameObject> {
        self.scene_objects
            .iter()
            .find(|object| object.id.to_string().eq_ignore_ascii_case(object_id))
            .cloned()
            .map(|object| PyGameObject { object })
    }

    /// Retourne le premier objet dont le nom correspond exactement.
    fn find_object(&self, name: &str) -> Option<PyGameObject> {
        self.scene_objects
            .iter()
            .filter(|object| object.name == name)
            .min_by_key(|object| object.id.to_string())
            .cloned()
            .map(|object| PyGameObject { object })
    }

    /// Retourne l'instantané de l'objet portant ce script.
    fn get_self_object(&self) -> PyResult<PyGameObject> {
        Ok(PyGameObject::from_object(self.require_self_object()?.clone()))
    }

    /// Recherche un objet par nom exact, et signale une erreur s'il est absent.
    fn require_object(&self, name: &str) -> PyResult<PyGameObject> {
        self.find_object(name).ok_or_else(|| {
            pyo3::exceptions::PyLookupError::new_err(format!(
                "Aucun objet ne porte le nom exact « {name} » dans la scène active."
            ))
        })
    }

    /// Distance euclidienne 2D en unités de tuile, sans tenir compte de Z.
    fn get_distance(&self, object1: PyRef<'_, PyGameObject>, object2: PyRef<'_, PyGameObject>) -> f64 {
        let dx = f64::from(object1.object.tile_pos.x) - f64::from(object2.object.tile_pos.x);
        let dy = f64::from(object1.object.tile_pos.y) - f64::from(object2.object.tile_pos.y);
        dx.hypot(dy)
    }

    fn get_scene(&self, name: &str) -> Option<PyGameScene> {
        self.scenes.iter()
            .find(|scene| scene.name == name)
            .cloned()
            .map(|scene| PyGameScene { scene })
    }

    #[getter]
    fn name(&self) -> PyResult<String> {
        Ok(self.require_self_object()?.name.clone())
    }

    #[getter]
    fn object_type(&self) -> PyResult<String> {
        Ok(object_type_name(&self.require_self_object()?.object_type))
    }

    #[getter]
    fn has_collisions(&self) -> PyResult<bool> {
        Ok(self.require_self_object()?.has_collisions)
    }

    #[getter]
    fn hidden(&self) -> PyResult<bool> {
        Ok(self.require_self_object()?.hidden)
    }

    #[getter]
    fn fit_texture_to_diamond(&self) -> PyResult<bool> {
        Ok(self.require_self_object()?.fit_texture_to_diamond)
    }

    #[getter]
    fn rotation(&self) -> PyResult<String> {
        Ok(format!("{:?}", self.require_self_object()?.rotation))
    }

    #[getter]
    fn layer(&self) -> PyResult<String> {
        Ok(format!("{:?}", self.require_self_object()?.layer))
    }

    #[getter]
    fn texture(&self) -> PyResult<Option<String>> {
        let object = self.require_self_object()?;
        Ok(object
            .asset_path
            .clone()
            .filter(|path| !path.is_empty())
            .or_else(|| {
                object
                    .properties
                    .get("texture")
                    .and_then(PropValue::as_str)
                    .filter(|path| !path.is_empty())
                    .map(str::to_owned)
            }))
    }

    #[getter]
    fn script(&self) -> PyResult<Option<String>> {
        Ok(self.require_self_object()?.script.clone())
    }

    #[getter]
    fn camera_zoom(&self) -> PyResult<f32> {
        Ok(self.require_self_object()?.camera_zoom)
    }

    #[getter]
    fn camera_interpolation(&self) -> PyResult<bool> {
        Ok(self.require_self_object()?.camera_interpolation)
    }

    #[getter]
    fn camera_interpolation_speed(&self) -> PyResult<f32> {
        Ok(self.require_self_object()?.camera_interpolation_speed)
    }

    /// Lit un champ intégré ou une propriété personnalisée de cet objet.
    fn get_self_property(&self, py: Python<'_>, key: &str) -> PyResult<PyObject> {
        let object = self.require_self_object()?;
        let value = match key {
            "id" | "object_id" => serde_json::Value::String(object.id.to_string()),
            "name" => serde_json::Value::String(object.name.clone()),
            "type" | "object_type" => {
                serde_json::Value::String(object_type_name(&object.object_type))
            }
            "x" => serde_json::Value::Number(object.tile_pos.x.into()),
            "y" => serde_json::Value::Number(object.tile_pos.y.into()),
            "z" => serde_json::Value::Number(object.tile_pos.z.into()),
            "has_collisions" => serde_json::Value::Bool(object.has_collisions),
            "hidden" => serde_json::Value::Bool(object.hidden),
            "fit_texture_to_diamond" => serde_json::Value::Bool(object.fit_texture_to_diamond),
            "rotation" => serde_json::Value::String(format!("{:?}", object.rotation)),
            "layer" => serde_json::Value::String(format!("{:?}", object.layer)),
            "texture" | "asset_path" => object
                .asset_path
                .clone()
                .filter(|path| !path.is_empty())
                .or_else(|| {
                    object
                        .properties
                        .get("texture")
                        .and_then(PropValue::as_str)
                        .filter(|path| !path.is_empty())
                        .map(str::to_owned)
                })
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null),
            "script" => object
                .script
                .clone()
                .map(serde_json::Value::String)
                .unwrap_or(serde_json::Value::Null),
            "footprint_width" => serde_json::Value::Number(object.footprint.width.into()),
            "footprint_height" => serde_json::Value::Number(object.footprint.height.into()),
            "camera_zoom" => serde_json::Number::from_f64(object.camera_zoom as f64)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            "camera_interpolation" => serde_json::Value::Bool(object.camera_interpolation),
            "camera_interpolation_speed" => {
                serde_json::Number::from_f64(object.camera_interpolation_speed as f64)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            }
            custom => object
                .properties
                .get(custom)
                .map(prop_to_json)
                .unwrap_or(serde_json::Value::Null),
        };
        let json = serde_json::to_string(&value)
            .map_err(|error| pyo3::exceptions::PyRuntimeError::new_err(error.to_string()))?;
        let json_module = PyModule::import_bound(py, "json")?;
        Ok(json_module.getattr("loads")?.call1((json,))?.unbind())
    }

    /// Coordonnée X sur la grille (uniquement pour un script d'objet).
    #[getter]
    fn x(&self) -> PyResult<i32> {
        self.tile_position
            .map(|position| position.0)
            .ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "game.x est disponible uniquement dans un script attaché à un objet",
                )
            })
    }

    /// Coordonnée Y sur la grille (uniquement pour un script d'objet).
    #[getter]
    fn y(&self) -> PyResult<i32> {
        self.tile_position
            .map(|position| position.1)
            .ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "game.y est disponible uniquement dans un script attaché à un objet",
                )
            })
    }

    /// Coordonnée Z (étage) sur la grille (uniquement pour un script d'objet).
    #[getter]
    fn z(&self) -> PyResult<i32> {
        self.tile_position
            .map(|position| position.2)
            .ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err(
                    "game.z est disponible uniquement dans un script attaché à un objet",
                )
            })
    }

    fn move_self(&self, x: i32, y: i32) -> PyResult<()> {
        let object_id = self.object_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Ce script n'est attaché à aucun objet")
        })?;
        self.push(GameCommand::Move { object_id, x, y })
    }

    fn destroy_self(&self) -> PyResult<()> {
        let object_id = self.object_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Ce script n'est attaché à aucun objet")
        })?;
        self.push(GameCommand::Destroy { object_id })
    }

    fn set_self_property(&self, key: String, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let object_id = self.object_id.clone().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("Ce script n'est attaché à aucun objet")
        })?;
        self.push(GameCommand::SetProperty {
            object_id,
            key,
            value: py_any_to_json(value)?,
        })
    }

    /// Spawn un objet dans la scène.
    fn spawn(&self, name: String, object_type: String, x: i32, y: i32) -> PyResult<()> {
        self.push(GameCommand::Spawn {
            name,
            object_type,
            x,
            y,
        })
    }

    /// Détruit un objet.
    fn destroy(&self, object_id: String) -> PyResult<()> {
        self.push(GameCommand::Destroy { object_id })
    }

    /// Déplace un objet.
    fn move_object(&self, object_id: String, x: i32, y: i32) -> PyResult<()> {
        self.push(GameCommand::Move { object_id, x, y })
    }

    /// Définit une propriété d'un objet.
    fn set_property(
        &self,
        object_id: String,
        key: String,
        value: &Bound<'_, PyAny>,
    ) -> PyResult<()> {
        let json_value = py_any_to_json(value)?;
        self.push(GameCommand::SetProperty {
            object_id,
            key,
            value: json_value,
        })
    }

    /// Stocke une valeur partagée par le monde et tous les scripts d'objet.
    fn set_global(&self, key: String, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let json_value = py_any_to_json(value)?;
        self.globals
            .lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Verrou de variables globales inaccessible"))?
            .insert(key, json_value);
        Ok(())
    }

    /// Lit une valeur partagée ; renvoie None si la clé n'a pas été définie.
    fn get_global(&self, py: Python<'_>, key: &str) -> PyResult<PyObject> {
        let value = self.globals
            .lock()
            .map_err(|_| pyo3::exceptions::PyRuntimeError::new_err("Verrou de variables globales inaccessible"))?
            .get(key)
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        json_value_to_py(py, value)
    }

    /// Charge une autre scène du monde. Le nom doit correspondre exactement.
    fn load_scene(&self, name: String) -> PyResult<()> {
        self.push(GameCommand::LoadScene { name })
    }

    /// Quitte la scène active et revient à la scène précédemment chargée.
    fn unload_scene(&self) -> PyResult<()> {
        self.push(GameCommand::UnloadScene)
    }

    fn transfer_object(
        &self,
        object_id: String,
        scene_name: String,
        x: i32,
        y: i32,
    ) -> PyResult<()> {
        self.push(GameCommand::TransferObject {
            object_id,
            scene_name,
            x,
            y,
        })
    }

    /// Émet un message de log depuis le script.
    fn log(&self, message: String) -> PyResult<()> {
        self.push(GameCommand::Log {
            source: self.script_source.clone(),
            level: LogLevel::Info,
            message,
        })
    }

    fn log_warning(&self, message: String) -> PyResult<()> {
        self.push(GameCommand::Log {
            source: self.script_source.clone(),
            level: LogLevel::Warning,
            message,
        })
    }

    fn log_error(&self, message: String) -> PyResult<()> {
        self.push(GameCommand::Log {
            source: self.script_source.clone(),
            level: LogLevel::Error,
            message,
        })
    }
}

fn object_type_name(object_type: &world::object::ObjectType) -> String {
    match object_type {
        world::object::ObjectType::Camera => "Camera".to_owned(),
        _ => object_type.to_string(),
    }
}

fn object_to_json_value(object: &SceneObject, key: &str) -> serde_json::Value {
    match key {
        "id" | "object_id" => serde_json::Value::String(object.id.to_string()),
        "name" => serde_json::Value::String(object.name.clone()),
        "type" | "object_type" => serde_json::Value::String(object_type_name(&object.object_type)),
        "x" => serde_json::Value::Number(object.tile_pos.x.into()),
        "y" => serde_json::Value::Number(object.tile_pos.y.into()),
        "z" => serde_json::Value::Number(object.tile_pos.z.into()),
        "has_collisions" => serde_json::Value::Bool(object.has_collisions),
        "hidden" => serde_json::Value::Bool(object.hidden),
        "fit_texture_to_diamond" => serde_json::Value::Bool(object.fit_texture_to_diamond),
        "rotation" => serde_json::Value::String(format!("{:?}", object.rotation)),
        "layer" => serde_json::Value::String(format!("{:?}", object.layer)),
        "texture" | "asset_path" => object
            .asset_path
            .clone()
            .filter(|path| !path.is_empty())
            .or_else(|| {
                object
                    .properties
                    .get("texture")
                    .and_then(PropValue::as_str)
                    .filter(|path| !path.is_empty())
                    .map(str::to_owned)
            })
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
        "script" => object
            .script
            .clone()
            .map(serde_json::Value::String)
            .unwrap_or(serde_json::Value::Null),
        "footprint_width" => serde_json::Value::Number(object.footprint.width.into()),
        "footprint_height" => serde_json::Value::Number(object.footprint.height.into()),
        "camera_zoom" => serde_json::Number::from_f64(object.camera_zoom as f64)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        "camera_interpolation" => serde_json::Value::Bool(object.camera_interpolation),
        "camera_interpolation_speed" => {
            serde_json::Number::from_f64(object.camera_interpolation_speed as f64)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null)
        }
        custom => object
            .properties
            .get(custom)
            .map(prop_to_json)
            .unwrap_or(serde_json::Value::Null),
    }
}

fn json_value_to_py(py: Python<'_>, value: serde_json::Value) -> PyResult<PyObject> {
    let json = serde_json::to_string(&value)
        .map_err(|error| pyo3::exceptions::PyRuntimeError::new_err(error.to_string()))?;
    let json_module = PyModule::import_bound(py, "json")?;
    Ok(json_module.getattr("loads")?.call1((json,))?.unbind())
}

fn prop_to_json(value: &PropValue) -> serde_json::Value {
    match value {
        PropValue::Bool(value) => serde_json::Value::Bool(*value),
        PropValue::Int(value) => serde_json::Value::Number((*value).into()),
        PropValue::Float(value) => serde_json::Number::from_f64(*value)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        PropValue::Str(value) => serde_json::Value::String(value.clone()),
        PropValue::List(values) => {
            serde_json::Value::Array(values.iter().map(prop_to_json).collect())
        }
    }
}

/// Convertit un objet Python basique en `serde_json::Value`.
fn py_any_to_json(value: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    if value.is_none() {
        return Ok(serde_json::Value::Null);
    }
    if let Ok(b) = value.extract::<bool>() {
        return Ok(serde_json::Value::Bool(b));
    }
    if let Ok(i) = value.extract::<i64>() {
        return Ok(serde_json::Value::Number(i.into()));
    }
    if let Ok(f) = value.extract::<f64>() {
        let n = serde_json::Number::from_f64(f).unwrap_or_else(|| serde_json::Number::from(0));
        return Ok(serde_json::Value::Number(n));
    }
    if let Ok(s) = value.extract::<String>() {
        return Ok(serde_json::Value::String(s));
    }
    if let Ok(list) = value.downcast::<pyo3::types::PyList>() {
        let items: PyResult<Vec<_>> = list.iter().map(|item| py_any_to_json(&item)).collect();
        return Ok(serde_json::Value::Array(items?));
    }
    if let Ok(dict) = value.downcast::<pyo3::types::PyDict>() {
        let mut map = serde_json::Map::new();
        for (k, v) in dict.iter() {
            let key = k.extract::<String>()?;
            map.insert(key, py_any_to_json(&v)?);
        }
        return Ok(serde_json::Value::Object(map));
    }
    let type_name = value
        .get_type()
        .name()
        .map(|n| n.to_string())
        .unwrap_or_else(|_| "inconnu".to_string());
    Err(pyo3::exceptions::PyTypeError::new_err(format!(
        "Type Python non supporté dans l'API moteur : {type_name}"
    )))
}
