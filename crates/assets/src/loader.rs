//! Chargement des ressources depuis le disque.

use std::path::Path;
use anyhow::Result;
use image::DynamicImage;

/// Charge une image PNG/JPEG depuis le disque.
pub fn load_image(path: &Path) -> Result<DynamicImage> {
    let img = image::open(path)
        .map_err(|e| anyhow::anyhow!("Impossible de charger `{}` : {}", path.display(), e))?;
    Ok(img)
}

/// Valide qu'un chemin de ressource est sûr (pas de traversal).
pub fn validate_asset_path(base: &Path, rel: &Path) -> Result<std::path::PathBuf> {
    let candidate = base.join(rel);
    let canonical = candidate.canonicalize()
        .map_err(|e| anyhow::anyhow!("Chemin invalide `{}` : {}", rel.display(), e))?;
    let base_canonical = base.canonicalize()
        .map_err(|e| anyhow::anyhow!("Dossier de base invalide : {e}"))?;
    if !canonical.starts_with(&base_canonical) {
        anyhow::bail!(
            "Tentative de traversal de chemin refusée : `{}`",
            rel.display()
        );
    }
    Ok(canonical)
}
