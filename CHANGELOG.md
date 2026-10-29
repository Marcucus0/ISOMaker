# Changelog

Les changements notables du projet sont consignés ici.

## 0.1.0 — 2026-10-29

### Ajouté

- Lecteur autonome `player` et export Windows en ZIP portable depuis **Fichier
  → Exporter le jeu** ; l'export inclut le runtime Python standard, valide
  l'intégrité de l'archive et consigne succès ou erreurs détaillées.
- Statistiques générées dans le README et actualisées par workflow GitHub.
- Recherche d'objets par nom ou UUID dans l'API Python, lecture de leurs
  instantanés et accès aux propriétés caméra depuis les scripts.
- Palette de tuiles propres à chaque projet, avec couleur, texture PNG/JPEG/GIF
  animé, collision et rotation pseudo-aléatoire par case.
- Collisions de grille pendant les déplacements de scripts, sélection et copie
  des lignes du journal, caméra interpolée et préférences d'éditeur.
- Gestion des scènes et fichiers du projet, outils isométriques, prefabs,
  runtime Python pour les scripts du monde et des objets.
- Couleur de grille réglable et sauvegardée, confirmation clavier de suppression
  et remplacement connecté de tuiles.
- Couverture de tests Rust pour le remplissage, les assets, le cycle de vie des
  scripts et la sérialisation ; tests Python du stub d'autocomplétion.

### Corrigé

- Références de documentation concernant les préférences de grille et la
  recherche d'objets Python.
- Documentation Python actualisée sur la lecture des objets et les collisions.
- Documentation mise à jour pour le lecteur, l'export et ses limites concernant
  les dépendances Python tierces.
- Test de relancement d'un script objet, qui tient compte du nom d'objet présent
  dans le préfixe des logs après renommage.

### Vérification

- `cargo check --workspace` — réussi sans avertissement.
- `cargo build --release -p editor` — réussi.
- `cargo test --workspace` — 30 tests réussis.
- `python -m unittest discover -s python/tests -v` — 5 tests réussis.
