//! Hiérarchie persistante Projet → Monde → Scènes.

use serde::{Deserialize, Serialize};

use crate::scene::Scene;

#[derive(Debug, Serialize, Deserialize)]
pub struct World {
    pub name: String,
    /// Script Python global associé au monde, relatif au dossier du projet.
    #[serde(default)]
    pub script: Option<String>,
    pub scenes: Vec<Scene>,
    pub active_scene_index: usize,
}

impl World {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            script: None,
            scenes: vec![Scene::new("Scène principale")],
            active_scene_index: 0,
        }
    }

    pub fn active_scene(&self) -> &Scene {
        self.scenes.get(self.active_scene_index)
            .expect("World must contain a valid active scene")
    }

    pub fn active_scene_mut(&mut self) -> &mut Scene {
        self.scenes.get_mut(self.active_scene_index)
            .expect("World must contain a valid active scene")
    }

    pub fn add_scene(&mut self, name: impl Into<String>) -> usize {
        self.scenes.push(Scene::new(name));
        let index = self.scenes.len() - 1;
        self.active_scene_index = index;
        index
    }

    pub fn select_scene(&mut self, index: usize) -> bool {
        if index >= self.scenes.len() {
            return false;
        }
        self.active_scene_index = index;
        true
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub format_version: u32,
    pub name: String,
    /// A project owns exactly one world; scenes are collected by that world.
    pub world: World,
}

impl Project {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            format_version: 1,
            name: name.into(),
            world: World::new("Monde principal"),
        }
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        let project: Self = serde_json::from_str(json)?;
        anyhow::ensure!(project.format_version == 1, "Version de projet non prise en charge");
        anyhow::ensure!(!project.world.scenes.is_empty(), "Le monde doit contenir au moins une scène");
        anyhow::ensure!(
            project.world.active_scene_index < project.world.scenes.len(),
            "L'indice de la scène active est invalide"
        );
        Ok(project)
    }
}
