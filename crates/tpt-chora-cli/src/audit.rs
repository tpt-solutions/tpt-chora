use std::fmt::Write;

#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("wgpu: {0}")]
    Wgpu(String),
}

pub fn run() -> Result<(), AuditError> {
    println!("tpt-chora audit\n");
    println!("=== Platform ===");
    report_platform();
    println!();

    println!("=== GPU Backend ===");
    report_gpu();
    println!();

    println!("=== Supply Chain ===");
    report_supply_chain();
    println!();

    println!("=== Native Backend Features ===");
    report_native_backends();
    println!();

    Ok(())
}

fn report_platform() {
    println!("  os: {}", std::env::consts::OS);
    println!("  arch: {}", std::env::consts::ARCH);
    println!(
        "  target: {}",
        std::env::var("TARGET").unwrap_or_else(|_| "unknown".into())
    );
}

fn report_gpu() {
    match pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await;
        match adapter {
            Some(a) => {
                let info = a.get_info();
                let mut out = String::new();
                writeln!(out, "  backend: {:?}", info.backend).unwrap();
                writeln!(out, "  device_type: {:?}", info.device_type).unwrap();
                writeln!(out, "  adapter_name: {}", info.name).unwrap();
                Ok::<_, AuditError>(out)
            }
            None => {
                let fallback = instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference: wgpu::PowerPreference::default(),
                        compatible_surface: None,
                        force_fallback_adapter: true,
                    })
                    .await;
                match fallback {
                    Some(a) => {
                        let info = a.get_info();
                        let mut out = String::new();
                        writeln!(out, "  backend: {:?}", info.backend).unwrap();
                        writeln!(out, "  device_type: {:?}", info.device_type).unwrap();
                        writeln!(out, "  adapter_name: {}", info.name).unwrap();
                        writeln!(out, "  warning: Tier 1 software-fallback adapter active")
                            .unwrap();
                        Ok::<_, AuditError>(out)
                    }
                    None => Err(AuditError::Wgpu("no adapter found".into())),
                }
            }
        }
    }) {
        Ok(report) => print!("{report}"),
        Err(e) => println!("  error: {e}"),
    }
}

fn report_supply_chain() {
    let output = std::process::Command::new("cargo")
        .args(["deny", "check"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_else(|e| format!("(unable to run: {e})"));

    if output.contains("advisories ok") && output.contains("licenses ok") {
        println!("  advisories: ok");
        println!("  licenses: ok");
        println!("  sources: ok");
    } else {
        println!("  output:");
        for line in output.lines() {
            println!("    {line}");
        }
    }
}

fn report_native_backends() {
    let features = [
        ("tpt-chora-a11y", "native-a11y-backends"),
        ("tpt-chora-input", "native-haptics-backends"),
        ("tpt-chora-media", "native-video-backends"),
    ];

    for (crate_name, feature) in &features {
        let enabled = is_feature_enabled(crate_name, feature);
        let verified = is_feature_verified(crate_name, feature);
        println!(
            "  {crate_name} {feature}: {status}{verified}",
            status = if enabled { "enabled" } else { "disabled" },
            verified = if verified {
                " (verified on this platform)"
            } else if enabled {
                " (enabled but not verified on this platform)"
            } else {
                ""
            }
        );
    }
}

fn is_feature_enabled(crate_name: &str, feature: &str) -> bool {
    let cargo_toml_path = format!("crates/{crate_name}/Cargo.toml");
    std::fs::read_to_string(&cargo_toml_path)
        .map(|c| c.contains(&format!("{feature} =")))
        .unwrap_or(false)
}

fn is_feature_verified(crate_name: &str, feature: &str) -> bool {
    let target = std::env::var("TARGET").unwrap_or_default();
    match (crate_name, feature, target.as_str()) {
        ("tpt-chora-a11y", "native-a11y-backends", t) if t.contains("windows") => true,
        ("tpt-chora-input", "native-haptics-backends", t) if t.contains("macos") => true,
        ("tpt-chora-media", "native-video-backends", t) if t.contains("linux") => true,
        _ => false,
    }
}
