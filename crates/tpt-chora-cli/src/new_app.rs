use std::fs;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum NewError {
    #[error("directory '{0}' already exists")]
    DirectoryExists(String),
    #[error("failed to create directory: {0}")]
    CreateDir(String),
    #[error("failed to write file: {0}")]
    WriteFile(String),
}

pub fn run(name: &str) -> Result<(), NewError> {
    let dir = Path::new(name);
    if dir.exists() {
        return Err(NewError::DirectoryExists(name.to_string()));
    }

    fs::create_dir_all(dir).map_err(|e| NewError::CreateDir(e.to_string()))?;
    fs::create_dir_all(dir.join("src")).map_err(|e| NewError::CreateDir(e.to_string()))?;

    write_file(
        &dir.join("Cargo.toml"),
        &format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"
license = "Apache-2.0 OR MIT"

[dependencies]
tpt-chora-render = {{ version = "0.1" }}
"#
        ),
    )?;

    write_file(
        &dir.join("src/main.rs"),
        r#"fn main() {
    println!("tpt-chora: minimal scaffold — replace with your render logic");
    let renderer = tpt_chora_render::Renderer::new_headless(800, 600)
        .expect("failed to create renderer");
    let pixels = renderer.render_frame()
        .expect("failed to render frame");
    println!("rendered {} bytes of pixel data", pixels.len());
}
"#,
    )?;

    write_file(
        &dir.join("starter.eidos"),
        &format!(
            r#"// {name} — starter .eidos scene description
// See spec.txt for the full Eidos IR reference.

component "root" {{
  width := "800";
  height := "600";
  background := "#0a0a12";
}}
"#
        ),
    )?;

    write_file(
        &dir.join(".gitignore"),
        "target/\n",
    )?;

    println!("created project `{name}` in ./{name}/");
    println!();
    println!("next steps:");
    println!("  cd {name}");
    println!("  cargo run");

    Ok(())
}

fn write_file(path: &Path, contents: &str) -> Result<(), NewError> {
    fs::write(path, contents).map_err(|e| NewError::WriteFile(format!("{}: {e}", path.display())))
}
