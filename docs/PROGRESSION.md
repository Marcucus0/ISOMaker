# Progression du moteur de jeu isométrique

Ce fichier suit l'état actuel du dépôt et les prochaines étapes. Il remplace
l'ancien inventaire détaillé qui mélangeait tâches terminées, fonctionnalités
en projet et résultats de tests datés. Le détail des responsabilités se trouve
dans [`ARCHITECTURE.md`](ARCHITECTURE.md). Les exigences initiales restent dans
[`PROMPT BASE.md`](PROMPT%20BASE.md) ; elles ne signifient pas qu'une fonction
est déjà implémentée.

## État actuel

Le projet est un prototype d'éditeur isométrique fonctionnel, construit en Rust
avec une interface `egui`, une fenêtre `winit` et un rendu `wgpu`. Il permet
d'éditer plusieurs scènes d'un monde, de placer des tuiles et des objets, puis
d'enregistrer le projet et ses textures. Ce n'est pas encore un moteur de jeu
complet prêt au déploiement.

### Fonctionnalités présentes

- **Projets et scènes** : un projet contient un monde, un monde contient
  plusieurs scènes. Le format `.isomaker` sérialise le projet complet et les
  dialogues Windows servent à ouvrir, enregistrer et enregistrer sous.
- **Fichiers du projet** : le panneau Assets sert de mini gestionnaire enraciné
  dans le dossier `<nom_du_projet>/` associé au fichier `.isomaker`. Il permet
  de naviguer dans les dossiers, d'ouvrir les fichiers avec l'application
  Windows associée, de créer des fichiers/dossiers et de renommer les éléments.
  Les renommages actualisent les références aux textures et scripts dans le
  projet, puis le sauvegardent.
- **Vue isométrique** : grille par défaut de 64 × 32 pixels par tuile,
  coordonnées logiques, caméra avec zoom centré sous le curseur, déplacement et
  affichage allégé à faible zoom. Un compteur FPS lissé s'affiche dans le
  viewport pendant Lecture.
- **Outils de tuiles** : peinture, gomme, sélection, déplacement, rotation,
  remplissage de zones contiguës ou de la sélection, annulation et
  rétablissement. En Lecture et en
  Pause, les commandes d'édition sont bloquées pour laisser le clavier au jeu.
- **Copier/coller de tuiles** : avec l'outil Sélection, `Ctrl+C` copie la tuile
  seule sélectionnée et `Ctrl+V` la colle sur la tuile sélectionnée.
- **Historique** : `Ctrl+Z` annule et `Ctrl+Y` rétablit la dernière modification
  de la scène.
- **Confirmation de suppression** : une boîte de dialogue confirme la suppression
  d'objets depuis la scène ou la liste d'objets. `Entrée` confirme et `Échap`
  annule.
- **Logs Python** : les messages normaux, avertissements et erreurs indiquent
  le fichier script d'origine et le nom de l'objet si le script y est attaché.
- **Projets récents** : le menu **Fichier → Projets récents** permet de rouvrir
  les dix derniers projets ouverts ou enregistrés. La liste est persistante.
- **Raccourcis de simulation** : `F9` lance/reprend, `F10` arrête et `F11` met
  en pause.
- **Caméras** : ajout de caméras de scène avec position, zoom et choix d'une
  caméra principale. Leur inspecteur propose un réglage pour activer le lissage
  et un curseur de vitesse. En Lecture et Pause, le viewport peut suivre une
  caméra choisie avec interpolation fluide de position et de zoom, ou afficher
  la vue libre de l'éditeur.
- **Paramètres de l'éditeur** : le menu **Fichier** ouvre les préférences
  globales sauvegardées dans le dossier de configuration utilisateur. La
  grille peut être masquée pendant Lecture et Pause, et sa couleur est
  configurable.
- **Sélection d'objets** : sélection rectangulaire avec `Shift`, surlignage,
  déplacement temporaire avec aperçu et suppression. Les modules et prefabs du
  panneau Bâtiments peuvent être glissés dans la scène.
- **Objets** : propriétés modifiables, collisions désactivées par défaut,
  propriété `Masqué` (false par défaut), texture vide par défaut, sélecteur de
  fichiers Windows, affichage des textures et option d'ajustement au losange
  activée par défaut. Chaque objet peut aussi choisir ou créer son script
  Python avec les hooks `_on_game_launch` et `_on_update`. L'inspecteur regroupe
  les réglages en catégories repliables : identité, transformation, apparence,
  physique, script et propriétés personnalisées. Les outils de la barre
  supérieure utilisent des icônes vectorielles dessinées par l'interface, avec
  leurs libellés conservés.
- **Modèle de bâtiments** : modules et prefabs peuvent être instanciés. La
  maison intégrée existe comme exemple ; ses textures se choisissent dans les
  propriétés plutôt que d'être assignées automatiquement.
- **Python** : runtime PyO3 et entrée **Monde → Script du monde**. Le script
  configuré est chargé à Lecture ; `_on_game_launch(game)` est appelé une fois
  et `_on_update(game, delta_time)` à chaque mise à jour. Le bouton **Créer un
  script…** génère ces deux fonctions si le fichier est nouveau ; un fichier
  sélectionné vide reçoit le même squelette. Les hooks globaux historiques
  restent compatibles. La référence des fonctions et un exemple se trouvent
  dans [`PYTHON_API.md`](PYTHON_API.md).
  Les scripts du monde et des objets peuvent interroger les touches maintenues,
  pressées ou relâchées pendant Lecture avec les constantes `game.keys`.
  Les scripts attachés aux objets peuvent aussi lire leurs coordonnées courantes
  et les champs de l'inspecteur, retrouver un autre objet avec
  `game.get_object(uuid)` ou `game.find_object(nom)`, puis modifier les champs
  éditables avec `set_self_property` ou `game.set_property`.
  Les scripts générés sont annotés pour l'autocomplétion VS Code/Pylance à l'aide
  du stub `python/engine_api/__init__.pyi`.
  Lecture est bloquée si le monde n'a pas de script défini ou si son fichier
  ne peut pas être chargé ; la fenêtre Monde s'ouvre alors.
  Les logs identiques consécutifs sont regroupés pour limiter le coût des
  sorties produites à chaque frame.

### Limites connues et travail restant

- L'éditeur de plans réutilisables de bâtiments n'existe pas encore. Son
  périmètre est listé dans [`todo list.md`](../todo%20list.md).
- La couleur de la grille est une préférence globale sauvegardée dans les
  paramètres de l'éditeur. L'option d'affichage de la grille en Lecture/Pause
  n'est pas persistée. Les dimensions de tuile existent dans les métadonnées de
  scène, sans UI dédiée.
- La boucle de scripts est liée à Lecture dans l'éditeur, mais l'exécution
  autonome d'un jeu publié n'est pas encore implémentée.
- L'API Python expose les objets de scène sous forme d'instantanés en lecture
  seule, mais pas encore l'état global du monde, les entrées souris ou les
  événements ; le débogage interactif et les permissions de scripts tiers
  restent à concevoir.
- Les collisions sont une propriété des objets ; la simulation physique et la
  résolution des collisions ne sont pas encore implémentées.
- Le format projet est en version 1. Les migrations entre versions restent à
  définir si le schéma évolue.

## Prochaines étapes proposées

1. Construire l'éditeur visuel de plans de bâtiments décrit dans le TODO, puis
   enregistrer les plans dans le projet et gérer leurs instances.
2. Étendre l'API Python avec lecture de l'état du monde, accès aux entrées souris
   et événements de jeu, en gardant les échanges groupés par callback.
3. Finaliser l'exécution autonome du jeu, le débogage des scripts et la
   séparation entre éditeur et runtime.
4. Définir les besoins de physique/collision, puis implémenter une première
   détection exploitable par les scripts.
5. Ajouter une interface pour les dimensions de tuile et décider quels réglages
   de vue doivent être sauvegardés.
6. Définir une stratégie de migration et de validation pour les versions
   futures du format `.isomaker`.

L'ordre peut changer selon les besoins exprimés pendant le développement.

## Vérification

Pour la préparation de release du 29 octobre 2026 :

- `cargo check --workspace` : réussi, sans avertissement.
- `cargo build --release -p editor` : réussi.
- `cargo test --workspace` : 24 tests réussis.
- `python -m unittest discover -s python/tests -v` : 5 tests réussis.

## Lancer l'éditeur

Depuis la racine du dépôt, avec Rust installé :

```powershell
cargo run -p editor
```
