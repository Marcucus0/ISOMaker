1. tu check si la documentation est a jour (architecture, progression, python_api)
2. tu check si tout le code dans le fichier .vscode (pour l'extention) est bon
3. tu compiles et vérifies les erreurs/avertissements avec :
   - `cargo check --workspace`
   - `cargo build --release -p editor`
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
