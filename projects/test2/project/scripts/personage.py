from __future__ import annotations
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from engine_api import GameContext


def _on_game_launch(game: GameContext) -> None:
    """Appelé une fois au lancement de la partie."""
    # Identifiant de cet objet : game.object_id
    game.move_self(1 , 0)
    game.set_self_property("hidden", False)


def _on_update(game: GameContext, delta_time: float) -> None:
    """Appelé à chaque mise à jour de la partie."""
    # Utiliser game.move_self(x, y), game.destroy_self() ou game.set_self_property(...).
    if game.key_pressed(game.keys.Z):
        game.move_self(game.x , game.y - 1)

    if game.key_pressed(game.keys.S):
        game.move_self(game.x , game.y + 1) 
        
    if game.key_pressed(game.keys.D):
        game.move_self(game.x + 1 , game.y) 

    if game.key_pressed(game.keys.Q):
        game.move_self(game.x - 1 , game.y) 
