use crate::semantic::{AccessibilityRole, AccessibilityState, SemanticIR, SemanticNode, SemanticNodeId};

pub struct FocusTraversal {
    current_focus: Option<SemanticNodeId>,
    focus_order: Vec<SemanticNodeId>,
}

#[derive(Debug, Clone, Copy)]
pub enum FocusDirection {
    Forward,
    Backward,
    Up,
    Down,
    First,
    Last,
}

#[derive(Debug, Clone)]
pub struct FocusResult {
    pub node_id: SemanticNodeId,
    pub bounds: [f32; 4],
    pub role: AccessibilityRole,
    pub label: String,
}

impl FocusTraversal {
    pub fn new() -> Self {
        Self {
            current_focus: None,
            focus_order: Vec::new(),
        }
    }

    pub fn compute_focus_order(&mut self, ir: &SemanticIR) {
        self.focus_order.clear();
        if let Some(root) = ir.root() {
            let children: Vec<SemanticNodeId> = root.children.clone();
            compute_subtree_order(ir, root, &mut self.focus_order);
            for child_id in children {
                if let Some(child) = ir.get_node(child_id) {
                    compute_subtree_order(ir, child, &mut self.focus_order);
                }
            }
        }
    }
}

fn compute_subtree_order(
    ir: &SemanticIR,
    node: &SemanticNode,
    order: &mut Vec<SemanticNodeId>,
) {
    if !node.state.contains(AccessibilityState::HIDDEN)
        && node.role != AccessibilityRole::Separator
    {
        order.push(node.id);
    }
    for &child_id in &node.children {
        if let Some(child) = ir.get_node(child_id) {
            compute_subtree_order(ir, child, order);
        }
    }
}
        if !node.state.contains(AccessibilityState::HIDDEN)
            && node.role != AccessibilityRole::Separator
        {
            order.push(node.id);
        }
        for &child_id in &node.children {
            if let Some(child) = ir.get_node(child_id) {
                self.compute_subtree_order(ir, child, order);
            }
        }
    }

    pub fn move_focus(
        &mut self,
        direction: FocusDirection,
        ir: &SemanticIR,
    ) -> Option<FocusResult> {
        match direction {
            FocusDirection::First => {
                self.current_focus = self.focus_order.first().copied();
            }
            FocusDirection::Last => {
                self.current_focus = self.focus_order.last().copied();
            }
            FocusDirection::Forward | FocusDirection::Backward => {
                let current_idx = self
                    .current_focus
                    .and_then(|id| self.focus_order.iter().position(|&fid| fid == id));

                match (direction, current_idx) {
                    (FocusDirection::Forward, Some(idx)) => {
                        let next = (idx + 1) % self.focus_order.len();
                        self.current_focus = Some(self.focus_order[next]);
                    }
                    (FocusDirection::Forward, None) => {
                        self.current_focus = self.focus_order.first().copied();
                    }
                    (FocusDirection::Backward, Some(idx)) => {
                        let prev = if idx == 0 {
                            self.focus_order.len() - 1
                        } else {
                            idx - 1
                        };
                        self.current_focus = Some(self.focus_order[prev]);
                    }
                    (FocusDirection::Backward, None) => {
                        self.current_focus = self.focus_order.last().copied();
                    }
                    _ => unreachable!(),
                }
            }
            FocusDirection::Up | FocusDirection::Down => {
                let current_idx = self
                    .current_focus
                    .and_then(|id| self.focus_order.iter().position(|&fid| fid == id));

                if let Some(idx) = current_idx {
                    let current_id = self.focus_order[idx];
                    if let Some(current_node) = ir.get_node(current_id) {
                        let current_bounds = current_node.bounds;
                        let center_y = (current_bounds[1] + current_bounds[3]) * 0.5;

                        let candidates: Vec<&SemanticNode> = self
                            .focus_order
                            .iter()
                            .filter_map(|&id| ir.get_node(id))
                            .filter(|n| n.id != current_id)
                            .collect();

                        let best = match direction {
                            FocusDirection::Up => candidates
                                .iter()
                                .filter(|n| (n.bounds[1] + n.bounds[3]) * 0.5 < center_y)
                                .min_by(|a, b| {
                                    let a_dist = ((a.bounds[0] + a.bounds[2]) * 0.5
                                        - (current_bounds[0] + current_bounds[2]) * 0.5)
                                        .abs();
                                    let b_dist = ((b.bounds[0] + b.bounds[2]) * 0.5
                                        - (current_bounds[0] + current_bounds[2]) * 0.5)
                                        .abs();
                                    a_dist.partial_cmp(&b_dist).unwrap()
                                }),
                            FocusDirection::Down => candidates
                                .iter()
                                .filter(|n| (n.bounds[1] + n.bounds[3]) * 0.5 > center_y)
                                .min_by(|a, b| {
                                    let a_dist = ((a.bounds[0] + a.bounds[2]) * 0.5
                                        - (current_bounds[0] + current_bounds[2]) * 0.5)
                                        .abs();
                                    let b_dist = ((b.bounds[0] + b.bounds[2]) * 0.5
                                        - (current_bounds[0] + current_bounds[2]) * 0.5)
                                        .abs();
                                    a_dist.partial_cmp(&b_dist).unwrap()
                                }),
                            _ => unreachable!(),
                        };

                        if let Some(best) = best {
                            self.current_focus = Some(best.id);
                        }
                    }
                }
            }
        }

        self.current_focus.and_then(|id| {
            ir.get_node(id).map(|node| FocusResult {
                node_id: node.id,
                bounds: node.bounds,
                role: node.role,
                label: node.label.clone(),
            })
        })
    }

    pub fn current_focus(&self) -> Option<SemanticNodeId> {
        self.current_focus
    }

    pub fn set_focus(&mut self, id: SemanticNodeId) {
        self.current_focus = Some(id);
    }

    pub fn clear_focus(&mut self) {
        self.current_focus = None;
    }
}
