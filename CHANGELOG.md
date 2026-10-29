# Changelog

Les changements notables du projet sont consignés ici.

## Non publié

### Ajouté

- Recherche d'objets par nom ou UUID dans l'API Python, lecture de leurs
  instantanés et accès aux propriétés caméra depuis les scripts.
- Préférences de caméra interpolée, couleur de grille réglable et sauvegardée,
  confirmation clavier de suppression et remplacement connecté de tuiles.
- Couverture de tests Rust pour le remplissage, les assets, le cycle de vie des
  scripts et la sérialisation ; tests Python du stub d'autocomplétion.

### Corrigé

- Références de documentation concernant les préférences de grille et la
  recherche d'objets Python.
- Test de relancement d'un script objet, qui tient compte du nom d'objet présent
  dans le préfixe des logs après renommage.

### Vérification

- `cargo check --workspace` — réussi sans avertissement.
- `cargo build --release -p editor` — réussi.
- `cargo test --workspace` — 24 tests réussis.
- `python -m unittest discover -s python/tests -v` — 5 tests réussis.
