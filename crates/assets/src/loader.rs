//! Chargement des ressources depuis le disque.

use anyhow::Result;
use image::DynamicImage;
use std::path::Path;

/// Charge une image PNG/JPEG depuis le disque.
pub fn load_image(path: &Path) -> Result<DynamicImage> {
    let img = image::open(path)
        .map_err(|e| anyhow::anyhow!("Impossible de charger `{}` : {}", path.display(), e))?;
    Ok(img)
}

/// Valide qu'un chemin de ressource est sûr (pas de traversal).
pub fn validate_asset_path(base: &Path, rel: &Path) -> Result<std::path::PathBuf> {
    let candidate = base.join(rel);
    let canonical = candidate
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Chemin invalide `{}` : {}", rel.display(), e))?;
    let base_canonical = base
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Dossier de base invalide : {e}"))?;
    if !canonical.starts_with(&base_canonical) {
        anyhow::bail!(
            "Tentative de traversal de chemin refusée : `{}`",
            rel.display()
        );
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TempTree(PathBuf);

    impl TempTree {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "isomaker_asset_path_{}_{}",
                std::process::id(),
                nonce
            ));
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn validates_existing_project_asset_and_rejects_path_traversal() {
        let tree = TempTree::new();
        let project = tree.0.join("project");
        let assets = project.join("assets");
        fs::create_dir_all(&assets).unwrap();
        fs::write(assets.join("sprite.png"), b"asset").unwrap();
        fs::write(tree.0.join("outside.png"), b"outside").unwrap();

        let resolved = validate_asset_path(&project, Path::new("assets/sprite.png")).unwrap();
        assert_eq!(resolved, assets.join("sprite.png").canonicalize().unwrap());
        assert!(validate_asset_path(&project, Path::new("../outside.png")).is_err());
        assert!(validate_asset_path(&project, Path::new("assets/missing.png")).is_err());
    }
}
