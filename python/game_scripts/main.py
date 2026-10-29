"""
Script global de démonstration, chargeable par l'API du runtime.

Ce script montre l'API disponible depuis Python.
Il utilise les hooks globaux `on_start(game)`, `on_update(game, delta_time)`
et `on_stop(game)`. Les scripts attachés à un objet reçoivent également
l'instantané de cet objet en second argument.
"""


def on_start(game):
    """Appelé une fois au démarrage de la simulation."""
    game.log("=== Simulation démarrée ===")

    # Définit une valeur globale.
    game.set_global("money", 10000)
    game.log("Capital initial : 10 000")

    # Spawne quelques objets dans la scène.
    game.spawn("Maison",   "Building",   3,  2)
    game.spawn("Entrepôt", "Building",  -2,  4)
    game.spawn("Villageois", "Character", 0,  0)

    game.log("Objets initiaux créés.")


def on_update(game, delta_time):
    """Appelé à chaque frame pendant la simulation."""
    # Exemple : on peut lire le delta_time pour des actions temporisées.
    # Pour l'instant on se contente de laisser la simulation tourner.
    pass


def on_stop(game):
    """Appelé à l'arrêt de la simulation."""
    game.log("=== Simulation arrêtée ===")
