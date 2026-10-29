"""Contract tests for the Python scripts shipped with the engine."""

from __future__ import annotations

import importlib.util
import unittest
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = ROOT / "python" / "game_scripts" / "main.py"
SPEC = importlib.util.spec_from_file_location("isomaker_sample_game", SCRIPT_PATH)
assert SPEC is not None and SPEC.loader is not None
SAMPLE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SAMPLE)


class FakeGameContext:
    def __init__(self) -> None:
        self.logs: list[str] = []
        self.globals: dict[str, Any] = {}
        self.spawned: list[tuple[str, str, int, int]] = []

    def log(self, message: str) -> None:
        self.logs.append(message)

    def set_global(self, key: str, value: Any) -> None:
        self.globals[key] = value

    def spawn(self, name: str, object_type: str, x: int, y: int) -> None:
        self.spawned.append((name, object_type, x, y))


class ShippedGameScriptTests(unittest.TestCase):
    def test_start_hook_logs_initializes_state_and_spawns_entities(self) -> None:
        game = FakeGameContext()

        SAMPLE.on_start(game)

        self.assertIn("=== Simulation démarrée ===", game.logs)
        self.assertEqual(game.globals["money"], 10000)
        self.assertEqual(
            game.spawned,
            [
                ("Maison", "Building", 3, 2),
                ("Entrepôt", "Building", -2, 4),
                ("Villageois", "Character", 0, 0),
            ],
        )

    def test_update_hook_accepts_frame_delta_without_emitting_commands(self) -> None:
        game = FakeGameContext()

        SAMPLE.on_update(game, 1 / 60)

        self.assertEqual(game.logs, [])
        self.assertEqual(game.spawned, [])
        self.assertEqual(game.globals, {})

    def test_stop_hook_logs_shutdown(self) -> None:
        game = FakeGameContext()

        SAMPLE.on_stop(game)

        self.assertEqual(game.logs, ["=== Simulation arrêtée ==="])


if __name__ == "__main__":
    unittest.main()
