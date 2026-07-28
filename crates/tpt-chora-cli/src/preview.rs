//! `tpt-chora preview <project-dir>`: watches a project's `.eidos`/shader/
//! asset files (via `tpt_chora_inspector::HotReloader`, the same file-change
//! detection Phase 10's hot reloading uses) and re-renders a PNG snapshot
//! on every change — a live dev-loop feedback cycle without needing a
//! windowed app or the full runtime.

use std::path::{Path, PathBuf};
use std::time::Duration;

use tpt_chora_inspector::hot_reload::{HotReloader, ReloadEvent};
use tpt_chora_render::Renderer;

#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    #[error("project directory '{0}' does not exist or is not a directory")]
    InvalidProjectDir(String),
    #[error("failed to create headless renderer: {0}")]
    Renderer(String),
    #[error("failed to render frame: {0}")]
    Render(String),
    #[error("failed to write preview PNG: {0}")]
    WritePng(String),
}

const WATCHED_EXTENSIONS: &[&str] = &[
    "eidos", "wgsl", "glsl", "spirv", "png", "jpg", "jpeg", "webp", "svg",
];
const PREVIEW_WIDTH: u32 = 800;
const PREVIEW_HEIGHT: u32 = 600;

/// Walks `root` for files with a watched extension, skipping `target/`
/// (build output, not project source).
fn collect_watch_targets(root: &Path) -> Vec<PathBuf> {
    let mut targets = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().and_then(|n| n.to_str()) == Some("target") {
                    continue;
                }
                stack.push(path);
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| WATCHED_EXTENSIONS.contains(&e))
            {
                targets.push(path);
            }
        }
    }

    targets
}

fn render_snapshot(out_path: &Path) -> Result<(), PreviewError> {
    let renderer = Renderer::new_headless(PREVIEW_WIDTH, PREVIEW_HEIGHT)
        .map_err(|e| PreviewError::Renderer(e.to_string()))?;
    let pixels = renderer
        .render_frame()
        .map_err(|e| PreviewError::Render(e.to_string()))?;

    let file =
        std::fs::File::create(out_path).map_err(|e| PreviewError::WritePng(e.to_string()))?;
    let writer = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, PREVIEW_WIDTH, PREVIEW_HEIGHT);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|e| PreviewError::WritePng(e.to_string()))?;
    writer
        .write_image_data(&pixels)
        .map_err(|e| PreviewError::WritePng(e.to_string()))?;
    Ok(())
}

fn describe_event(event: &ReloadEvent) -> String {
    match event {
        ReloadEvent::EidosFileChanged(p) => format!("eidos:  {}", p.display()),
        ReloadEvent::ShaderFileChanged(p) => format!("shader: {}", p.display()),
        ReloadEvent::AssetChanged(p) => format!("asset:  {}", p.display()),
    }
}

pub fn run(project_dir: &str) -> Result<(), PreviewError> {
    let root = Path::new(project_dir);
    if !root.is_dir() {
        return Err(PreviewError::InvalidProjectDir(project_dir.to_string()));
    }

    let mut reloader = HotReloader::new();
    for target in collect_watch_targets(root) {
        reloader.watch(target);
    }
    println!(
        "tpt-chora preview: watching {} file(s) under {}",
        reloader.watched_count(),
        root.display()
    );

    let preview_dir = root.join(".tpt-chora-preview");
    std::fs::create_dir_all(&preview_dir).map_err(|e| PreviewError::WritePng(e.to_string()))?;
    let out_path = preview_dir.join("preview.png");

    render_snapshot(&out_path)?;
    println!("  initial render -> {}", out_path.display());
    println!("watching for changes (Ctrl+C to stop)...");

    loop {
        std::thread::sleep(Duration::from_millis(500));
        let events = reloader.poll_events();
        if events.is_empty() {
            continue;
        }

        for event in &events {
            println!("  changed: {}", describe_event(event));
        }
        render_snapshot(&out_path)?;
        println!("  re-rendered -> {}", out_path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_project_dir_errors_before_watching() {
        let err = run("/this/path/does/not/exist").unwrap_err();
        assert!(matches!(err, PreviewError::InvalidProjectDir(_)));
    }

    #[test]
    fn collect_watch_targets_finds_eidos_and_skips_target_dir() {
        let dir = std::env::temp_dir().join("tpt_chora_preview_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("target")).unwrap();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("starter.eidos"), "// scene").unwrap();
        std::fs::write(dir.join("src/main.rs"), "fn main() {}").unwrap();
        std::fs::write(dir.join("target/ignored.eidos"), "// build output").unwrap();

        let targets = collect_watch_targets(&dir);
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0], dir.join("starter.eidos"));

        let _ = std::fs::remove_dir_all(&dir);
    }
}
