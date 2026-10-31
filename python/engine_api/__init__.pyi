"""
Stubs de type pour l'API moteur exposée aux scripts Python.
Utilisés par les IDE (VSCode, PyCharm) pour l'autocomplétion.
"""

from __future__ import annotations
from typing import Any


class Key:
    """Constante de touche fournie par game.keys ; ne pas instancier."""
    ...


class KeySet:
    A: Key
    B: Key
    C: Key
    D: Key
    E: Key
    F: Key
    G: Key
    H: Key
    I: Key
    J: Key
    K: Key
    L: Key
    M: Key
    N: Key
    O: Key
    P: Key
    Q: Key
    R: Key
    S: Key
    T: Key
    U: Key
    V: Key
    W: Key
    X: Key
    Y: Key
    Z: Key
    DIGIT_0: Key
    DIGIT_1: Key
    DIGIT_2: Key
    DIGIT_3: Key
    DIGIT_4: Key
    DIGIT_5: Key
    DIGIT_6: Key
    DIGIT_7: Key
    DIGIT_8: Key
    DIGIT_9: Key
    SPACE: Key
    ENTER: Key
    ESCAPE: Key
    BACKSPACE: Key
    DELETE: Key
    TAB: Key
    LEFT: Key
    RIGHT: Key
    UP: Key
    DOWN: Key
    SHIFT: Key
    CTRL: Key
    ALT: Key
    F1: Key
    F2: Key
    F3: Key
    F4: Key
    F5: Key
    F6: Key
    F7: Key
    F8: Key
    F9: Key
    F10: Key
    F11: Key
    F12: Key


class GameObject:
    """Instantané en lecture seule d'un objet de la scène."""

    id: str
    object_id: str
    name: str
    object_type: str
    x: int
    y: int
    z: int
    has_collisions: bool
    hidden: bool
    fit_texture_to_diamond: bool
    camera_zoom: float
    camera_interpolation: bool
    camera_interpolation_speed: float
    rotation: str
    layer: str
    texture: str | None
    script: str | None
    footprint_width: int
    footprint_height: int

    def get_property(self, key: str) -> Any:
        """Lit un champ intégré ou une propriété personnalisée ; None si absent."""
        ...


class GameScene:
    """Instantané en lecture seule d'une scène du projet."""

    name: str
    objects: list[GameObject]

    def get_object(self, object_id: str) -> GameObject | None:
        """Récupère un objet par UUID dans cette scène."""
        ...

    def find_object(self, name: str) -> GameObject | None:
        """Trouve le premier objet dont le nom correspond exactement."""
        ...


class UIAnchor:
    """Ancre servant à positionner un élément du HUD dans le viewport."""
    ...


class UIAnchorSet:
    TOP_LEFT: UIAnchor
    TOP_CENTER: UIAnchor
    TOP_RIGHT: UIAnchor
    CENTER_LEFT: UIAnchor
    CENTER: UIAnchor
    CENTER_RIGHT: UIAnchor
    BOTTOM_LEFT: UIAnchor
    BOTTOM_CENTER: UIAnchor
    BOTTOM_RIGHT: UIAnchor


class GameUI:
    def set_text(
        self,
        id: str,
        text: str,
        x: float = 0.0,
        y: float = 0.0,
        anchor: UIAnchor | None = None,
        font_size: float = 24.0,
        color: tuple[int, int, int, int] | None = None,
        visible: bool = True,
    ) -> None:
        """Crée ou met à jour un texte HUD, visible par défaut."""
        ...

    def update_text(self, id: str, text: str) -> None: ...
    def update_position(self, id: str, x: float, y: float) -> None: ...
    def update_anchor(self, id: str, anchor: UIAnchor) -> None: ...
    def update_font_size(self, id: str, font_size: float) -> None: ...
    def update_color(self, id: str, color: tuple[int, int, int, int]) -> None: ...
    def fade_out(self, id: str, duration: float) -> None:
        """Fait disparaître le texte progressivement puis le supprime."""
        ...
    def fade_in(self, id: str, duration: float) -> None:
        """Fait apparaître progressivement un texte existant."""
        ...

    def remove_text(self, id: str) -> None:
        """Supprime un texte HUD."""
        ...


class GameContext:
    """
    Contexte transmis aux hooks du monde et aux scripts attachés aux objets.

    Instance créée par le moteur Rust (via PyO3) — ne pas instancier manuellement.
    """

    @property
    def delta_time(self) -> float:
        """Temps écoulé depuis le dernier frame, en secondes."""
        ...

    @property
    def ui(self) -> GameUI:
        """API des textes HUD du jeu."""
        ...

    @property
    def ui_anchor(self) -> UIAnchorSet:
        """Ancres disponibles pour positionner un élément HUD."""
        ...

    @property
    def object_id(self) -> str | None:
        """UUID de l'objet lié au script, ou None pour le script du monde."""
        ...

    @property
    def name(self) -> str:
        """Nom de l'objet scripté."""
        ...

    @property
    def object_type(self) -> str:
        """Type de l'objet (lecture seule)."""
        ...

    @property
    def has_collisions(self) -> bool:
        """Indique si les collisions sont activées."""
        ...

    @property
    def hidden(self) -> bool:
        """Indique si l'objet est masqué."""
        ...

    @property
    def fit_texture_to_diamond(self) -> bool:
        """Indique si la texture est ajustée au losange."""
        ...

    @property
    def rotation(self) -> str:
        """Orientation : North, East, South ou West."""
        ...

    @property
    def layer(self) -> str:
        """Couche de rendu de l'objet."""
        ...

    @property
    def texture(self) -> str | None:
        """Chemin de texture, ou None si aucune texture n'est définie."""
        ...

    @property
    def script(self) -> str | None:
        """Chemin du script associé, ou None."""
        ...

    @property
    def camera_zoom(self) -> float:
        """Zoom de l'objet caméra ; valeur par défaut 1.0."""
        ...

    @property
    def camera_interpolation(self) -> bool:
        """Indique si le suivi de cette caméra est interpolé."""
        ...

    @property
    def camera_interpolation_speed(self) -> float:
        """Vitesse d'interpolation de la caméra, entre 0.1 et 20.0."""
        ...

    @property
    def x(self) -> int:
        """Coordonnée X actuelle sur la grille (script d'objet uniquement)."""
        ...

    @property
    def y(self) -> int:
        """Coordonnée Y actuelle sur la grille (script d'objet uniquement)."""
        ...

    @property
    def z(self) -> int:
        """Étage Z actuel sur la grille (script d'objet uniquement)."""
        ...

    @property
    def keys(self) -> KeySet:
        """Constantes pour les fonctions d'entrée clavier (ex. game.keys.Z)."""
        ...

    def key_pressed(self, key: Key) -> bool:
        """True tant que la touche est maintenue (ex. game.keys.Z)."""
        ...

    def key_just_pressed(self, key: Key) -> bool:
        """True pendant le callback du frame où la touche vient d'être pressée."""
        ...

    def key_just_released(self, key: Key) -> bool:
        """True pendant le callback du frame où la touche vient d'être relâchée."""
        ...

    def key_released(self, key: Key) -> bool:
        """Alias de key_just_released."""
        ...

    def move_self(self, x: int, y: int) -> None:
        """Déplace l'objet lié au script. Valide uniquement pour un script d'objet."""
        ...

    def destroy_self(self) -> None:
        """Supprime l'objet lié au script. Valide uniquement pour un script d'objet."""
        ...

    def set_self_property(self, key: str, value: Any) -> None:
        """Modifie un champ éditable de l'objet ou une propriété personnalisée."""
        ...

    def get_self_property(self, key: str) -> Any:
        """Lit un champ d'objet ou une propriété personnalisée ; None si absent."""
        ...

    def get_object(self, object_id: str) -> GameObject | None:
        """Récupère un instantané en lecture seule par UUID."""
        ...

    def find_object(self, name: str) -> GameObject | None:
        """Trouve le premier objet de nom exact, ou None si absent."""
        ...

    def get_self_object(self) -> GameObject:
        """Retourne l'objet portant ce script ; erreur si le script est celui du monde."""
        ...

    def require_object(self, name: str) -> GameObject:
        """Trouve l'objet par nom exact, ou lève une erreur s'il est absent."""
        ...

    def get_distance(self, object1: GameObject, object2: GameObject) -> float:
        """Distance euclidienne 2D en tuiles ; la coordonnée Z est ignorée."""
        ...

    def get_scene(self, name: str) -> GameScene | None:
        """Récupère une scène par son nom exact, sans l'activer."""
        ...

    # Événement disponible en Lecture :
    # def _on_click(game: GameContext) -> None: ...
    # Hooks disponibles sur le script d'un objet qui sert de zone :
    # def _on_area_entered(game: GameContext, other: GameObject) -> None: ...
    # def _on_area_stayed(game: GameContext, other: GameObject) -> None: ...
    # def _on_area_exited(game: GameContext, other: GameObject) -> None: ...

    def spawn(
        self,
        name: str,
        object_type: str,
        x: int,
        y: int,
    ) -> None:
        """
        Crée un objet dans la scène.

        Args:
            name: Nom de l'objet (affiché dans l'éditeur).
            object_type: "Building", "Character", "Item", "Decoration", ou nom personnalisé.
            x: Position X sur la grille.
            y: Position Y sur la grille.
        """
        ...

    def destroy(self, object_id: str) -> None:
        """
        Supprime un objet par son UUID.

        Args:
            object_id: UUID de l'objet (chaîne).
        """
        ...

    def move_object(self, object_id: str, x: int, y: int) -> None:
        """
        Déplace un objet existant.

        Args:
            object_id: UUID de l'objet.
            x, y: Nouvelle position sur la grille.
        """
        ...

    def set_property(self, object_id: str, key: str, value: Any) -> None:
        """
        Définit une propriété personnalisée sur un objet.

        Types supportés : bool, int, float, str, list, dict.

        Args:
            object_id: UUID de l'objet.
            key: Nom de la propriété.
            value: Valeur (types de base Python uniquement).
        """
        ...

    def set_global(self, key: str, value: Any) -> None:
        """
        Définit immédiatement une valeur partagée pendant la session de jeu.

        Args:
            key: Clé de la valeur.
            value: Valeur (types de base Python uniquement).
        """
        ...

    def get_global(self, key: str) -> Any:
        """Lit une valeur partagée, ou None si la clé n'existe pas."""
        ...

    def load_scene(self, name: str) -> None:
        """Charge la scène du monde portant ce nom et empile la scène courante."""
        ...

    def unload_scene(self) -> None:
        """Quitte la scène active et revient à la précédente, si elle existe."""
        ...

    def transfer_object(self, object_id: str, scene_name: str, x: int, y: int) -> None:
        """Déplace l'objet vers une scène sans l'activer, en conservant son identifiant et ses propriétés."""
        ...

    def log(self, message: str) -> None:
        """Émet un message de log (niveau Info) visible dans le panneau Log de l'éditeur."""
        ...

    def log_warning(self, message: str) -> None:
        """Émet un message de log de niveau Avertissement."""
        ...

    def log_error(self, message: str) -> None:
        """Émet un message de log de niveau Erreur."""
        ...
