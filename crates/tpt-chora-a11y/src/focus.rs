use crate::semantic::{
    AccessibilityRole, AccessibilityState, SemanticIR, SemanticNode, SemanticNodeId,
};

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
            compute_subtree_order(ir, root, &mut self.focus_order);
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
                                    a_dist.total_cmp(&b_dist)
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
                                    a_dist.total_cmp(&b_dist)
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

impl Default for FocusTraversal {
    fn default() -> Self {
        Self::new()
    }
}

fn compute_subtree_order(ir: &SemanticIR, node: &SemanticNode, order: &mut Vec<SemanticNodeId>) {
    if !node.state.contains(AccessibilityState::HIDDEN) && node.role != AccessibilityRole::Separator
    {
        order.push(node.id);
    }
    for &child_id in &node.children {
        if let Some(child) = ir.get_node(child_id) {
            compute_subtree_order(ir, child, order);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: u64) -> SemanticNode {
        SemanticNode {
            id: SemanticNodeId(id),
            role: AccessibilityRole::Generic,
            label: format!("node_{id}"),
            description: String::new(),
            state: AccessibilityState::empty(),
            bounds: [0.0, 0.0, 100.0, 100.0],
            children: Vec::new(),
            parent: None,
            z_depth: 0.0,
        }
    }

    fn make_node_with_children(id: u64, children: Vec<SemanticNodeId>) -> SemanticNode {
        let mut node = make_node(id);
        node.children = children;
        node
    }

    fn make_node_with_state(id: u64, state: AccessibilityState) -> SemanticNode {
        let mut node = make_node(id);
        node.state = state;
        node
    }

    fn make_node_with_role(id: u64, role: AccessibilityRole) -> SemanticNode {
        let mut node = make_node(id);
        node.role = role;
        node
    }

    fn build_three_node_ir() -> SemanticIR {
        let mut ir = SemanticIR::new();
        let c1 = SemanticNodeId(2);
        let c2 = SemanticNodeId(3);
        ir.add_node(make_node_with_children(1, vec![c1, c2]));
        ir.add_node(make_node(2));
        ir.add_node(make_node(3));
        ir
    }

    #[test]
    fn new_creates_empty_traversal() {
        let ft = FocusTraversal::new();
        assert!(ft.current_focus().is_none());
        assert!(ft.focus_order.is_empty());
    }

    #[test]
    fn compute_focus_order_three_nodes() {
        let ir = build_three_node_ir();
        let mut ft = FocusTraversal::new();
        ft.compute_focus_order(&ir);
        assert_eq!(ft.focus_order.len(), 3);
        assert_eq!(ft.focus_order[0], SemanticNodeId(1));
        assert_eq!(ft.focus_order[1], SemanticNodeId(2));
        assert_eq!(ft.focus_order[2], SemanticNodeId(3));
    }

    #[test]
    fn move_first() {
        let ir = build_three_node_ir();
        let mut ft = FocusTraversal::new();
        ft.compute_focus_order(&ir);
        let result = ft.move_focus(FocusDirection::First, &ir);
        assert!(result.is_some());
        assert_eq!(ft.current_focus(), Some(SemanticNodeId(1)));
    }

    #[test]
    fn move_last() {
        let ir = build_three_node_ir();
        let mut ft = FocusTraversal::new();
        ft.compute_focus_order(&ir);
        let result = ft.move_focus(FocusDirection::Last, &ir);
        assert!(result.is_some());
        assert_eq!(ft.current_focus(), Some(SemanticNodeId(3)));
    }

    #[test]
    fn move_forward_advances() {
        let ir = build_three_node_ir();
        let mut ft = FocusTraversal::new();
        ft.compute_focus_order(&ir);
        ft.move_focus(FocusDirection::First, &ir);
        ft.move_focus(FocusDirection::Forward, &ir);
        assert_eq!(ft.current_focus(), Some(SemanticNodeId(2)));
    }

    #[test]
    fn move_forward_wraps_around() {
        let ir = build_three_node_ir();
        let mut ft = FocusTraversal::new();
        ft.compute_focus_order(&ir);
        ft.move_focus(FocusDirection::Last, &ir);
        assert_eq!(ft.current_focus(), Some(SemanticNodeId(3)));
        ft.move_focus(FocusDirection::Forward, &ir);
        assert_eq!(ft.current_focus(), Some(SemanticNodeId(1)));
    }

    #[test]
    fn move_backward_goes_to_previous() {
        let ir = build_three_node_ir();
        let mut ft = FocusTraversal::new();
        ft.compute_focus_order(&ir);
        ft.move_focus(FocusDirection::First, &ir);
        ft.move_focus(FocusDirection::Forward, &ir);
        ft.move_focus(FocusDirection::Forward, &ir);
        assert_eq!(ft.current_focus(), Some(SemanticNodeId(3)));
        ft.move_focus(FocusDirection::Backward, &ir);
        assert_eq!(ft.current_focus(), Some(SemanticNodeId(2)));
    }

    #[test]
    fn move_backward_wraps_around() {
        let ir = build_three_node_ir();
        let mut ft = FocusTraversal::new();
        ft.compute_focus_order(&ir);
        ft.move_focus(FocusDirection::First, &ir);
        assert_eq!(ft.current_focus(), Some(SemanticNodeId(1)));
        ft.move_focus(FocusDirection::Backward, &ir);
        assert_eq!(ft.current_focus(), Some(SemanticNodeId(3)));
    }

    #[test]
    fn clear_focus() {
        let ir = build_three_node_ir();
        let mut ft = FocusTraversal::new();
        ft.compute_focus_order(&ir);
        ft.move_focus(FocusDirection::First, &ir);
        assert!(ft.current_focus().is_some());
        ft.clear_focus();
        assert!(ft.current_focus().is_none());
    }

    #[test]
    fn set_focus_directly() {
        let mut ft = FocusTraversal::new();
        ft.set_focus(SemanticNodeId(42));
        assert_eq!(ft.current_focus(), Some(SemanticNodeId(42)));
    }

    #[test]
    fn empty_focus_order_move_returns_none() {
        let ir = SemanticIR::new();
        let mut ft = FocusTraversal::new();
        let result = ft.move_focus(FocusDirection::Forward, &ir);
        assert!(result.is_none());
        assert!(ft.current_focus().is_none());
    }

    #[test]
    fn hidden_nodes_excluded_from_focus_order() {
        let mut ir = SemanticIR::new();
        let c1 = SemanticNodeId(2);
        let c2 = SemanticNodeId(3);
        ir.add_node(make_node_with_children(1, vec![c1, c2]));
        ir.add_node(make_node(2));
        ir.add_node(make_node_with_state(3, AccessibilityState::HIDDEN));

        let mut ft = FocusTraversal::new();
        ft.compute_focus_order(&ir);
        assert_eq!(ft.focus_order.len(), 2);
        assert!(!ft.focus_order.contains(&SemanticNodeId(3)));
    }

    #[test]
    fn separator_nodes_excluded_from_focus_order() {
        let mut ir = SemanticIR::new();
        let c1 = SemanticNodeId(2);
        let c2 = SemanticNodeId(3);
        ir.add_node(make_node_with_children(1, vec![c1, c2]));
        ir.add_node(make_node(2));
        ir.add_node(make_node_with_role(3, AccessibilityRole::Separator));

        let mut ft = FocusTraversal::new();
        ft.compute_focus_order(&ir);
        assert_eq!(ft.focus_order.len(), 2);
        assert!(!ft.focus_order.contains(&SemanticNodeId(3)));
    }
}
