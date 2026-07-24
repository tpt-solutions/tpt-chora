use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub struct HotReloader {
    watched_paths: HashSet<PathBuf>,
    pending_events: Vec<ReloadEvent>,
}

#[derive(Debug, Clone)]
pub enum ReloadEvent {
    EidosFileChanged(PathBuf),
    ShaderFileChanged(PathBuf),
    AssetChanged(PathBuf),
}

impl HotReloader {
    pub fn new() -> Self {
        Self {
            watched_paths: HashSet::new(),
            pending_events: Vec::new(),
        }
    }

    pub fn watch(&mut self, path: PathBuf) {
        self.watched_paths.insert(path);
    }

    pub fn unwatch(&mut self, path: &Path) {
        self.watched_paths.remove(path);
    }

    pub fn poll_events(&mut self) -> Vec<ReloadEvent> {
        let events = self.pending_events.drain(..).collect();
        events
    }

    pub fn process_change(&mut self, path: &Path) -> Option<ReloadEvent> {
        let event = match path.extension().and_then(|e| e.to_str()) {
            Some("eidos") => Some(ReloadEvent::EidosFileChanged(path.to_path_buf())),
            Some("wgsl") | Some("glsl") | Some("spirv") => {
                Some(ReloadEvent::ShaderFileChanged(path.to_path_buf()))
            }
            Some("png") | Some("jpg") | Some("webp") | Some("svg") => {
                Some(ReloadEvent::AssetChanged(path.to_path_buf()))
            }
            _ => None,
        };

        if let Some(ref evt) = event {
            self.pending_events.push(evt.clone());
        }

        event
    }

    pub fn watched_count(&self) -> usize {
        self.watched_paths.len()
    }

    pub fn pending_count(&self) -> usize {
        self.pending_events.len()
    }
}
