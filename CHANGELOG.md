# Changelog

## 0.4.0 — 2026-10-31

### Ajouté

- Interface HUD pilotable en Python : création de textes, neuf points d'ancrage,
  et mises à jour séparées du contenu, de la position, de la taille et de la couleur.
- Callback `_on_click(game)` pour exécuter le script d'un objet cliqué en Lecture.
- Variables globales partagées entre les scripts du monde et des objets pendant
  la session, réinitialisées à l'arrêt.
- Accès Python en lecture seule aux scènes et objets d'autres scènes, ainsi que
  recherche obligatoire d'objet et mesure de distance.
- Copie/collage de zones de tuiles sans les objets présents, et placement des
  nouveaux objets sur la tuile sélectionnée ou survolée.
- Cadrage de l'éditeur indépendant par scène, noms uniques par scène et commandes
  de viewport pour afficher/masquer FPS et sélecteur de caméra en Lecture.
- Snippets VS Code pour l'interface HUD et le callback de clic d'objet.

### Modifié

- Mise à jour de la documentation d'architecture et de l'API Python pour les
  nouvelles fonctions de script et le comportement du viewport.

### Vérification

- `cargo check --workspace` : réussi sans erreur ni avertissement.
- `cargo build --release -p editor` : réussi dans `target/release-check`.
- `cargo test --workspace` : 29 tests réussis dans `target/release-check`.
- `python -m unittest discover -s python/tests -v` : 5 tests réussis.
- Paramètres, extensions recommandées et snippets VS Code valides en JSON.

## 0.3.0 — 2026-10-30

### Ajouté

- Suggestions des douze derniers fichiers choisis dans les champs de texture
  et de script Python de l'inspecteur et des définitions de type, avec listes
  séparées par format ; le sélecteur Windows reste disponible.
- Confirmation avant de quitter si le projet comporte des modifications non
  enregistrées, avec les choix sauvegarder, quitter sans sauvegarder et annuler.
  Cette confirmation peut être désactivée dans les paramètres de l'éditeur.
- Raccourcis `F` pour Peindre et `G` pour Gomme.

### Optimisé et modifié

- Regroupement des tuiles, textures et lignes de grille en meshes, réduisant
  le travail de dessin par frame. La mesure faite sur la scène de référence a
  montré un gain d'environ 10 FPS.
- Évite les copies des objets et le calcul des chevauchements lorsque les hooks
  Python correspondants ne sont pas chargés.
- La molette du viewport est désactivée en Lecture et Pause.

### Vérification

- `cargo check --workspace` et `cargo build --release -p editor`.
- `cargo test --workspace` et `python -m unittest discover -s python/tests -v`.
- Paramètres VS Code et snippets Python contrôlés.

## 0.2.0 — 2026-10-30

### Ajouté

- Types d'objets personnalisés créés dans le projet, avec propriétés typées,
  valeurs par défaut génériques (texture, script, collisions, empreinte,
  rotation, couche, visibilité) et ajout d'instances depuis l'onglet Scène.
- Événements de zone Python `_on_area_entered`, `_on_area_stayed` et
  `_on_area_exited`, avec instantanés de l'objet détecté.
- Transfert d'objets entre scènes, navigation par chargement/dépilement de scène,
  choix de la scène principale et gestion des scènes depuis l'éditeur.
- Copie et collage d'objets, avec conservation des propriétés et adaptation des
  identifiants lors du collage.
- Miniatures des textures dans la palette de tuiles et snippets VS Code pour les
  hooks Python du monde et des zones.

### Modifié

- La définition des types d'objets se règle dans l'inspecteur ; les sélecteurs
  de texture et de script importent les fichiers dans le projet.
- Suppression des logs d'information lors d'un transfert d'objet réussi.
- Retrait de l'interface de construction par bâtiments et prefabs ; les anciens
  champs de prefab restent lisibles pour compatibilité.

### Vérification

- `cargo check --workspace` et `cargo build --release -p editor` — réussis.
- `cargo test --workspace` — 29 tests réussis.
- `python -m unittest discover -s python/tests -v` — 5 tests réussis.
- Snippets VS Code valides en JSON.

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
