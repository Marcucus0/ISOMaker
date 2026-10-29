use scripting::{ScriptRuntime, GameCommand};
use std::path::Path;

#[test]
fn test_python_script_execution() {
    let mut runtime = ScriptRuntime::new();
    let script_path = Path::new("../../python/game_scripts/main.py");
    assert!(script_path.exists(), "Le fichier de script doit exister");

    // 1. Charge le script
    runtime.load_script(script_path).expect("Le chargement du script Python doit réussir");

    // 2. Exécute on_start
    let start_cmds = runtime.call_start().expect("call_start doit réussir");
    assert!(!start_cmds.is_empty(), "on_start doit émettre des commandes");

    // Vérifie les commandes émises par main.py
    let has_log = start_cmds.iter().any(|cmd| matches!(cmd, GameCommand::Log { message, .. } if message.contains("Simulation démarrée")));
    assert!(has_log, "Le log de démarrage doit être présent");

    let has_global = start_cmds.iter().any(|cmd| matches!(cmd, GameCommand::SetGlobal { key, .. } if key == "money"));
    assert!(has_global, "set_global('money') doit être émis");

    let spawned_count = start_cmds.iter().filter(|cmd| matches!(cmd, GameCommand::Spawn { .. })).count();
    assert_eq!(spawned_count, 3, "main.py doit avoir spawn 3 objets");

    // 3. Exécute on_update
    let update_cmds = runtime.call_update(0.016).expect("call_update doit réussir");
    assert!(update_cmds.is_empty(), "on_update par défaut ne doit pas émettre de commande");

    // 4. Exécute on_stop
    let stop_cmds = runtime.call_stop().expect("call_stop doit réussir");
    let has_stop_log = stop_cmds.iter().any(|cmd| matches!(cmd, GameCommand::Log { message, .. } if message.contains("Simulation arrêtée")));
    assert!(has_stop_log, "Le log d'arrêt doit être présent");
}

