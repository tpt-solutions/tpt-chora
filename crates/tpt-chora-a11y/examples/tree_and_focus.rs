//! Phase 5 milestone: build an accessibility tree and exercise focus
//! traversal, then print the tree and the focus order to stdout so the
//! output can be inspected without a screen reader.
//!
//! Run with: `cargo run -p tpt-chora-a11y --example tree_and_focus`

use tpt_chora_a11y::{
    AccessibilityRole, AccessibilityState, FocusDirection, FocusTraversal, SemanticIR, SemanticNode,
};

fn main() {
    let mut ir = SemanticIR::new();

    let root_id = ir.add_node(SemanticNode {
        id: tpt_chora_a11y::SemanticNodeId(1),
        role: AccessibilityRole::Generic,
        label: "main".into(),
        description: String::new(),
        state: AccessibilityState::empty(),
        bounds: [0.0, 0.0, 512.0, 512.0],
        children: vec![],
        parent: None,
        z_depth: 0.0,
    });

    let heading_id = ir.add_node(SemanticNode {
        id: tpt_chora_a11y::SemanticNodeId(2),
        role: AccessibilityRole::Heading,
        label: "Title".into(),
        description: String::new(),
        state: AccessibilityState::empty(),
        bounds: [0.0, 0.0, 512.0, 48.0],
        children: vec![],
        parent: Some(root_id),
        z_depth: 1.0,
    });

    let button_id = ir.add_node(SemanticNode {
        id: tpt_chora_a11y::SemanticNodeId(3),
        role: AccessibilityRole::Button,
        label: "Submit".into(),
        description: String::new(),
        state: AccessibilityState::empty(),
        bounds: [0.0, 48.0, 512.0, 48.0],
        children: vec![],
        parent: Some(root_id),
        z_depth: 1.0,
    });

    ir.add_node(SemanticNode {
        id: tpt_chora_a11y::SemanticNodeId(4),
        role: AccessibilityRole::Link,
        label: "Learn more".into(),
        description: String::new(),
        state: AccessibilityState::empty(),
        bounds: [0.0, 96.0, 512.0, 24.0],
        children: vec![],
        parent: Some(root_id),
        z_depth: 1.0,
    });

    if let Some(root) = ir.get_node(root_id) {
        let mut children = root.children.clone();
        children.push(heading_id);
        children.push(button_id);
        if let Some(root_mut) = ir.get_node_mut(root_id) {
            root_mut.children = children;
        }
    }

    println!("=== Accessibility Tree ===");
    for node in ir.flatten_tree() {
        let indent = "  ".repeat(count_depth(&ir, node.id));
        println!("{}{:?} (id={:?})", indent, node.role, node.id);
    }
    println!();

    println!("=== Focus Traversal ===");
    let mut focus = FocusTraversal::new();
    focus.compute_focus_order(&ir);

    let directions = [
        FocusDirection::First,
        FocusDirection::Forward,
        FocusDirection::Forward,
        FocusDirection::Forward,
        FocusDirection::Last,
        FocusDirection::Backward,
    ];

    for dir in &directions {
        match focus.move_focus(*dir, &ir) {
            Some(result) => println!("  {:?} -> {:?} ({:?})", dir, result.label, result.role),
            None => println!("  {:?} -> no focus", dir),
        }
    }
}

fn count_depth(ir: &SemanticIR, node_id: tpt_chora_a11y::SemanticNodeId) -> usize {
    let mut depth = 0;
    let mut current = node_id;
    while let Some(node) = ir.get_node(current) {
        if let Some(parent) = node.parent {
            depth += 1;
            current = parent;
        } else {
            break;
        }
    }
    depth
}
