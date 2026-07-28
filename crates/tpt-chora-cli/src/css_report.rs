use std::fs;

use tpt_chora_compat::{CssParser, EidosTranspiler, ViolationReason};

#[derive(Debug, thiserror::Error)]
pub enum CssReportError {
    #[error("failed to read file: {0}")]
    ReadFile(String),
    #[error("parse error: {0}")]
    Parse(String),
}

pub fn run(path: &str) -> Result<(), CssReportError> {
    let contents = fs::read_to_string(path).map_err(|e| CssReportError::ReadFile(e.to_string()))?;

    let mut parser = CssParser::new(contents);
    let parsed = parser
        .parse()
        .map_err(|e| CssReportError::Parse(e.to_string()))?;

    let rule_count = parsed.rules.len();
    let decl_count: usize = parsed.rules.iter().map(|r| r.declarations.len()).sum();

    let transpiler = EidosTranspiler::new();
    let result = transpiler.transpile(&parsed);

    let violation_count = result.violations.len();
    let auto_count = result.auto_corrections.len();
    let clean = decl_count - violation_count;

    let score = if decl_count == 0 {
        100.0
    } else {
        (clean as f64 / decl_count as f64) * 100.0
    };

    println!("css-report: {path}");
    println!();
    println!("summary:");
    println!("  rules:              {rule_count}");
    println!("  declarations:       {decl_count}");
    println!("  compatibility:      {score:.1}%");
    println!();

    if !result.violations.is_empty() {
        println!("violations ({violation_count}):");
        for v in &result.violations {
            println!(
                "  {} [{}]: {}",
                v.rule_selector,
                v.property,
                reason_label(&v.reason)
            );
        }
        println!();
    }

    if !result.auto_corrections.is_empty() {
        println!("auto-corrections ({auto_count}):");
        for c in &result.auto_corrections {
            println!("  {c}");
        }
        println!();
    }

    if violation_count == 0 && auto_count == 0 {
        println!("no violations found — CSS is fully compatible with Eidos IR");
    }

    Ok(())
}

fn reason_label(reason: &ViolationReason) -> &'static str {
    match reason {
        ViolationReason::TextOverflow => "text-overflow: visible not allowed",
        ViolationReason::UnsafeZIndex => "z-index: must be managed by proof system",
        ViolationReason::AbsolutePositioning => "position: absolute/fixed not allowed",
        ViolationReason::UnboundedWidth => "width: must be bounded",
        ViolationReason::UnboundedHeight => "height: must be bounded",
        ViolationReason::MissingContrast => "contrast: insufficient contrast ratio",
        ViolationReason::FontSizeTooSmall => "font-size: below minimum 10px threshold",
    }
}
