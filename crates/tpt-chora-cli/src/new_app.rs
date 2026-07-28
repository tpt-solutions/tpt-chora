use std::fs;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum NewError {
    #[error(
        "'{0}' is not a valid project name — use only ASCII letters, digits, '-', and '_', \
         starting with a letter (no path separators or '..')"
    )]
    InvalidName(String),
    #[error("directory '{0}' already exists")]
    DirectoryExists(String),
    #[error("failed to create directory: {0}")]
    CreateDir(String),
    #[error("failed to write file: {0}")]
    WriteFile(String),
}

/// Rejects anything that isn't a plain, single-component crate-name-shaped
/// identifier before it's used to build filesystem paths or interpolated
/// into generated file contents — blocks path traversal (`../..`),
/// absolute paths, and names that would break the generated `Cargo.toml`
/// (e.g. embedded quotes/newlines).
fn validate_name(name: &str) -> Result<(), NewError> {
    let valid = !name.is_empty()
        && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');

    if valid {
        Ok(())
    } else {
        Err(NewError::InvalidName(name.to_string()))
    }
}

pub fn run(name: &str) -> Result<(), NewError> {
    validate_name(name)?;

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
            r##"// {name} — starter .eidos scene description
// See spec.txt for the full Eidos IR reference.

component "root" {{
  width := "800";
  height := "600";
  background := "#0a0a12";
}}
"##
        ),
    )?;

    write_file(&dir.join(".gitignore"), "target/\n")?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_name_accepted() {
        assert!(validate_name("my-app").is_ok());
        assert!(validate_name("my_app_2").is_ok());
    }

    #[test]
    fn rejects_path_traversal() {
        assert!(validate_name("../../etc/passwd").is_err());
        assert!(validate_name("..").is_err());
    }

    #[test]
    fn rejects_path_separators() {
        assert!(validate_name("foo/bar").is_err());
        assert!(validate_name("foo\\bar").is_err());
    }

    #[test]
    fn rejects_absolute_path() {
        assert!(validate_name("/tmp/evil").is_err());
    }

    #[test]
    fn rejects_empty_and_non_letter_start() {
        assert!(validate_name("").is_err());
        assert!(validate_name("123app").is_err());
    }

    #[test]
    fn rejects_embedded_quotes_and_newlines() {
        assert!(validate_name("foo\"\ninjected").is_err());
    }

    #[test]
    fn run_rejects_invalid_name_before_touching_filesystem() {
        let err = run("../should-not-be-created").unwrap_err();
        assert!(matches!(err, NewError::InvalidName(_)));
    }
}
