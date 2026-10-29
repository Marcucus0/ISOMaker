# Projet : moteur de jeu 2D isométrique avec éditeur visuel — Rust + Python

## 1\. Objectif général

Je veux développer mon propre moteur de jeu accompagné d'un éditeur visuel complet, dans l'esprit de Godot ou Unity, mais conçu initialement pour les jeux 2D isométriques.

Ce projet doit être un véritable logiciel réutilisable, pas simplement un jeu ni une démonstration graphique. Il doit permettre de créer des cartes, placer des objets, construire des bâtiments modulaires, configurer les propriétés des éléments, sauvegarder les projets et exécuter des jeux.

Je dispose déjà de code Python contenant de la logique de jeu que je souhaite pouvoir réutiliser. Je n'utilise pas Pygame et je ne veux pas être obligé de migrer toute ma logique vers Rust.

L'architecture doit donc reposer sur deux technologies complémentaires :

- **Rust pour le moteur et les parties techniques sensibles aux performances.**
- **Python pour la logique de jeu et les scripts exécutés par le moteur.**

Je veux commencer par un prototype fonctionnel de bout en bout, puis améliorer progressivement le logiciel sans devoir tout reconstruire.

## 2\. Architecture technique

Tu dois concevoir une architecture modulaire, performante, économe en mémoire et sécurisée.

### Moteur en Rust

Rust doit prendre en charge les tâches centrales :

- Gestion de la fenêtre, des événements et des entrées utilisateur.
- Rendu graphique accéléré par GPU.
- Caméra, zoom et déplacement dans le monde.
- Système de coordonnées et grille isométrique.
- Gestion des objets, transformations, scènes et couches de rendu.
- Chargement, mise en cache et libération des ressources.
- Tri d'affichage des objets isométriques.
- Gestion des collisions et de la physique lorsque nécessaire.
- Gestion du temps, des mises à jour et de la boucle de jeu.
- Communication avec les scripts Python.
- Sauvegarde et chargement des données du projet.
- Gestion des erreurs et des ressources.

Choisis les bibliothèques Rust adaptées au besoin réel. Évalue notamment `wgpu` pour le rendu, `winit` pour la fenêtre et `egui` ou une solution équivalente pour l'interface de l'éditeur.

Ne multiplie pas les bibliothèques sans raison. Justifie les choix structurants et privilégie des dépendances maintenues, multiplateformes et adaptées à un projet durable.

### Logique de jeu en Python

Python doit permettre de définir les comportements du jeu sans avoir à modifier ou recompiler le moteur Rust.

Il doit notamment pouvoir gérer :

- Les règles et mécaniques de jeu.
- L'économie et les ressources.
- Les personnages et leur comportement.
- Les bâtiments et leurs interactions.
- L'inventaire et les objets.
- Les événements et les quêtes.
- L'IA et les décisions des entités.
- Les interactions entre les systèmes du jeu.
- Les comportements personnalisés des objets.
- Les événements de démarrage, de mise à jour et d'arrêt.

Les scripts doivent pouvoir accéder aux fonctionnalités du moteur via une API documentée, stable et simple à utiliser.

Exemple d'API souhaitée, à adapter à l'architecture retenue :

``` python
def on_start(game):
    game.set_money(10000)
    game.spawn("house", position=(10, 5))

def on_update(game, delta_time):
    if game.input.is_pressed("build"):
        game.build("house")
```

Cet exemple illustre l'ergonomie souhaitée ; il ne représente pas une API déjà implémentée.

### Communication Rust ↔ Python

Tu as la responsabilité de choisir la meilleure solution pour relier les deux langages.

Étudie en priorité l'intégration de Python dans le processus Rust avec PyO3 et, si approprié, `pyo3` avec une distribution Python embarquée ou une configuration d'environnement adaptée. Compare cette approche à un processus Python séparé uniquement si cela présente un avantage réel.

Les priorités, dans cet ordre, sont :

1. Sécurité et isolation des responsabilités.
2. Faible latence et bonnes performances.
3. Faible consommation mémoire.
4. Faible surcharge lors des appels entre les langages.
5. Simplicité de déploiement et de maintenance.
6. Possibilité de déboguer les scripts Python.

Privilégie une intégration en processus si elle offre le meilleur compromis pour le projet, mais ne la considère pas comme automatiquement sécurisée.

Contraintes importantes :

- Éviter les allers-retours inutiles entre Rust et Python.
- Ne pas traverser la frontière entre les langages pour chaque opération graphique individuelle.
- Exposer une API limitée et cohérente plutôt que des accès arbitraires à la mémoire interne du moteur.
- Regrouper les opérations lorsque c'est pertinent.
- Utiliser des structures de données adaptées aux échanges.
- Définir clairement la propriété des objets, leur durée de vie et la gestion des erreurs.
- Éviter les copies de données coûteuses lorsque cela apporte un bénéfice réel et mesurable.
- Ne jamais bloquer inutilement le thread de rendu sur des opérations Python lentes.
- Définir un comportement prévisible lorsqu'un script provoque une erreur.

Évalue les risques liés au Global Interpreter Lock (GIL), aux callbacks, à la réentrance, aux références Python et à la synchronisation des threads. Ne lance pas arbitrairement du code Python depuis plusieurs threads.

Le moteur doit conserver le contrôle de sa boucle de rendu et de ses ressources critiques.

### Sécurité des scripts

Ne prétends pas que l'intégration de Python dans Rust crée automatiquement une sandbox.

Par défaut, considère que les scripts Python du projet sont du code de confiance. Si le moteur doit exécuter des scripts fournis par des tiers, prévois une architecture de permissions et une véritable isolation adaptée au niveau de menace.

Si une isolation forte est nécessaire, étudie notamment l'exécution dans un processus distinct avec des permissions système restreintes et une communication IPC limitée.

Prévois également :

- Des limites raisonnables pour les opérations accessibles aux scripts.
- Des messages d'erreur exploitables.
- Une gestion des exceptions Python.
- Des contrôles sur les données provenant des projets et des ressources importées.
- Une validation des chemins de fichiers.
- Une gestion prudente des ressources externes et des scripts chargés.
- Une séparation entre les données du projet, les ressources et le code exécutable.

Ne présente jamais une simple liste blanche de fonctions comme une isolation de sécurité complète.

## 3\. Éditeur visuel

L'éditeur doit être utilisable sans écrire de code pour les opérations courantes.

### Barre supérieure

Prévois une barre d'outils avec :

- Sélection.
- Déplacement.
- Rotation.
- Redimensionnement.
- Peinture de tuiles.
- Gomme.
- Outils de construction.
- Boutons Lecture, Pause et Arrêt.
- Sélecteur de scène.
- Commandes Annuler et Rétablir.

Les raccourcis clavier doivent être cohérents et configurables à terme.

### Panneau gauche

Il doit contenir des panneaux accessibles par onglets ou sections :

- Bibliothèque des ressources.
- Explorateur de fichiers du projet.
- Hiérarchie des objets de la scène.
- Bibliothèque de tuiles.
- Bibliothèque de modules de construction.

Les ressources doivent pouvoir être glissées-déposées dans la scène lorsque cela est pertinent.

### Vue centrale

La vue centrale doit afficher le monde 2D isométrique et permettre de l'éditer directement.

Fonctionnalités nécessaires :

- Zoom fluide à la souris.
- Déplacement de la caméra.
- Grille isométrique configurable.
- Sélection d'objets à la souris.
- Déplacement et rotation visuels.
- Redimensionnement des éléments compatibles.
- Accrochage à la grille.
- Prévisualisation avant placement.
- Affichage des repères et des dimensions.
- Sélection multiple si possible.
- Annulation et rétablissement des modifications.
- Gestion de scènes suffisamment grandes sans dessiner inutilement tout ce qui est hors écran.

Les coordonnées du monde doivent rester indépendantes de la résolution, du zoom et de la position de la fenêtre.

### Panneau droit : inspecteur

Lorsque je sélectionne un élément, l'inspecteur doit afficher ses propriétés modifiables :

- Nom.
- Position logique.
- Dimensions ou empreinte sur la grille.
- Rotation et orientation.
- Ressource graphique associée.
- Couche de rendu.
- Type d'objet.
- Propriétés spécifiques.
- Script Python associé, le cas échéant.
- Paramètres supplémentaires définis par les composants de l'objet.

Les modifications faites dans l'inspecteur doivent être immédiatement reflétées dans la scène.

## 4\. Monde 2D isométrique

Le moteur est initialement consacré à la 2D isométrique. Il ne faut pas développer un moteur 3D complet inutilement.

Le monde doit utiliser des coordonnées logiques distinctes des coordonnées d'écran.

Pour une grille isométrique classique, utilise une projection de ce type :

``` text
screen_x = origin_x + (x - y) * tile_width / 2
screen_y = origin_y + (x + y) * tile_height / 2 - z
```

L'implémentation devra également gérer la transformation inverse, la sélection précise des cases à la souris et la prise en compte de la caméra et du zoom.

Ne suppose pas que tous les objets sont exactement de la taille d'une tuile. Les bâtiments, arbres et autres éléments peuvent occuper plusieurs cases ou avoir des dimensions visuelles indépendantes de leur empreinte logique.

### Couches

Sépare clairement les couches logiques et les règles de tri visuel.

Prévois notamment :

- Terrain.
- Décoration.
- Objets.
- Constructions.
- Premier plan.

Le tri isométrique doit éviter les problèmes de superposition entre bâtiments, arbres, personnages et objets. Prévois une stratégie cohérente pour les éléments de grande taille et les cas où un objet doit être dessiné devant ou derrière un autre.

## 5\. Éditeur de terrain et de tuiles

L'éditeur doit permettre de :

- Peindre des tuiles sur la grille.
- Effacer des tuiles.
- Remplacer un type de tuile.
- Remplir des zones.
- Sélectionner un type de terrain.
- Gérer plusieurs couches de terrain et de décoration.
- Importer des textures.
- Configurer les dimensions et les ancres des tuiles.
- Sauvegarder et recharger les cartes.

Prépare une architecture qui permettra d'ajouter ultérieurement de l'autotiling, des règles de transition entre terrains et des outils de peinture avancés.

Évite de stocker une carte sous la forme d'un énorme ensemble d'objets coûteux si une représentation plus compacte est possible.

## 6\. Système de construction modulaire

Les bâtiments doivent pouvoir être construits à partir de modules réutilisables.

Exemples de modules :

- Murs.
- Angles.
- Portes.
- Fenêtres.
- Toits.
- Escaliers.
- Piliers.
- Éléments de décoration.

Chaque module doit pouvoir définir :

- Un identifiant stable.
- Une ressource graphique.
- Une empreinte logique.
- Une ancre.
- Une orientation.
- Une couche ou une règle de profondeur.
- Des points d'accroche lorsque cela est utile.
- Des propriétés spécifiques.

L'utilisateur doit pouvoir placer, déplacer, tourner et assembler les modules visuellement.

Les bâtiments assemblés doivent pouvoir être sélectionnés et déplacés en groupe.

Prévois un système de préfabs ou de modèles réutilisables permettant de créer un bâtiment type et d'en placer plusieurs instances. Définis clairement comment les modifications du modèle source affectent les instances, avec la possibilité d'ajouter ultérieurement des modifications locales.

Distingue bien :

- La ressource graphique.
- La définition d'un type d'objet.
- L'instance présente dans une scène.
- Le prefab réutilisable.

Ces concepts ne doivent pas être confondus dans l'architecture.

## 7\. Ressources et importation

L'éditeur doit pouvoir importer des images et les organiser dans le projet.

Prévois :

- Une bibliothèque de ressources.
- Des chemins relatifs au projet.
- La mise en cache des textures.
- La gestion de la transparence.
- Les ancres et points de pivot.
- La prévisualisation.
- Des messages d'erreur en cas de ressource manquante.
- Une séparation entre fichiers source et données générées.

Ne duplique pas inutilement les mêmes textures en mémoire. Le système de ressources doit pouvoir évoluer sans imposer une réécriture du moteur.

## 8\. Projets, scènes et sauvegardes

Un projet doit contenir ses scènes, ses ressources, ses définitions d'objets, ses bâtiments et ses scripts Python.

Utilise initialement un format simple, documenté et versionnable, par exemple JSON pour les métadonnées et les données de scène lorsque cela convient.

Les sauvegardes doivent stocker les coordonnées logiques et les propriétés nécessaires à la reconstruction du monde, jamais les positions d'écran calculées pour l'affichage.

Prévois :

- Création et ouverture d'un projet.
- Création et ouverture de scènes.
- Sauvegarde et rechargement.
- Validation des données.
- Gestion des ressources manquantes.
- Une version de format pour permettre des migrations futures.

La sauvegarde doit rester fiable même lorsque des objets sont ajoutés, déplacés ou supprimés.

## 9\. Séparation entre éditeur et jeu exécuté

L'éditeur et le jeu exécuté doivent partager le même moteur et le même modèle de données, mais leurs responsabilités doivent être clairement séparées.

Lorsque l'utilisateur clique sur Lecture :

- Le moteur lance la scène.
- Les scripts Python sont initialisés.
- Les mises à jour de la simulation commencent.
- Les entrées utilisateur sont transmises aux systèmes appropriés.

La Pause doit suspendre ou contrôler la simulation sans bloquer inutilement l'interface de l'éditeur.

L'Arrêt doit arrêter la simulation et restituer l'état éditable de la scène.

Les modifications temporaires du jeu ne doivent pas écraser silencieusement les données du projet.

Prévois une architecture qui permettra ensuite d'ajouter des collisions, des personnages, des animations, des composants et des systèmes de jeu supplémentaires.

## 10\. Performances et consommation des ressources

La priorité est d'obtenir un moteur fluide, réactif et raisonnablement peu gourmand en mémoire.

Prends notamment en compte :

- Le rendu accéléré par GPU.
- Le culling des éléments hors écran.
- Le regroupement des appels de rendu lorsque cela est pertinent.
- La mise en cache des textures et des ressources.
- Une représentation compacte des tuiles.
- Le partitionnement des grandes cartes en chunks.
- La limitation des allocations répétitives.
- La séparation entre données logiques et données de rendu.
- Le contrôle de la fréquence des mises à jour Python.
- Le regroupement des appels Rust/Python.
- L'absence de calculs coûteux inutiles à chaque frame.
- La gestion explicite de la durée de vie des ressources.

N'optimise pas prématurément chaque détail. Mets en place des mesures simples, des tests reproductibles et des outils de profilage avant de prendre des décisions coûteuses.

Ne promets pas un nombre d'objets ou un framerate sans l'avoir mesuré sur une configuration définie.

## 11\. Qualité et architecture du code

Le projet doit être modulaire et maintenable.

Sépare les responsabilités du moteur, du rendu, de l'éditeur, des ressources, du monde, des scripts et des sauvegardes.

Structure indicative à adapter :

``` text
project/
├── Cargo.toml
├── crates/
│   ├── engine/
│   ├── renderer/
│   ├── editor/
│   ├── scripting/
│   ├── world/
│   └── assets/
├── python/
│   ├── api/
│   └── game_scripts/
├── examples/
├── tests/
└── docs/
```

Cette structure est une proposition de départ, pas une obligation rigide. Choisis le nombre de crates et les frontières entre modules en fonction des dépendances réelles.

Exigences :

- Code lisible et documenté.
- Types et interfaces clairement définis.
- Gestion cohérente des erreurs.
- Tests des systèmes importants.
- Pas de dépendances inutiles.
- Pas de fonctions fictives présentées comme déjà implémentées.
- Pas de code prétendument testé sans exécution effective.
- Pas de fichier vide ou de simple placeholder pour prétendre qu'une fonctionnalité est terminée.
- Pas de monolithe contenant toute la logique du projet.

## 12\. Méthode de développement

Je veux d'abord un prototype complet mais raisonnablement limité, puis une amélioration progressive.

Ne commence pas par implémenter toutes les fonctionnalités imaginables.

Procède par étapes :

1. Définir l'architecture et justifier les choix techniques structurants.
2. Mettre en place un projet Rust qui compile et ouvre une fenêtre.
3. Afficher une grille isométrique et une caméra fonctionnelle.
4. Ajouter la sélection, le déplacement et le placement d'objets.
5. Créer une première interface d'éditeur avec une hiérarchie et un inspecteur.
6. Ajouter le système de ressources et la sauvegarde des scènes.
7. Intégrer Python et démontrer l'exécution d'un script réel.
8. Ajouter l'édition de terrain et les premières constructions modulaires.
9. Implémenter les commandes Lecture, Pause et Arrêt.
10. Ajouter les tests, corriger les problèmes et mesurer les performances.
11. Documenter la création d'un projet et l'ajout de scripts.

À chaque étape, fournis du code réellement utilisable, explique les commandes de compilation et vérifie que l'étape fonctionne avant de poursuivre.

Si une décision technique importante reste incertaine, compare brièvement les options et recommande celle qui correspond le mieux aux contraintes du projet.

## 13\. Résultat attendu pour le premier prototype

Le premier prototype doit permettre de démontrer un parcours complet :

1. Lancer l'éditeur.
2. Créer ou ouvrir un projet.
3. Afficher une grille isométrique.
4. Peindre un terrain simple.
5. Importer ou sélectionner une texture.
6. Placer, sélectionner et déplacer des objets.
7. Modifier les propriétés d'un objet dans l'inspecteur.
8. Assembler un bâtiment simple à partir de modules.
9. Sauvegarder la scène puis la recharger.
10. Lancer la simulation.
11. Exécuter un script Python qui interagit réellement avec le moteur.
12. Mettre en pause puis arrêter la simulation sans corrompre les données de l'éditeur.

Le prototype doit privilégier un flux de travail réellement fonctionnel plutôt qu'une interface impressionnante dont les boutons ne font rien.

## 14\. Ta première réponse

Avant de produire une grande quantité de code, commence par :

1. Présenter l'architecture retenue.
2. Justifier le choix du rendu, de l'interface et de l'intégration Python.
3. Expliquer brièvement les compromis de performance, de mémoire, de sécurité et de déploiement.
4. Proposer un MVP découpé en étapes vérifiables.
5. Identifier les dépendances et les prérequis nécessaires.

Ensuite, commence à construire le projet progressivement. Privilégie les commandes et les fichiers concrets, limite les explications inutiles et indique précisément comment compiler, lancer et tester chaque étape.

L'objectif final est un véritable moteur de jeu 2D isométrique extensible, avec un éditeur visuel pratique et une logique de jeu écrite en Python, exécutée au moyen d'une interface propre avec un moteur performant en Rust.