use engine::{InputState, Key};
use scripting::{GameCommand, ScriptRuntime};
use std::{fs, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};
use world::{coords::TilePos, object::{ObjectType, PropValue, SceneObject}};

struct TemporaryScript(PathBuf);

impl TemporaryScript {
    fn new(source: &str) -> Self {
        let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let path = std::env::temp_dir().join(format!("isomaker_object_api_{}_{}.py", std::process::id(), nonce));
        fs::write(&path, source).expect("write temporary Python script");
        Self(path)
    }
}

impl Drop for TemporaryScript {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn test_object() -> SceneObject {
    let mut object = SceneObject::new("Guard", ObjectType::Character, TilePos::with_z(4, 7, 2));
    object.has_collisions = true;
    object.hidden = true;
    object.fit_texture_to_diamond = false;
    object.rotation = world::object::Rotation4::South;
    object.layer = world::Layer::Foreground;
    object.script = Some("scripts/guard.py".to_owned());
    object.set_prop("health", PropValue::Int(75));
    object
}

#[test]
fn object_hooks_read_all_context_fields_and_emit_mutation_commands() {
    let object = test_object();
    let object_id = object.id.to_string();
    let source = r#"
def _on_game_launch(game):
    assert game.object_id is not None
    assert game.name == "Guard"
    assert game.object_type == "Character"
    assert game.x == 4 and game.y == 7 and game.z == 2
    assert game.has_collisions is True
    assert game.hidden is True
    assert game.fit_texture_to_diamond is False
    assert game.rotation == "South"
    assert game.layer == "Foreground"
    assert game.texture is None
    assert game.script == "scripts/guard.py"
    assert game.get_self_property("health") == 75
    assert game.get_self_property("missing") is None
    assert game.get_self_property("footprint_width") == 1

    game.set_self_property("hidden", False)
    game.set_self_property("has_collisions", False)
    game.set_self_property("name", "Sentinel")
    game.set_self_property("rotation", "East")
    game.set_self_property("layer", "Objects")
    game.set_self_property("texture", "textures/sentinel.png")
    game.set_self_property("script", None)
    game.set_self_property("health", 80)
    game.set_self_property("footprint_width", 2)
    game.log("object launch ok")

def _on_update(game, delta_time):
    assert abs(delta_time - 0.016) < 0.000001
    if game.key_just_pressed(game.keys.Z):
        game.move_self(game.x, game.y - 1)
        game.set_self_property("z", game.z + 1)
"#;

    let temporary = TemporaryScript::new(source);
    let mut runtime = ScriptRuntime::new();
    runtime.load_object_script(&temporary.0, object_id.clone()).expect("load object script");

    let launch = runtime.call_object_launch(std::slice::from_ref(&object)).expect("call object launch");
    assert!(launch.iter().any(|command| matches!(command,
        GameCommand::SetProperty { object_id: id, key, value }
        if id == &object_id && key == "hidden" && value == false
    )));
    assert!(launch.iter().any(|command| matches!(command,
        GameCommand::SetProperty { key, value, .. }
        if key == "texture" && value == "textures/sentinel.png"
    )));
    assert!(launch.iter().any(|command| matches!(command,
        GameCommand::SetProperty { key, value, .. }
        if key == "footprint_width" && value == 2
    )));
    assert!(launch.iter().any(|command| matches!(command,
        GameCommand::Log { message, .. } if message == "object launch ok"
    )));

    let mut input = InputState::new();
    input.key_down(Key::Z);
    let mut moved_object = object;
    moved_object.tile_pos = TilePos::with_z(4, 7, 2);
    let update = runtime.call_object_update(0.016, &[moved_object], &input).expect("call object update");
    assert!(update.iter().any(|command| matches!(command,
        GameCommand::Move { object_id: id, x: 4, y: 6 } if id == &object_id
    )));
    assert!(update.iter().any(|command| matches!(command,
        GameCommand::SetProperty { object_id: id, key, value }
        if id == &object_id && key == "z" && value == 3
    )));
}

#[test]
fn object_hook_is_skipped_when_its_object_is_not_in_the_active_scene() {
    let object = test_object();
    let temporary = TemporaryScript::new("def _on_game_launch(game):\n    game.log('should not run')\n");
    let mut runtime = ScriptRuntime::new();
    runtime.load_object_script(&temporary.0, object.id.to_string()).expect("load object script");

    let commands = runtime.call_object_launch(&[]).expect("skip removed object's script");
    assert!(commands.is_empty());
}
