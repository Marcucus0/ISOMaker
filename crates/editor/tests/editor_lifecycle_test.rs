use editor::EditorApp;
use editor::EditorState;
use engine::game_loop::EngineMode;
use std::{fs, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};
use world::{coords::TilePos, object::{ObjectType, Rotation4}};

struct TemporaryScript(PathBuf);

impl TemporaryScript {
    fn new(source: &str) -> Self {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("isomaker_editor_script_{}_{}.py", std::process::id(), nonce));
        fs::write(&path, source).expect("write temporary Python script");
        Self(path)
    }
}

impl Drop for TemporaryScript {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[test]
fn test_editor_full_lifecycle() {
    let mut app = EditorApp::new();

    // 1. État initial de l'éditeur
    assert_eq!(app.state.game_loop.mode, EngineMode::Editor);
    let initial_objects_count = app.state.active_scene().objects.len();

    // 2. Ajout d'un objet éditeur
    let test_obj_id = app.state.active_scene_mut().spawn(
        "Tour de garde",
        world::object::ObjectType::Building,
        TilePos::new(10, 10),
    );
    assert_eq!(app.state.active_scene().objects.len(), initial_objects_count + 1);

    // 3. Sauvegarde de la scène
    let temp_save_path = std::env::temp_dir().join("test_scene_save.isomaker");
    app.save_scene(&temp_save_path).expect("La sauvegarde doit réussir");
    assert!(temp_save_path.exists(), "Le fichier sauvegardé doit exister");

    // 4. Chargement d'un script Python réel
    let script_path = Path::new("../../python/game_scripts/main.py");
    if script_path.exists() {
        app.load_script(script_path).expect("Le chargement du script doit réussir");
    }

    // 5. Lancement de la simulation (Play)
    app.state.play();
    assert_eq!(app.state.game_loop.mode, EngineMode::Playing);

    if script_path.exists() {
        // main.py spawne 3 objets à on_start
        assert_eq!(
            app.state.active_scene().objects.len(),
            initial_objects_count + 1 + 3,
            "Les objets spawnés par le script Python doivent être présents dans la scène active"
        );
    }

    // 6. Simulation de frames (Update)
    for _ in 0..10 {
        app.state.update(0.016);
    }
    assert_eq!(app.state.game_loop.mode, EngineMode::Playing);

    // 7. Arrêt de la simulation (Stop) -> Restauration de l'état éditeur
    app.state.stop();
    assert_eq!(app.state.game_loop.mode, EngineMode::Editor);

    // La scène doit être restaurée dans son état AVANT le Play
    assert_eq!(
        app.state.active_scene().objects.len(),
        initial_objects_count + 1,
        "La scène doit être restaurée sans les objets temporaires du jeu"
    );
    assert!(
        app.state.active_scene().get_object(test_obj_id).is_some(),
        "L'objet créé avant le Play doit toujours être présent"
    );

    // 8. Rechargement de la scène sauvegardée
    app.load_scene(&temp_save_path).expect("Le rechargement doit réussir");
    assert_eq!(app.state.active_scene().objects.len(), initial_objects_count + 1);

    // Nettoyage
    let _ = std::fs::remove_file(temp_save_path);
}

#[test]
fn object_launch_runs_again_after_toolbar_stop_transition() {
    let mut state = EditorState::new();
    let object_id = state.active_scene_mut().spawn(
        "Original",
        ObjectType::Character,
        TilePos::with_z(3, 4, 2),
    );
    state.active_scene_mut().get_object_mut(object_id).unwrap().hidden = true;
    let object_id_text = object_id.to_string();
    let script = TemporaryScript::new(
        "def _on_game_launch(game):\n    game.set_self_property('hidden', False)\n    game.set_self_property('has_collisions', False)\n    game.set_self_property('fit_texture_to_diamond', True)\n    game.set_self_property('name', 'Launched')\n    game.set_self_property('x', 5)\n    game.set_self_property('y', 6)\n    game.set_self_property('z', 8)\n    game.set_self_property('rotation', 'East')\n    game.set_self_property('layer', 'Foreground')\n    game.set_self_property('texture', 'assets/character.png')\n    game.set_self_property('script', 'scripts/next.py')\n    game.set_self_property('score', 42)\n    game.set_self_property('footprint_width', 2)\n    game.log('object launch regression marker')\n",
    );

    state.script_runtime.load_object_script(&script.0, object_id_text.clone()).unwrap();
    state.play();
    {
        let object = state.active_scene().get_object(object_id).unwrap();
        assert!(!object.hidden);
        assert!(!object.has_collisions);
        assert!(object.fit_texture_to_diamond);
        assert_eq!(object.name, "Launched");
        assert_eq!(object.tile_pos, TilePos::with_z(5, 6, 8));
        assert_eq!(object.rotation, Rotation4::East);
        assert_eq!(object.layer, world::Layer::Foreground);
        assert_eq!(object.asset_path.as_deref(), Some("assets/character.png"));
        assert_eq!(object.script.as_deref(), Some("scripts/next.py"));
        assert_eq!(object.footprint.width, 2);
    }

    // Le bouton Stop met le mode en Editor avant d'appeler EditorState::stop,
    // comme le fait la barre d'outils de l'application.
    state.game_loop.stop();
    state.stop();
    let restored = state.active_scene().get_object(object_id).unwrap();
    assert!(restored.hidden, "Stop doit restaurer l'objet pré-Play");
    assert_eq!(restored.name, "Original");
    assert_eq!(restored.tile_pos, TilePos::with_z(3, 4, 2));

    state.script_runtime.load_object_script(&script.0, object_id_text).unwrap();
    state.play();
    let relaunched = state.active_scene().get_object(object_id).unwrap();
    assert!(!relaunched.hidden, "_on_game_launch doit se relancer après Stop");
    assert_eq!(relaunched.name, "Launched");
    assert!(state.log_messages.iter().any(|entry| {
        entry.message == "object launch regression marker" && entry.repetitions == 2
    }));
    state.stop();
}

