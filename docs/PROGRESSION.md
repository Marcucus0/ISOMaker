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
  Dans l'éditeur, les scènes peuvent être renommées ou supprimées depuis leur
  menu clic droit (la dernière scène est protégée). En Lecture, l'API Python
  propose `game.load_scene(name)` et `game.unload_scene()` avec une pile de
  navigation, ainsi que `game.transfer_object(id, scene_name, x, y)` pour
  déplacer le même objet (UUID et propriétés conservés) entre scènes sans
  changer la scène active. Le script peut appeler explicitement
  `game.load_scene(scene_name)` pour l'activer. Le transfert respecte les collisions de destination ;
  l'arrêt restaure le projet et la scène de départ. Le monde
  possède une scène principale choisie dans ses paramètres, utilisée au début
  de Lecture et par le lecteur autonome ; les anciens projets utilisent la
  première scène.
- **Fichiers du projet** : le panneau Assets sert de mini gestionnaire enraciné
  dans le dossier `<nom_du_projet>/` associé au fichier `.isomaker`. Il permet
  de naviguer dans les dossiers, d'ouvrir les fichiers avec l'application
  Windows associée, de créer des fichiers/dossiers et de renommer les éléments.
  Les renommages actualisent les références aux textures et scripts dans le
  projet, puis le sauvegardent.
- **Fermeture** : compare le projet à son dernier enregistrement depuis la
  croix de la fenêtre ou **Fichier → Quitter**. En cas de changements, propose
  Sauvegarder et quitter, Quitter sans sauvegarder ou Annuler. La confirmation
  peut être désactivée dans les paramètres de l'éditeur.
- **Références d'assets dans l'inspecteur** : les champs texture et script
  proposent les derniers fichiers utilisés du type correspondant ; les listes
  sont distinctes et le sélecteur Windows reste disponible.
- **Vue isométrique** : grille par défaut de 64 × 32 pixels par tuile,
  coordonnées logiques, caméra avec zoom centré sous le curseur, déplacement et
  affichage allégé à faible zoom. Un compteur FPS lissé s'affiche dans le
  viewport pendant Lecture.
- **Outils de tuiles** : peinture, gomme, sélection, déplacement, rotation,
  remplissage de zones contiguës ou de la sélection, annulation et
  rétablissement. `F` active Peindre et `G` active Gomme. En Lecture et en
  Pause, les commandes d'édition sont bloquées pour laisser le clavier au jeu.
- **Tuiles personnalisées** : chaque projet démarre avec les six terrains
  historiques et peut ajouter ses propres tuiles. Leur nom, couleur, texture
  PNG/JPEG ou GIF animé, collision, rotation pseudo-aléatoire de la texture
  et option « Visible dans l'éditeur uniquement » (masquée en Lecture)
  sont enregistrés dans le projet et dans ses fichiers `tiles/`; les textures
  sont importées dans les assets propres au projet. Une tuile personnalisée
  peut être supprimée après confirmation ; ses cases sont effacées dans toutes
  les scènes et son fichier de définition est retiré lors de la sauvegarde.
- **Copier/coller** : avec l'outil Sélection, `Ctrl+C` copie les objets
  sélectionnés (ou une tuile seule si aucun objet n'est sélectionné) et
  `Ctrl+V` les colle sous le pointeur. Les objets gardent leurs propriétés et
  leurs écarts relatifs, reçoivent de nouveaux identifiants et peuvent être
  collés dans une autre scène du même projet. Le presse-papiers est vidé lors
  de l'ouverture d'un autre projet.
- **Historique** : `Ctrl+Z` annule et `Ctrl+Y` rétablit la dernière modification
  de la scène.
- **Copier/coller** : `Ctrl+C` sur une zone sélectionnée copie les tuiles de
  cette zone, y compris les vides, sans les objets qui s'y trouvent ; `Ctrl+V`
  colle la zone à partir de la tuile survolée.
- **Placement des objets** : un nouvel objet générique, type personnalisé ou
  caméra apparaît sur la tuile sélectionnée, sinon sur celle survolée (ou la
  dernière survolée si la commande vient d'un panneau latéral).
- **Confirmation de suppression** : une boîte de dialogue confirme la suppression
  d'objets depuis la scène ou la liste d'objets. `Entrée` confirme et `Échap`
  annule.
- **Logs Python** : les messages normaux, avertissements et erreurs indiquent
  le fichier script d'origine et le nom de l'objet si le script y est attaché.
  Les lignes peuvent être sélectionnées, puis copiées avec `Ctrl+C` ou le bouton
  **Copier sélection** ; `Shift` étend la sélection et `Ctrl` bascule une ligne.
- **Projets récents** : le menu **Fichier → Projets récents** permet de rouvrir
  les dix derniers projets ouverts ou enregistrés. La liste est persistante.
- **Raccourcis de simulation** : `F9` lance/reprend, `F10` arrête et `F11` met
  en pause.
- **Raccourcis des outils** : `F` active Peindre, `G` Gomme et `A` Remplir
  lorsque l'éditeur est actif et qu'aucun champ de texte n'a le focus.
- **Interface HUD** : les scripts peuvent créer ou actualiser du texte avec
  `game.ui.set_text`, modifier séparément son contenu, sa position, son ancrage,
  sa taille ou sa couleur, puis le retirer par identifiant avec
  `game.ui.remove_text`. Neuf points d'ancrage positionnent les textes dans le
  viewport en points d'interface. Le HUD persiste pendant la pause et est vidé
  au Stop ; il n'est pas sauvegardé dans le projet et n'est pas interactif.
- **Contrôles du viewport en Lecture** : la barre de statut permet d'afficher
  ou masquer le sélecteur caméra/vue libre et le compteur FPS.
- **Caméras** : ajout de caméras de scène avec position, zoom et choix d'une
  caméra principale. Leur inspecteur propose un réglage pour activer le lissage
  et un curseur de vitesse. En Lecture et Pause, le viewport peut suivre une
  caméra choisie avec interpolation fluide de position et de zoom, ou afficher
  la vue libre de l'éditeur. La molette du viewport est désactivée en Lecture
  et Pause. En édition, le cadrage libre (position et zoom) est mémorisé par
  scène pendant la session et restauré lors du changement de scène.
- **Paramètres de l'éditeur** : le menu **Fichier** ouvre les préférences
  globales sauvegardées dans le dossier de configuration utilisateur. La
  grille peut être masquée pendant Lecture et Pause, et sa couleur est
  configurable. Des cases définissent l'affichage initial des FPS, du sélecteur
  de vue et des logs. Un projet peut être choisi pour s'ouvrir au prochain
  démarrage de l'éditeur.
- **Sélection d'objets** : sélection rectangulaire avec `Shift`, surlignage,
  déplacement temporaire avec aperçu et suppression. Le panneau Bâtiments et
  l'outil Construire ont été retirés au profit des types d'objets personnalisés.
- **Objets** : propriétés modifiables, collisions désactivées par défaut,
  propriété `Masqué` (false par défaut), texture vide par défaut, sélecteur de
  fichiers Windows, affichage des textures et option d'ajustement au losange
  activée par défaut. Chaque objet peut aussi choisir ou créer son script
  Python avec les hooks `_on_game_launch` et `_on_update`. L'inspecteur regroupe
  les réglages en catégories repliables : identité, transformation, apparence,
  physique, script et propriétés personnalisées. Les outils de la barre
  supérieure utilisent des icônes vectorielles dessinées par l'interface, avec
  leurs libellés conservés.
- **Collisions de grille** : en Lecture, les déplacements des scripts sont
  bloqués par les tuiles et objets solides à la même élévation lorsque le mobile
  et l'obstacle ont tous deux les collisions activées. Le calcul utilise les
  empreintes rectangulaires ; formes libres et poussée restent à faire.
- **Zones de détection** : un objet dont le script définit `_on_area_entered`,
  `_on_area_stayed` ou `_on_area_exited` détecte les autres empreintes
  rectangulaires qui le recouvrent au même étage. L'empreinte se règle dans
  l'inspecteur. Chaque hook reçoit `(game, other)` et les collisions physiques
  ne sont pas nécessaires pour les événements de zone.
- **Types d'objets personnalisés** : l'onglet **Types** permet de créer et
  sélectionner des types ; leur définition s'édite dans l'inspecteur. On peut y
  ajouter des propriétés texte, entières, décimales,
  booléennes ou de type scène, puis d'ajouter les instances depuis l'onglet
  **Scène**. Chaque type peut définir les valeurs initiales de ses propriétés
  génériques : texture, collisions, visibilité, empreinte, rotation, couche et
  script. Les propriétés déclarées et génériques sont copiées à la création ;
  les valeurs restent propres à chaque instance. Les propriétés scène sont
  choisies dans la liste des scènes du projet et sont actualisées quand une
  scène est renommée ou supprimée. Types et valeurs sont sauvegardés dans
  `.isomaker` ; les données legacy de bâtiments restent lisibles.
- **Noms d'objets** : les noms sont uniques par scène. Lors d'un ajout ou d'un
  renommage qui ferait doublon, le moteur choisit automatiquement le premier
  suffixe libre, comme `Coffre (1)`.
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
  `game.get_object(uuid)` ou `game.find_object(nom)`, mesurer la distance 2D
  en tuiles avec `game.get_distance(objet1, objet2)`, puis modifier les champs
  éditables avec `set_self_property` ou `game.set_property`.
  Les scripts générés sont annotés pour l'autocomplétion VS Code/Pylance à l'aide
  du stub `python/engine_api/__init__.pyi`.
  `game.set_global(key, value)` et `game.get_global(key)` partagent les valeurs
  JSON entre le monde et les scripts d'objets pendant la lecture ; elles sont
  réinitialisées au Stop et ne sont pas sauvegardées dans le projet.
  Lecture est bloquée si le monde n'a pas de script défini ou si son fichier
  ne peut pas être chargé ; la fenêtre Monde s'ouvre alors.
  Les logs identiques consécutifs sont regroupés pour limiter le coût des
  sorties produites à chaque frame.
- **Lecteur et export Windows** : le binaire `player` lance le projet sans les
  panneaux d'édition. **Fichier → Exporter le jeu** sauvegarde le projet,
  demande le chemin de sortie et lance la compilation en arrière-plan.
  `scripts/export_game.py` compile le lecteur et produit un ZIP portable avec
  le projet, son dossier associé et la bibliothèque standard Python. Les
  paquets Python tiers ne sont pas inclus automatiquement. Le succès et les
  erreurs détaillées sont consignés dans le panneau Log, après validation ZIP.

### Limites connues et travail restant

- **Optimisations de rendu et de scripts** : les meshes des tuiles, textures,
  contours et grille sont regroupés par lot pour limiter les formes egui
  construites à chaque frame. Les recherches de palette et d'objets déplacés
  sont indexées pendant le dessin. Les callbacks Python absents sont ignorés et
  les instantanés sont réutilisés quand les commandes du monde ne changent pas
  la scène. Il reste à profiler le tri complet des objets, les instantanés
  Python et les événements de zones sur de grandes scènes. Les snapshots et la
  détection de zones sont maintenant omis lorsqu'aucun hook concerné n'est chargé.

- Le placement par glisser-déposer et les scripts ou textures par défaut partagés
  par un type restent à ajouter si nécessaire.
- La couleur de la grille est une préférence globale sauvegardée dans les
  paramètres de l'éditeur. Les dimensions de tuile existent dans les métadonnées de
  scène, sans UI dédiée.
- La visibilité par défaut de la grille en mode édition est configurable et
  sauvegardée dans les préférences globales de l'éditeur.
- Le HUD Python peut faire disparaître un texte avec `game.ui.fade_out(id,
  durée)` ; le moteur réduit son opacité puis le retire automatiquement.
- Le HUD Python peut aussi faire apparaître un texte existant avec
  `game.ui.fade_in(id, durée)` ; `set_text(..., visible=False)` le crée caché
  avant ce fondu.
- L'export ne collecte pas encore les dépendances Python tierces utilisées par
  les scripts. Le ZIP doit être vérifié sur une installation Windows propre.
- L'API Python expose les scènes et objets du projet sous forme d'instantanés en lecture seule. Les entrées souris génériques et les événements hors des zones restent à développer. Le débogage interactif et les permissions de scripts tiers restent à concevoir.
- Le format projet est en version 1. Les migrations entre versions restent à
  définir si le schéma évolue.

## Prochaines étapes proposées

1. Ajouter le placement par glisser-déposer des types d'objets et évaluer les
   besoins de scripts ou textures par défaut partagés par un type.
2. Étendre l'API Python avec lecture de l'état du monde, accès aux entrées souris
   et autres événements de jeu, en gardant les échanges groupés par callback.
3. Gérer les dépendances Python tierces de l'export, valider le ZIP sur une
   installation Windows propre et améliorer le débogage des scripts.
4. Étendre les collisions avec des formes libres et la résolution de poussée.
5. Ajouter une interface pour les dimensions de tuile et décider quels réglages
   de vue doivent être sauvegardés.
6. Définir une stratégie de migration et de validation pour les versions
   futures du format `.isomaker`.

L'ordre peut changer selon les besoins exprimés pendant le développement.

## Vérification

Préparation de release du 29 octobre 2026 :

- `cargo check --workspace` : réussi, sans avertissement.
- `cargo build --release -p editor` : réussi.
- `cargo test --workspace` : 24 tests réussis.
- `python -m unittest discover -s python/tests -v` : 5 tests réussis.

Après l'ajout des tests pour les tuiles personnalisées et animées :

- `cargo check --workspace` : réussi, sans avertissement.
- `cargo build --release -p editor` : réussi.
- `cargo test --workspace` : 30 tests réussis dans un répertoire de build
  distinct pour éviter le verrou Windows sur `target/debug/editor.exe`.
- `python -m unittest discover -s python/tests -v` : 5 tests réussis.

## Lancer l'éditeur

Depuis la racine du dépôt, avec Rust installé :

```powershell
cargo run -p editor
```
