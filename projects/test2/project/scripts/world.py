from __future__ import annotations
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from engine_api import GameContext


def _on_game_launch(game: GameContext) -> None:
    """Appelé une fois au lancement de la partie."""
    pass


def _on_update(game: GameContext, delta_time: float) -> None:
    """Appelé à chaque mise à jour de la partie."""
