//! Phase 4 milestone: demonstrate the haptic feedback patterns
//! available through the input engine, showing how each platform
//! backend translates a HapticPattern into platform-specific
//! vibration sequences.
//!
//! Run with: `cargo run -p tpt-chora-input --example haptic_patterns`

use tpt_chora_input::devices::MouseButton;
use tpt_chora_input::{DeviceEvent, HapticPattern, HapticRouter};

fn main() {
    let router = HapticRouter::new();

    let patterns = [
        ("Light tap", HapticPattern::Light),
        ("Medium tap", HapticPattern::Medium),
        ("Heavy impact", HapticPattern::Heavy),
        ("Selection click", HapticPattern::Selection),
        ("Success", HapticPattern::Success),
        ("Warning", HapticPattern::Warning),
        ("Error", HapticPattern::Error),
    ];

    println!("=== Haptic Patterns ===");
    println!("Platform: {:?}", std::env::consts::OS);
    println!();

    for (name, pattern) in &patterns {
        println!("{}:", name);
        let events = HapticRouter::translate_pattern(pattern);
        for event in &events {
            println!(
                "  intensity={:.2} duration={}ms delay={}ms",
                event.intensity, event.duration_ms, event.delay_ms
            );
        }
        println!();
    }

    println!("=== Routing ===");
    let test_events = [
        DeviceEvent::MouseDown {
            button: MouseButton::Left,
            x: 100.0,
            y: 200.0,
        },
        DeviceEvent::TouchBegin {
            id: 1,
            x: 50.0,
            y: 75.0,
        },
    ];

    for event in &test_events {
        if let Some(pattern) = router.route_event(event) {
            println!("{:?} -> {:?}", event, pattern);
        }
    }
}
