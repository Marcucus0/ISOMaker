# ISOMaker

ISOMaker est un éditeur et moteur de jeu 2D isométrique écrit en Rust. Il permet de construire des scènes, de gérer les tuiles et les objets d’un projet, puis d’exécuter le monde avec des scripts Python.

[![Workflow README](https://img.shields.io/github/actions/workflow/status/Marcucus0/ISOMaker/update-readme.yml?branch=main&label=actualisation%20du%20README)](https://github.com/Marcucus0/ISOMaker/actions/workflows/update-readme.yml) [![Dernière release GitHub](https://img.shields.io/github/v/release/Marcucus0/ISOMaker?label=release)](https://github.com/Marcucus0/ISOMaker/releases)

## État du projet

<!-- README:GENERATED:START -->
**Version actuelle :** `0.1.0`  
**Dernière release :** `0.1.0` · 2026-10-29 ([changelog](CHANGELOG.md); sans tag Git)  
**Commits du dépôt :** 6  
**Dernier commit :** `d8eb979 (2026-10-29)`  
**Code et documentation :** 34 fichiers Rust · 8 804 lignes Rust · 4 fichiers Python · 5 documents Markdown

_Mis à jour le 2026-10-29._
<!-- README:GENERATED:END -->

Les chiffres ci-dessus sont générés depuis le dépôt. Pour les recalculer localement :

```powershell
python scripts/update_readme.py
```

## Démarrer

Prérequis : une installation récente de Rust (stable) et Python 3 pour les scripts et tests Python.

```powershell
cargo check --workspace
cargo run -p editor
```

Construire une version optimisée :

```powershell
cargo build --release -p editor
```

## Tests

```powershell
cargo test --workspace
python -m unittest discover -s python/tests -v
```

## Documentation

- [Architecture](docs/ARCHITECTURE.md)
- [Progression du projet](docs/PROGRESSION.md)
- [API Python](docs/PYTHON_API.md)
- [Prompt de base et objectifs](docs/PROMPT%20BASE.md)
- [Checklist avant release](docs/avant%20la%20release.md)
- [Changelog](CHANGELOG.md)

## Technologies

Rust, `egui`/`wgpu` pour l’éditeur et le rendu, et Python embarqué via PyO3 pour la logique de jeu.

## Licence

Aucun fichier de licence n’est actuellement déclaré dans le dépôt.
