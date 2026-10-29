"""Contract tests for the Python autocomplete stub shipped with the editor."""

from __future__ import annotations

import ast
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
STUB_PATH = ROOT / "python" / "engine_api" / "__init__.pyi"


class EngineApiStubTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.module = ast.parse(STUB_PATH.read_text(encoding="utf-8"), filename=str(STUB_PATH))
        cls.classes = {
            node.name: node
            for node in cls.module.body
            if isinstance(node, ast.ClassDef)
        }

    def test_other_object_lookup_is_declared_as_optional(self) -> None:
        game_context = self.classes["GameContext"]
        methods = {
            node.name: node
            for node in game_context.body
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef))
        }

        self.assertEqual(ast.unparse(methods["get_object"].returns), "GameObject | None")
        self.assertEqual(ast.unparse(methods["find_object"].returns), "GameObject | None")

    def test_scene_object_and_camera_fields_are_declared(self) -> None:
        game_object = self.classes["GameObject"]
        fields = {
            node.target.id
            for node in game_object.body
            if isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name)
        }

        self.assertTrue({"id", "name", "x", "y", "z"}.issubset(fields))
        self.assertTrue(
            {"camera_zoom", "camera_interpolation", "camera_interpolation_speed"}.issubset(fields)
        )


if __name__ == "__main__":
    unittest.main()
