use crate::css_parser::{ParsedCss, CssRule, CssDeclaration};

pub struct EidosTranspiler {
    safety_checks: bool,
}

#[derive(Debug, Clone)]
pub struct TranspileResult {
    pub eidos_ir: String,
    pub violations: Vec<Violation>,
    pub auto_corrections: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Violation {
    pub rule_selector: String,
    pub property: String,
    pub reason: ViolationReason,
    pub auto_corrected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViolationReason {
    TextOverflow,
    UnsafeZIndex,
    AbsolutePositioning,
    UnboundedWidth,
    UnboundedHeight,
    MissingContrast,
    FontSizeTooSmall,
}

impl EidosTranspiler {
    pub fn new() -> Self {
        Self {
            safety_checks: true,
        }
    }

    pub fn with_safety_checks(mut self, enabled: bool) -> Self {
        self.safety_checks = enabled;
        self
    }

    pub fn transpile(&self, css: &ParsedCss) -> TranspileResult {
        let mut eidos_ir = String::from("// Auto-generated Chora-IR from CSS\n\n");
        let mut violations = Vec::new();
        let mut auto_corrections = Vec::new();

        for rule in &css.rules {
            let mut node_ir = format!("component \"{}\" {{\n", rule.selector);

            for decl in &rule.declarations {
                if self.safety_checks {
                    if let Some(violation) = self.check_violation(rule, decl) {
                        let corrected_value = self.auto_correct(&violation, decl);
                        if let Some(corrected) = corrected_value {
                            auto_corrections.push(format!(
                                "{}.{}: auto-corrected '{}' to '{}'",
                                rule.selector, decl.property, decl.value, corrected
                            ));
                            node_ir.push_str(&format!(
                                "  {} := \"{}\";\n",
                                decl.property, corrected
                            ));
                        } else {
                            violations.push(violation);
                            node_ir.push_str(&format!(
                                "  // SAFETY: {} = \"{}\" (would violate proof)\n",
                                decl.property, decl.value
                            ));
                        }
                        continue;
                    }
                }

                node_ir.push_str(&format!(
                    "  {} := \"{}\";\n",
                    decl.property, decl.value
                ));
            }

            node_ir.push_str("}\n\n");
            eidos_ir.push_str(&node_ir);
        }

        TranspileResult {
            eidos_ir,
            violations,
            auto_corrections,
        }
    }

    fn check_violation(
        &self,
        rule: &CssRule,
        decl: &CssDeclaration,
    ) -> Option<Violation> {
        match decl.property.as_str() {
            "overflow" if decl.value == "visible" => Some(Violation {
                rule_selector: rule.selector.clone(),
                property: decl.property.clone(),
                reason: ViolationReason::TextOverflow,
                auto_corrected: false,
            }),
            "z-index" => Some(Violation {
                rule_selector: rule.selector.clone(),
                property: decl.property.clone(),
                reason: ViolationReason::UnsafeZIndex,
                auto_corrected: false,
            }),
            "position" if decl.value == "absolute" || decl.value == "fixed" => {
                Some(Violation {
                    rule_selector: rule.selector.clone(),
                    property: decl.property.clone(),
                    reason: ViolationReason::AbsolutePositioning,
                    auto_corrected: false,
                })
            }
            "width" if decl.value == "auto" || decl.value == "100vw" => {
                Some(Violation {
                    rule_selector: rule.selector.clone(),
                    property: decl.property.clone(),
                    reason: ViolationReason::UnboundedWidth,
                    auto_corrected: false,
                })
            }
            "height" if decl.value == "auto" || decl.value == "100vh" => {
                Some(Violation {
                    rule_selector: rule.selector.clone(),
                    property: decl.property.clone(),
                    reason: ViolationReason::UnboundedHeight,
                    auto_corrected: false,
                })
            }
            "font-size" => {
                if let Some(size) = self.parse_font_size(&decl.value) {
                    if size < 10.0 {
                        return Some(Violation {
                            rule_selector: rule.selector.clone(),
                            property: decl.property.clone(),
                            reason: ViolationReason::FontSizeTooSmall,
                            auto_corrected: false,
                        });
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn auto_correct(&self, violation: &Violation, _decl: &CssDeclaration) -> Option<String> {
        match violation.reason {
            ViolationReason::TextOverflow => Some("hidden".to_string()),
            ViolationReason::FontSizeTooSmall => Some("10px".to_string()),
            ViolationReason::UnboundedWidth => Some("100%".to_string()),
            ViolationReason::UnboundedHeight => Some("auto".to_string()),
            _ => None,
        }
    }

    fn parse_font_size(&self, value: &str) -> Option<f32> {
        let cleaned = value.trim().to_lowercase();
        cleaned
            .strip_suffix("px")
            .and_then(|s| s.trim().parse::<f32>().ok())
    }
}
