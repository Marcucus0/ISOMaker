# ISOMaker

ISOMaker est un éditeur et moteur de jeu 2D isométrique écrit en Rust. Il permet de construire des scènes, de gérer les tuiles et les objets d’un projet, puis d’exécuter le monde avec des scripts Python.

[![Workflow README](https://img.shields.io/github/actions/workflow/status/Marcucus0/ISOMaker/update-readme.yml?branch=main&label=actualisation%20du%20README)](https://github.com/Marcucus0/ISOMaker/actions/workflows/update-readme.yml) [![Dernière release GitHub](https://img.shields.io/github/v/release/Marcucus0/ISOMaker?label=release)](https://github.com/Marcucus0/ISOMaker/releases)

## État du projet

<!-- README:GENERATED:START -->
**Version actuelle :** `0.2.0`<br>
**Dernière release :** `v0.2.0` · 2026-10-30<br>
**Commits du dépôt :** 9<br>
**Dernier commit :** `ee5545a (2026-10-30)`<br>
**Code et documentation :** 34 fichiers Rust · 10 306 lignes Rust · 5 fichiers Python · 5 documents Markdown

_Mis à jour le 2026-10-30._
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

## Lancer un projet dans le lecteur

Le lecteur masque les outils de l'éditeur et lance directement la scène active
en mode jeu. Échap ferme la fenêtre.

```powershell
cargo run -p editor --bin player -- chemin/vers/jeu.isomaker
```

Depuis l'éditeur, choisir **Fichier → Exporter le jeu**. L'éditeur sauvegarde le
projet, demande où écrire le ZIP puis lance la compilation en arrière-plan.
Le succès ou le détail des erreurs apparaît dans les logs ; l'archive est
vérifiée avant d'être signalée comme terminée. L'export peut aussi être lancé
directement (Cargo et Python doivent être installés) :

```powershell
python scripts/export_game.py chemin/vers/jeu.isomaker
```

L'archive apparaît dans `dist/`. Elle contient le lecteur, le projet, son
dossier d'assets/scripts et le runtime Python standard ; les modules Python
tiers utilisés par les scripts ne sont pas ajoutés automatiquement.

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
