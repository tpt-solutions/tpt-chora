use std::fmt::Write;

#[derive(Debug, thiserror::Error)]
pub enum DoctorError {
    #[error("wgpu: {0}")]
    Wgpu(String),
    #[error("toolchain: {0}")]
    Toolchain(String),
}

pub fn run() -> Result<(), DoctorError> {
    println!("tpt-chora doctor\n");

    report_toolchain();
    report_gpu();

    Ok(())
}

fn report_toolchain() {
    println!("toolchain:");
    println!(
        "  rustc: {}",
        std::process::Command::new("rustc")
            .arg("--version")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|e| format!("(unable to run: {e})"))
    );
    println!(
        "  cargo: {}",
        std::process::Command::new("cargo")
            .arg("--version")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|e| format!("(unable to run: {e})"))
    );
    println!("  target: {}", std::env::var("TARGET").unwrap_or_else(|_| std::env::consts::ARCH.to_string()));
    println!("  host: {}", std::env::var("HOST").unwrap_or_else(|_| {
        #[cfg(target_os = "windows")]
        { "x86_64-pc-windows-msvc".to_string() }
        #[cfg(target_os = "macos")]
        { "aarch64-apple-darwin".to_string() }
        #[cfg(target_os = "linux")]
        { "x86_64-unknown-linux-gnu".to_string() }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        { std::env::consts::ARCH.to_string() }
    }));

    let rustup_host = std::env::var("HOST").unwrap_or_default();
    let target = std::env::var("TARGET").unwrap_or_default();
    if !rustup_host.is_empty() && !target.is_empty() && rustup_host != target {
        println!("  note: cross-compilation detected (host={rustup_host}, target={target})");
    }

    println!();
}

fn report_gpu() {
    println!("gpu:");

    match pollster::block_on(async {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await;

        let (adapter, is_fallback) = match adapter {
            Some(a) => (a, false),
            None => {
                let fallback = instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference: wgpu::PowerPreference::default(),
                        compatible_surface: None,
                        force_fallback_adapter: true,
                    })
                    .await
                    .ok_or_else(|| DoctorError::Wgpu("no adapter found (hardware or software)".into()))?;
                (fallback, true)
            }
        };

        let info = adapter.get_info();
        let mut out = String::new();

        writeln!(out, "  backend: {:?}", info.backend).unwrap();
        writeln!(out, "  device: {}", info.device).unwrap();
        writeln!(out, "  vendor: {:04x}", info.vendor).unwrap();
        writeln!(out, "  device_type: {:?}", info.device_type).unwrap();
        writeln!(out, "  driver: {}", info.driver).unwrap();
        writeln!(out, "  driver_info: {}", info.driver_info).unwrap();
        writeln!(out, "  adapter_name: {}", info.name).unwrap();

        if is_fallback {
            writeln!(out, "  warning: using Tier 1 software-fallback adapter (no hardware GPU found)").unwrap();
        } else {
            writeln!(out, "  tier: hardware (Tier 3)").unwrap();
        }

        Ok::<_, DoctorError>(out)
    }) {
        Ok(report) => print!("{report}"),
        Err(e) => println!("  error: {e}"),
    }

    println!();
}
