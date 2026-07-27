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
        match path.extension().and_then(|e| e.to_str()) {
            Some("eidos") => Some(ReloadEvent::EidosFileChanged(path.to_path_buf())),
            Some("wgsl") | Some("glsl") | Some("spirv") => {
                Some(ReloadEvent::ShaderFileChanged(path.to_path_buf()))
            }
            Some("png") | Some("jpg") | Some("jpeg") | Some("webp") | Some("svg") => {
                Some(ReloadEvent::AssetChanged(path.to_path_buf()))
            }
            _ => None,
        }
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

impl Default for HotReloader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::thread;
    use std::time::Duration;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("tpt_chora_test_{}", name))
    }

    fn create_temp_file(name: &str, contents: &[u8]) -> PathBuf {
        let p = temp_path(name);
        fs::write(&p, contents).unwrap();
        p
    }

    fn cleanup(path: &Path) {
        let _ = fs::remove_file(path);
    }

    #[test]
    fn new_creates_empty_reloader() {
        let r = HotReloader::new();
        assert_eq!(r.watched_count(), 0);
        assert_eq!(r.pending_count(), 0);
    }

    #[test]
    fn watch_increases_count() {
        let p = create_temp_file("watch_basic.dat", b"hello");
        let mut r = HotReloader::new();
        r.watch(p.clone());
        assert_eq!(r.watched_count(), 1);
        cleanup(&p);
    }

    #[test]
    fn unwatch_decreases_count() {
        let p = create_temp_file("unwatch_basic.dat", b"hello");
        let mut r = HotReloader::new();
        r.watch(p.clone());
        assert_eq!(r.watched_count(), 1);
        r.unwatch(&p);
        assert_eq!(r.watched_count(), 0);
        assert!(!r.last_modified.contains_key(&p));
        cleanup(&p);
    }

    #[test]
    fn process_change_eidos() {
        let mut r = HotReloader::new();
        let p = PathBuf::from("/fake/file.eidos");
        let event = r.process_change(&p).unwrap();
        match event {
            ReloadEvent::EidosFileChanged(path) => assert_eq!(path, p),
            _ => panic!("expected EidosFileChanged"),
        }
    }

    #[test]
    fn process_change_wgsl() {
        let mut r = HotReloader::new();
        let p = PathBuf::from("/fake/shader.wgsl");
        let event = r.process_change(&p).unwrap();
        match event {
            ReloadEvent::ShaderFileChanged(path) => assert_eq!(path, p),
            _ => panic!("expected ShaderFileChanged"),
        }
    }

    #[test]
    fn process_change_png() {
        let mut r = HotReloader::new();
        let p = PathBuf::from("/fake/image.png");
        let event = r.process_change(&p).unwrap();
        match event {
            ReloadEvent::AssetChanged(path) => assert_eq!(path, p),
            _ => panic!("expected AssetChanged"),
        }
    }

    #[test]
    fn process_change_unknown_extension() {
        let mut r = HotReloader::new();
        let p = PathBuf::from("/fake/readme.txt");
        assert!(r.process_change(&p).is_none());
    }

    #[test]
    fn poll_events_no_change() {
        let p = create_temp_file("poll_nochange.eidos", b"data");
        let mut r = HotReloader::new();
        r.watch(p.clone());
        let events = r.poll_events();
        assert!(events.is_empty());
        assert_eq!(r.pending_count(), 0);
        cleanup(&p);
    }

    #[test]
    fn poll_events_detects_change() {
        let p = create_temp_file("poll_change.eidos", b"before");
        let mut r = HotReloader::new();
        r.watch(p.clone());

        thread::sleep(Duration::from_millis(1100));
        fs::write(&p, b"after").unwrap();

        let events = r.poll_events();
        assert_eq!(events.len(), 1);
        assert_eq!(r.pending_count(), 1);
        cleanup(&p);
    }

    #[test]
    fn drain_pending_returns_all_and_clears() {
        let p = create_temp_file("drain.eidos", b"before");
        let mut r = HotReloader::new();
        r.watch(p.clone());

        thread::sleep(Duration::from_millis(1100));
        fs::write(&p, b"after").unwrap();
        r.poll_events();

        let drained = r.drain_pending();
        assert_eq!(drained.len(), 1);
        assert_eq!(r.pending_count(), 0);
        cleanup(&p);
    }
}
