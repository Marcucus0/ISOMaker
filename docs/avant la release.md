0. tu vas check tout le code voir si du code qu'on utilise pas est toujours là
1. tu compiles et vérifies les erreurs/avertissements avec :
   - `cargo check --workspace`
   - `cargo build --release -p editor`
2. tu check si la documentation est a jour (architecture, progression, python_api)
3. tu check si tout le code dans le fichier .vscode (pour l'extention) est bon et cohérent / a jour

4. tu fais les tests Rust et Python :
   - `cargo test --workspace` pour les tests Rust ;
   - `python -m unittest discover -s python/tests -v` pour les tests Python.
   Les tests Rust couvrent notamment la grille, le remplissage connecté, la
   persistance multi-scène/caméra, l'API objet Python, les logs, et les assets.
   Les tests Python vérifient le script d'exemple et le contrat du stub Pylance.
   Sous Windows, lance les tests dans un répertoire `CARGO_TARGET_DIR` distinct
   si `target/debug/editor.exe` est verrouillé par une instance ouverte.

Vérification effectuée le 29 octobre 2026 :

- Documentation contrôlée : `ARCHITECTURE.md`, `PROGRESSION.md`, `PYTHON_API.md`
  et README mis à jour ; le stub `python/engine_api/__init__.pyi` reste déclaré
  dans `.vscode/settings.json`.
- `cargo check --workspace` : réussi sans erreur ni avertissement.
- `cargo build --release -p editor` : réussi.
- `cargo test --workspace` dans `target/release-check` : 30 tests réussis.
- `python -m unittest discover -s python/tests -v` : 5 tests réussis.
- Export du projet `test2` : ZIP créé et vérifié, intégrité confirmée.


enfin : 
tu vas faire un changelog, le git add, faire le commit et le push

Vérification effectuée pour la release 0.2.0 le 30 octobre 2026 :

- Documentation contrôlée : `ARCHITECTURE.md`, `PROGRESSION.md`, `PYTHON_API.md` et README actualisés.
- Paramètres VS Code et snippets contrôlés ; le stub API reste référencé dans `.vscode/settings.json` et les snippets sont valides en JSON.
- `cargo check --workspace` : réussi sans avertissement.
- `cargo build --release -p editor` : réussi.
- `cargo test --workspace` dans `target/release-check` : 29 tests réussis.
- `python -m unittest discover -s python/tests -v` : 5 tests réussis.

Vérification effectuée pour la release 0.3.0 le 30 octobre 2026 :

- Documentation contrôlée : `ARCHITECTURE.md`, `PROGRESSION.md`, `PYTHON_API.md` et README.
- Paramètres VS Code et snippets contrôlés ; les deux fichiers JSON sont valides.
- `cargo check --workspace` : réussi sans avertissement.
- `cargo build --release -p editor` : réussi.
- `cargo test --workspace` dans `target/release-check` : 29 tests réussis.
- `python -m unittest discover -s python/tests -v` : 5 tests réussis.

Vérification effectuée pour la release 0.4.0 le 31 octobre 2026 :

- Documentation contrôlée : `ARCHITECTURE.md`, `PROGRESSION.md`, `PYTHON_API.md`,
  `PROMPT BASE.md` et README actualisés avec le HUD, les clics sur objets, les
  variables globales de session et les accès inter-scènes.
- Paramètres et extensions recommandées VS Code contrôlés ; `settings.json`,
  `extensions.json` et `engine-api.code-snippets` valides en JSON.
- `cargo check --workspace` : réussi sans avertissement.
- `cargo build --release -p editor` dans `target/release-check` : réussi.
- `cargo test --workspace` dans `target/release-check` : 29 tests réussis.
- `python -m unittest discover -s python/tests -v` : 5 tests réussis.
