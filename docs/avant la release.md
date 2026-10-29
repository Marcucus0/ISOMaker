1. tu check si la documentation est a jour (architecture, progression, python_api)
2. tu check si tout le code dans le fichier .vscode (pour l'extention) est bon
3. tu compile et regarde si il y a des erreurs / warnings
4. tu fais les tests Rust et Python :
   - `cargo test --workspace` pour les tests Rust ;
   - `python -m unittest discover -s python/tests -v` pour les tests Python.
   Sous Windows, ferme l'editeur avant `cargo test --workspace` si
   `target/debug/editor.exe` est verrouille.

enfin : 
tu vas faire un changelog, faire le commit et le push
