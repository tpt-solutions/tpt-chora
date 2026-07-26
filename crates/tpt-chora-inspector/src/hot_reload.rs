use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub struct HotReloader {
    watched_paths: HashSet<PathBuf>,
    last_modified: HashMap<PathBuf, SystemTime>,
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
            last_modified: HashMap::new(),
            pending_events: Vec::new(),
        }
    }

    pub fn watch(&mut self, path: PathBuf) {
        if let Ok(metadata) = std::fs::metadata(&path) {
            if let Ok(modified) = metadata.modified() {
                self.last_modified.insert(path.clone(), modified);
            }
        }
        self.watched_paths.insert(path);
    }

    pub fn unwatch(&mut self, path: &Path) {
        self.watched_paths.remove(path);
        self.last_modified.remove(path);
    }

    pub fn poll_events(&mut self) -> Vec<ReloadEvent> {
        let mut new_events = Vec::new();
        let paths: Vec<PathBuf> = self.watched_paths.iter().cloned().collect();

        for path in &paths {
            if let Ok(metadata) = std::fs::metadata(path) {
                if let Ok(modified) = metadata.modified() {
                    let changed = self
                        .last_modified
                        .get(path)
                        .map(|&prev| modified > prev)
                        .unwrap_or(true);

                    if changed {
                        if let Some(event) = self.process_change(path) {
                            new_events.push(event);
                        }
                        self.last_modified.insert(path.clone(), modified);
                    }
                }
            }
        }

        self.pending_events.extend(new_events.clone());
        new_events
    }

    pub fn process_change(&mut self, path: &Path) -> Option<ReloadEvent> {
        let event = match path.extension().and_then(|e| e.to_str()) {
            Some("eidos") => Some(ReloadEvent::EidosFileChanged(path.to_path_buf())),
            Some("wgsl") | Some("glsl") | Some("spirv") => {
                Some(ReloadEvent::ShaderFileChanged(path.to_path_buf()))
            }
            Some("png") | Some("jpg") | Some("jpeg") | Some("webp") | Some("svg") => {
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

    pub fn drain_pending(&mut self) -> Vec<ReloadEvent> {
        std::mem::take(&mut self.pending_events)
    }
}
