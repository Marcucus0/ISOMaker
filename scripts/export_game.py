#!/usr/bin/env python3
"""Build a portable Windows ZIP containing the ISOMaker player and a project."""

from __future__ import annotations

import argparse
import ast
import json
import os
import pathlib
import shutil
import subprocess
import sys
import tempfile
import zipfile

ROOT = pathlib.Path(__file__).resolve().parents[1]


def python_runtime_home(python_home: pathlib.Path | None) -> pathlib.Path:
    if python_home is not None:
        home = python_home.resolve(strict=True)
    else:
        python_exe = os.environ.get("PYO3_PYTHON") or sys.executable
        output = subprocess.check_output(
            [python_exe, "-c", "import sys; print(sys.base_prefix)"], text=True
        ).strip()
        home = pathlib.Path(output).resolve(strict=True)
    if not (home / "Lib").is_dir():
        raise SystemExit(f"Runtime Python invalide : dossier Lib absent dans {home}")
    if not list(home.glob("python3*.dll")) and not list(home.parent.glob("python3*.dll")):
        raise SystemExit(f"DLL Python introuvable dans {home}; utilisez --python-home")
    return home


def copy_python_runtime(home: pathlib.Path, destination: pathlib.Path) -> None:
    destination.mkdir(parents=True, exist_ok=True)
    def ignore_development_files(directory: str, names: list[str]) -> set[str]:
        ignored = {name for name in names if name == "__pycache__" or name.endswith(".pyc")}
        if pathlib.Path(directory).name == "Lib":
            ignored.update({"site-packages", "test", "tests", "tkinter", "turtledemo", "idlelib", "ensurepip", "venv"})
        return ignored

    shutil.copytree(
        home / "Lib", destination / "Lib", dirs_exist_ok=True,
        ignore=ignore_development_files,
    )
    dlls = home / "DLLs"
    if dlls.is_dir():
        shutil.copytree(dlls, destination / "DLLs", dirs_exist_ok=True)
    for archive in home.glob("python3*.zip"):
        shutil.copy2(archive, destination / archive.name)


def project_resource_references(data: dict) -> list[tuple[str, str]]:
    references: list[tuple[str, str]] = []

    def add(label: str, value: object) -> None:
        if isinstance(value, str) and value.strip():
            references.append((label, value))

    world = data.get("world", {})
    add("script du monde", world.get("script"))
    for scene in world.get("scenes", []):
        for obj in scene.get("objects", {}).values():
            if not isinstance(obj, dict):
                continue
            name = obj.get("name", "?")
            add(f"texture de l'objet {name}", obj.get("asset_path"))
            if not obj.get("asset_path"):
                texture = (obj.get("properties") or {}).get("texture")
                if isinstance(texture, dict):
                    texture = texture.get("Str")
                add(f"texture de l'objet {name}", texture)
            add(f"script de l'objet {name}", obj.get("script"))

    for tile in data.get("tile_definitions", []):
        name = tile.get("name", "?")
        add(f"texture de tuile {name}", tile.get("texture"))
        add(f"son de pas de tuile {name}", tile.get("footstep_sound"))
        for sound in tile.get("footstep_variants", []):
            add(f"variante audio de tuile {name}", sound)

    for object_type in data.get("object_types", []):
        defaults = object_type.get("defaults") or {}
        name = object_type.get("name", "?")
        add(f"texture par défaut du type {name}", defaults.get("texture"))
        add(f"script par défaut du type {name}", defaults.get("script"))
    return references


def validate_project_resources(package: pathlib.Path, project_name: str) -> None:
    project_file = package / project_name
    data = json.loads(project_file.read_text(encoding="utf-8"))
    package_root = package.resolve()
    missing: list[str] = []
    references = project_resource_references(data)
    tile_directory = package / pathlib.Path(project_name).stem / "tiles"
    if tile_directory.is_dir():
        for tile_file in tile_directory.glob("*.json"):
            tile_data = json.loads(tile_file.read_text(encoding="utf-8"))
            references.extend(
                project_resource_references({"tile_definitions": [tile_data]})
            )

    # Les sons lancés par game.audio.create sont référencés dans le script,
    # pas dans le JSON du projet. Vérifie les chemins littéraux détectables.
    script_paths = [stored for label, stored in references if "script" in label]
    for stored_script in script_paths:
        normalized_script = stored_script.replace("\\", "/").lstrip("/")
        script_file = (package / normalized_script).resolve()
        if not script_file.is_relative_to(package_root) or not script_file.is_file():
            continue
        try:
            syntax_tree = ast.parse(script_file.read_text(encoding="utf-8"))
        except (OSError, SyntaxError, UnicodeError):
            continue
        for node in ast.walk(syntax_tree):
            if not isinstance(node, ast.Call) or not isinstance(node.func, ast.Attribute):
                continue
            audio_create = (
                node.func.attr == "create"
                and isinstance(node.func.value, ast.Attribute)
                and node.func.value.attr == "audio"
            )
            if not audio_create:
                continue
            path_argument = node.args[1] if len(node.args) > 1 else next(
                (keyword.value for keyword in node.keywords if keyword.arg == "path"),
                None,
            )
            if isinstance(path_argument, ast.Constant) and isinstance(path_argument.value, str):
                references.append(
                    (
                        f"son créé dans {script_file.name} ligne {node.lineno}",
                        path_argument.value,
                    )
                )

    for label, stored in references:
        normalized = stored.replace("\\", "/")
        if normalized.startswith("/") and not pathlib.PureWindowsPath(normalized).drive:
            normalized = normalized.lstrip("/")
        requested = pathlib.Path(normalized)
        if requested.is_absolute():
            missing.append(f"{label}: chemin absolu non exportable ({stored})")
            continue
        resolved = (package / requested).resolve()
        if not resolved.is_relative_to(package_root):
            missing.append(f"{label}: chemin hors du projet ({stored})")
        elif not resolved.is_file():
            missing.append(f"{label}: fichier absent du projet ({stored})")
    if missing:
        raise RuntimeError(
            "Ressources manquantes ou non exportables :\n- " + "\n- ".join(missing)
        )


def export(project_file: pathlib.Path, output: pathlib.Path, python_home: pathlib.Path | None, player_binary: pathlib.Path | None = None) -> pathlib.Path:
    project_file = project_file.resolve(strict=True)
    if project_file.suffix.lower() != ".isomaker":
        raise SystemExit("Le projet doit être un fichier .isomaker")
    try:
        json.loads(project_file.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise SystemExit(f"Fichier projet illisible ou invalide : {error}") from error

    runtime_home = python_runtime_home(python_home)
    if player_binary is None:
        python_executable = runtime_home / ("python.exe" if os.name == "nt" else "bin/python3")
        if not python_executable.is_file():
            raise SystemExit(f"Interpreteur de compilation introuvable : {python_executable}")
        cargo = shutil.which("cargo")
        if cargo is None and os.name == "nt":
            candidate = pathlib.Path.home() / ".cargo" / "bin" / "cargo.exe"
            if candidate.is_file():
                cargo = str(candidate)
        if cargo is None:
            raise SystemExit("cargo est introuvable; ajoutez Rust au PATH ou verifiez l'installation de Cargo")

        build_environment = os.environ.copy()
        build_environment["PYO3_PYTHON"] = str(python_executable)
        subprocess.run(
            [cargo, "build", "--release", "-p", "editor", "--bin", "player"],
            cwd=ROOT,
            env=build_environment,
            check=True,
        )
        player_exe = ROOT / "target" / "release" / "player.exe"
    else:
        player_exe = player_binary.resolve(strict=True)
    if not player_exe.is_file():
        raise SystemExit(f"Exécutable player absent après compilation : {player_exe}")

    output = output.resolve()
    output.parent.mkdir(parents=True, exist_ok=True)
    stem = project_file.stem
    with tempfile.TemporaryDirectory(prefix="isomaker-export-") as temporary:
        package = pathlib.Path(temporary) / stem
        package.mkdir()
        shutil.copy2(player_exe, package / "player.exe")
        shutil.copy2(project_file, package / project_file.name)

        project_data = project_file.parent / stem
        if project_data.is_dir():
            shutil.copytree(project_data, package / stem)
        else:
            (package / stem).mkdir()

        copy_python_runtime(runtime_home, package / "python-runtime")
        python_dlls = list(runtime_home.glob("python3*.dll")) or list(runtime_home.parent.glob("python3*.dll"))
        for dll in python_dlls:
            shutil.copy2(dll, package / dll.name)
        for pattern in ("vcruntime*.dll", "msvcp*.dll", "ucrtbase.dll"):
            runtime_dlls = list(runtime_home.glob(pattern)) or list(runtime_home.parent.glob(pattern))
            for dll in runtime_dlls:
                shutil.copy2(dll, package / dll.name)
        for license_name in ("LICENSE.txt", "LICENSE", "COPYING"):
            license_file = runtime_home / license_name
            if not license_file.is_file():
                license_file = runtime_home.parent / license_name
            if license_file.is_file():
                shutil.copy2(license_file, package / ("PYTHON-" + license_name))

        (package / "LISEZMOI.txt").write_text(
            "ISOMaker — Jeu exporté\n\n"
            "Double-cliquez sur player.exe pour lancer le jeu.\n"
            "Le fichier projet et son dossier associé doivent rester à côté de player.exe.\n"
            "Appuyez sur Échap pour quitter.\n",
            encoding="utf-8",
        )
        validate_project_resources(package, project_file.name)
        temporary_zip = output.with_suffix(output.suffix + ".tmp")
        try:
            with zipfile.ZipFile(temporary_zip, "w", zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
                for path in package.rglob("*"):
                    if path.is_file():
                        archive.write(path, path.relative_to(package.parent))
            with zipfile.ZipFile(temporary_zip, "r") as archive:
                names = set(archive.namelist())
                required_files = {
                    f"{stem}/player.exe",
                    f"{stem}/{project_file.name}",
                    f"{stem}/python-runtime/Lib/os.py",
                }
                missing = sorted(required_files - names)
                if missing:
                    raise RuntimeError("Fichiers requis absents de l'archive : " + ", ".join(missing))
                damaged = archive.testzip()
                if damaged is not None:
                    raise RuntimeError(f"Archive ZIP corrompue à l'entrée {damaged}")
            temporary_zip.replace(output)
        finally:
            temporary_zip.unlink(missing_ok=True)
    return output


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("project", type=pathlib.Path, help="Fichier projet .isomaker")
    parser.add_argument(
        "-o", "--output", type=pathlib.Path,
        help="Chemin du ZIP produit (par défaut : dist/<projet>-windows.zip)",
    )
    parser.add_argument(
        "--python-home", type=pathlib.Path,
        help="Racine du Python partagé par PyO3 (par défaut : PYO3_PYTHON ou Python actif)",
    )
    parser.add_argument("--player-bin", type=pathlib.Path, help="Lecteur deja compile (export sans Cargo)")
    args = parser.parse_args()
    output = args.output or (ROOT / "dist" / f"{args.project.stem}-windows.zip")
    result = export(args.project, output, args.python_home, args.player_bin)
    print(f"Export créé : {result}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
