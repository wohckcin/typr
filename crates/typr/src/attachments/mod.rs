//! Pictures a card refers to: copied in beside the library, and read back without escaping it.

use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use rune::{Image, ImageCache, Waker};

/// The attachments directory and the decoded pictures drawn from it.
#[derive(Clone)]
pub struct Attachments {
    root: PathBuf,
    cache: Option<Arc<ImageCache>>,
}

impl std::fmt::Debug for Attachments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Attachments")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl Default for Attachments {
    fn default() -> Self {
        Attachments::detached(PathBuf::new())
    }
}

impl Attachments {
    /// Pictures stored under `root`, decoded on Rune's image threads.
    pub fn new(root: PathBuf, waker: Waker) -> Self {
        Attachments {
            root,
            cache: Some(Arc::new(ImageCache::new(waker))),
        }
    }

    /// A directory that holds nothing: every reference is missing rather than resolved.
    pub fn detached(root: PathBuf) -> Self {
        Attachments { root, cache: None }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The file a reference names, or `None` when it escapes the directory or is not there.
    pub fn resolve(&self, src: &str) -> Option<PathBuf> {
        let relative = Path::new(src.trim());
        if self.root.as_os_str().is_empty() || src.trim().is_empty() {
            return None;
        }
        if relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_) | Component::CurDir))
        {
            return None;
        }
        let path = self.root.join(relative);
        if !path.is_file() {
            return None;
        }
        let root = self.root.canonicalize().ok()?;
        let path = path.canonicalize().ok()?;
        path.starts_with(&root).then_some(path)
    }

    /// The decoded picture for a source as written, or `None` so the renderer draws its
    /// placeholder and the alt text.
    pub fn picture(&self, src: &str) -> Option<Image> {
        let cache = self.cache.as_ref()?;
        let path = self.resolve(src)?;
        Some(cache.file(path, 0.0, 1.0))
    }
}

#[cfg(test)]
mod tests;
