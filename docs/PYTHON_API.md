# API Python du moteur

Ce document décrit les fonctions actuellement exposées aux scripts via
`GameContext`. Il couvre le script du monde choisi dans **Monde → Script du
monde…**. Les exemples sont exécutés dans le runtime de l'éditeur.

## Hooks du script du monde

Le fichier peut définir ces deux fonctions :

```python
def _on_game_launch(game):
    """Appelée une fois quand on démarre Lecture."""
    ...


def _on_update(game, delta_time):
    """Appelée à chaque mise à jour pendant Lecture."""
    ...
```

Les deux hooks sont optionnels. Si le fichier sélectionné est vide, l'éditeur
ajoute automatiquement ces définitions avec `pass`. `delta_time` est exprimé
en secondes. Le module reste chargé pendant la session : ses variables globales
Python persistent donc d'un appel à `_on_update` au suivant.

## Autocomplétion dans VS Code

Les stubs de types de l'API se trouvent dans `python/engine_api/__init__.pyi`.
À la racine du dépôt, `.vscode/settings.json` ajoute `python/` aux chemins
d'analyse de Pylance. Les nouveaux scripts générés importent `GameContext`
uniquement sous `TYPE_CHECKING`, ce qui active l'autocomplétion sans tenter
d'importer ce module dans le runtime embarqué.

Les hooks générés sont annotés ainsi :

```python
from __future__ import annotations
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from engine_api import GameContext


def _on_update(game: GameContext, delta_time: float) -> None:
    if game.key_just_pressed(game.keys.Z):
        game.move_self(game.x + 1, game.y)
```

Pour que ces suggestions apparaissent, ouvrir le dépôt comme dossier VS Code et
utiliser l'extension Python avec Pylance. Les scripts déjà existants doivent
avoir le type `GameContext` sur leur paramètre `game` ; le script d'exemple du
projet `test2` a été annoté.

## Script attaché à un objet

Dans l'inspecteur d'un objet, **Script Python → Choisir…** permet d'attacher un
fichier existant et **Créer…** en crée un avec les deux hooks de base. Ce script
utilise les mêmes signatures que le script du monde. Il est appelé une fois au
lancement puis à chaque mise à jour tant que l'objet existe. Chaque objet
attaché possède sa propre instance Python du module.

Dans ces callbacks, `game.object_id` contient l'UUID de l'objet. Les méthodes
`move_self`, `destroy_self` et `set_self_property` agissent sur cet objet :

```python
def _on_game_launch(game):
    game.log(f"Script lancé pour l'objet {game.object_id}")


def _on_update(game, delta_time):
    # Exemple : déplace cet objet d'une case vers la droite.
    # game.move_self(5, 3)
    pass
```

La position actuelle de l'objet est disponible séparément dans `game.x`,
`game.y` et `game.z` (coordonnées de tuile, avec `z` comme étage). Ces valeurs
sont en lecture seule. Elles sont également accessibles dans
`_on_game_launch(game)` :

```python
def _on_update(game, delta_time):
    if game.key_just_pressed(game.keys.Z):
        # move_self attend des coordonnées X et Y absolues et conserve l'étage Z.
        game.move_self(game.x + 1, game.y)
```

Ces propriétés ne sont disponibles que pour un script attaché à un objet ; les
accéder depuis le script du monde déclenche une erreur explicite.

### Accéder aux autres objets

`game.get_object(uuid)` retourne un `GameObject` en lecture seule, ou `None` si
l'UUID n'existe pas. `game.find_object(nom)` recherche le premier objet dont le
nom correspond exactement. Quand plusieurs objets portent le même nom, le
premier UUID dans l'ordre lexical est retourné. `GameObject` expose `id`,
`name`, `object_type`, `x`, `y`, `z`, `camera_zoom`, ainsi que les principaux
champs de l'inspecteur. `get_property(key)` lit aussi une propriété
personnalisée.

Par exemple, le script du personnage peut déplacer la caméra nommée « Caméra »
à la position du personnage sans coder l'UUID de la caméra :

```python
def _on_update(game: GameContext, delta_time: float) -> None:
    camera = game.find_object("Caméra")
    if camera is not None:
        game.move_object(camera.id, game.x, game.y)
```

La lecture retourne un instantané des objets au début du callback. Les
commandes de déplacement émises par les scripts sont appliquées après les
callbacks ; un script voit donc les modifications des callbacks précédents au
prochain appel.

### Propriétés d'un objet

Le tableau distingue les champs visibles dans l'inspecteur et ceux qu'un script
peut actuellement consulter ou modifier. La présence d'un champ dans l'éditeur
ne signifie pas qu'il est déjà exposé à Python.

| Champ de l'objet | Dans l'inspecteur | API Python actuelle |
|---|---|---|
| Identifiant (`id`) | UUID affiché en abrégé | `game.object_id` donne l'UUID complet en lecture seule. |
| Nom (`name`) | Modifiable | `game.name` en lecture ; `set_self_property("name", valeur)` pour l'écriture. |
| Type (`object_type`) | Affiché (`Building`, `Character`, `Item`, `Decoration`, `Camera`, `Tile` ou type personnalisé) | `game.object_type` en lecture seule. |
| Position (`tile_pos`) | Coordonnées X, Y et élévation Z | `game.x`, `game.y`, `game.z` en lecture ; `set_self_property("x"/"y"/"z", entier)` ou `move_self(x, y)` pour déplacer. |
| Collisions (`has_collisions`) | Case à cocher pour les objets autres que les caméras | `game.has_collisions` en lecture ; `set_self_property("has_collisions", bool)` pour l'écriture. La simulation de collisions reste à implémenter. |
| Masqué (`hidden`) | Case à cocher, désactivée par défaut | `game.hidden` en lecture ; `set_self_property("hidden", bool)` pour l'écriture. |
| Texture (`asset_path`, propriété `texture`) | Fichier image sélectionnable pour les objets autres que les caméras | `game.texture` en lecture ; `set_self_property("texture", chemin)` ou `None` pour l'écriture. |
| Ajuster la texture (`fit_texture_to_diamond`) | Case à cocher, activée par défaut pour les objets autres que les caméras | `game.fit_texture_to_diamond` en lecture ; `set_self_property("fit_texture_to_diamond", bool)` pour l'écriture. |
| Rotation (`rotation`) | `North`, `East`, `South` ou `West` pour les objets autres que les caméras | `game.rotation` en lecture ; `set_self_property("rotation", valeur)` pour l'écriture. |
| Zoom caméra (`camera_zoom`) | Réglage dédié aux caméras | `game.camera_zoom` en lecture ; `set_self_property("camera_zoom", nombre)` en écriture. |
| Interpolation caméra (`camera_interpolation`) | Case à cocher sur les caméras, activée par défaut | `game.camera_interpolation` en lecture ; `set_self_property("camera_interpolation", bool)` en écriture. |
| Vitesse d'interpolation (`camera_interpolation_speed`) | Curseur caméra de 0,1 à 20, valeur initiale 8 | `game.camera_interpolation_speed` en lecture ; `set_self_property("camera_interpolation_speed", nombre)` en écriture. |
| Couche (`layer`) | Sélection parmi `Terrain`, `Decoration`, `Ground`, `Objects`, `Buildings`, `Foreground`, `Overlay` | `game.layer` en lecture ; `set_self_property("layer", valeur)` pour l'écriture. |
| Empreinte (`footprint`) | Pas de contrôle dédié dans l'inspecteur ; dimensions par défaut 1 × 1 tuile | `get_self_property("footprint_width")` / `get_self_property("footprint_height")`, avec setters correspondants. |
| Script (`script`) | Chemin du script Python attaché | `game.script` en lecture ; `set_self_property("script", chemin)` ou `None` pour l'écriture. |
| Référence prefab (`prefab_id`) | Pas affichée dans l'inspecteur | Non accessible par le contexte Python actuel. |
| Propriétés personnalisées (`properties`) | Valeurs affichées dans la section **Propriétés** ; la texture y apparaît aussi | `game.get_self_property(key)` lit une valeur ; `game.set_self_property(key, value)` l'ajoute ou la modifie. |

Exemples de champs intégrés et de propriété personnalisée :

```python
def _on_game_launch(game: GameContext) -> None:
    # Les champs intégrés ont des getters dédiés et des noms documentés.
    game.set_self_property("hidden", False)
    game.set_self_property("name", "Personnage")
    game.set_self_property("rotation", "East")

    # Les propriétés personnalisées sont également lisibles.
    game.set_self_property("etat", "au repos")
    etat = game.get_self_property("etat")
```

`id` et `object_type` sont en lecture seule. Les clés `name`, `x`, `y`, `z`,
`has_collisions`, `hidden`, `fit_texture_to_diamond`, `rotation`, `layer`,
`texture` (alias `asset_path`), `script`, `footprint_width` et
`footprint_height` mettent à jour les champs intégrés.
Les rotations acceptent `North`, `East`, `South` ou `West` ; les couches
acceptent les noms listés dans le tableau. Une valeur de type incorrect ou une
valeur enum inconnue produit un message d'erreur dans le journal. Toute autre
clé lit ou modifie une propriété personnalisée.

Les modifications sont mises en file et appliquées après le callback Python.
Un appel à `get_self_property` dans le même callback ne voit donc pas une
modification mise en file juste avant.

Changer `script` pendant une session modifie la référence de l'objet ; le
nouveau script sera chargé au prochain lancement.

`move_self(x, y)` demande un déplacement sur la grille, `destroy_self()` demande
la suppression, et `set_self_property(key, value)` modifie un champ intégré ou
une propriété personnalisée.
Comme les autres commandes, leurs effets sont appliqués après le callback.
Dans un script du monde, `game.object_id` vaut `None` et les trois méthodes
`*_self` lèvent une erreur ; utilisez alors les méthodes prenant un UUID explicite.

Les scripts d'objet n'ont pas encore de hook d'arrêt dédié.

## Fonctions disponibles sur `game`

| Fonction | Effet |
|---|---|
| `game.spawn(name, object_type, x, y)` | Demande la création d'un objet à une position de tuile. |
| `game.destroy(object_id)` | Demande la suppression de l'objet portant cet UUID. |
| `game.move_object(object_id, x, y)` | Demande le déplacement d'un objet existant. |
| `game.set_property(object_id, key, value)` | Modifie un champ d'objet éditable ou une propriété personnalisée. |
| `game.set_global(key, value)` | Émet une commande de valeur globale ; actuellement l'éditeur l'inscrit dans le journal, sans fournir de stockage/lecture de variables globales. |
| `game.log(message)` | Écrit un message d'information dans le journal de l'éditeur, préfixé par le fichier script et le nom de l'objet si disponible. |
| `game.log_warning(message)` | Écrit un avertissement avec la même origine. |
| `game.log_error(message)` | Écrit une erreur avec la même origine. |
| `game.delta_time` | Temps écoulé depuis le dernier frame, en secondes. Dans `_on_update`, l'argument `delta_time` fournit la même valeur. |
| `game.key_pressed(key)` | True tant que la touche est maintenue. |
| `game.key_just_pressed(key)` | True dans le callback du frame où la touche est pressée. |
| `game.key_just_released(key)` | True dans le callback du frame où la touche est relâchée. |
| `game.key_released(key)` | Alias de `key_just_released`. |

Les fonctions clavier sont disponibles dans les callbacks de mise à jour du
monde et des objets pendant Lecture. Elles prennent une constante de `game.keys`
et non une chaîne : lettres (`game.keys.A` à `game.keys.Z`), chiffres
(`game.keys.DIGIT_0` à `game.keys.DIGIT_9`), flèches (`LEFT`, `RIGHT`, `UP`,
`DOWN`), `SPACE`, `ENTER`, `ESCAPE`, `BACKSPACE`, `DELETE`, `TAB`, `SHIFT`,
`CTRL`, `ALT` et `F1` à `F12`. Les lettres suivent le caractère logique du
clavier : `game.keys.Z` désigne donc Z sur AZERTY comme sur QWERTY, avec ou sans
Shift.

```python
def _on_update(game, delta_time):
    if game.key_just_pressed(game.keys.SPACE):
        game.log("Espace pressée")

    if game.key_pressed(game.keys.LEFT):
        # Déplacement continu tant que la flèche reste enfoncée.
        pass

    if game.key_just_released(game.keys.RIGHT):
        game.log("Flèche droite relâchée")

    if game.key_pressed(game.keys.SHIFT) and game.key_pressed(game.keys.Z):
        # Shift et Z sont deux touches indépendantes.
        pass
```

Les états « juste pressée » et « juste relâchée » sont conservés pour tous les
callbacks exécutés pendant le même frame, puis réinitialisés avant le frame
suivant. La lecture clavier ne bloque pas l'utilisation des touches par le jeu
lorsque la fenêtre de l'éditeur reçoit le focus.

`spawn` reconnaît les types `Building`, `Character`, `Item` et `Decoration`.
Tout autre texte devient le nom d'un type personnalisé. Ces fonctions renvoient
`None` : en particulier, `spawn` ne renvoie pas encore l'identifiant créé.
`destroy`, `move_object` et `set_property` attendent l'UUID texte d'un objet
existant.

Les valeurs acceptées par `set_property` et `set_global` sont `None`, un
booléen, un entier, un nombre décimal, une chaîne, une liste ou un dictionnaire
dont les clés sont des chaînes. Les listes peuvent contenir ces mêmes valeurs.
À l'application, `None` devient une chaîne vide pour une propriété, et un
dictionnaire affecté à une propriété devient actuellement la chaîne
`"[object]"` : évite les dictionnaires avec `set_property`. `set_global` garde
la valeur JSON dans la commande, mais l'éditeur ne la stocke pas encore.

`game.find_object(name)` recherche un objet par son nom et
`game.get_object(object_id)` le récupère par UUID. Ces appels renvoient un
instantané en lecture seule, avec les champs exposés dans `ObjectSnapshot`.
L'UUID créé par `spawn` n'est toutefois pas encore renvoyé : pour modifier ou
supprimer un objet précis depuis un script, il faut donc déjà connaître son UUID
ou le retrouver par son nom.

Les fonctions ajoutent des commandes à une file. Le moteur applique cette file
après le retour du callback Python ; le script ne modifie donc pas directement
la scène Rust pendant son exécution.

### Fréquence des logs

Évite d'appeler `game.log` à chaque frame pour une valeur de suivi. Chaque appel
traverse l'API Python/Rust et ajoute un message. Le panneau regroupe maintenant
les messages identiques consécutifs (en affichant « répété N fois ») et conserve
au plus 200 entrées distinctes ;
chaque message indique le fichier Python d'origine et, pour un script d'objet,
le nom de l'objet (par exemple `[camera.py — Caméra]`). Les messages `Info` ne
sont pas recopiés dans la sortie console native. Les avertissements et erreurs
restent transmis au logger natif avec le même préfixe.

Pour observer une valeur périodiquement, limite sa fréquence dans le script :

```python
elapsed = 0.0


def _on_update(game, delta_time):
    global elapsed
    elapsed += delta_time
    if elapsed >= 1.0:
        game.log(f"Temps depuis le dernier log : {elapsed:.2f} s")
        elapsed = 0.0
```

## Exemple complet

```python
"""Script Python lié au monde."""

elapsed = 0.0


def _on_game_launch(game):
    global elapsed
    elapsed = 0.0

    game.log("La partie démarre")
    game.set_global("money", 10000)
    game.spawn("Maison", "Building", 3, 2)


def _on_update(game, delta_time):
    global elapsed
    elapsed += delta_time

    if elapsed >= 1.0:
        game.log("Une seconde de jeu s'est écoulée")
        elapsed = 0.0
```

Dans cet exemple, `set_global` sert pour l'instant à émettre une commande
visible dans les logs de débogage ; il ne crée pas encore un état global que le
script pourrait relire.

## Compatibilité et limites actuelles

Les anciens scripts globaux peuvent encore définir `on_start(game)`,
`on_update(game, delta_time)` et `on_stop(game)`. Le script configuré dans le
menu **Monde** utilise les hooks préfixés par `_on_` ci-dessus. Les exceptions
Python sont remontées dans le journal de l'éditeur avec le chemin et le nom du
hook.

L'API ne fournit pas encore la lecture des objets et de leurs propriétés, la
lecture des variables globales, les entrées souris, les événements de jeu, ni
le retour d'identifiant de `spawn`. Elle ne constitue pas une sandbox :
les scripts chargés sont du code Python de confiance.
