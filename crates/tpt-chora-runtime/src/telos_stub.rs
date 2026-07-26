pub struct TelosState {
    transitions: Vec<StateTransition>,
}

pub struct TelosEvent {
    pub target_id: u64,
    pub event_type: EventType,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    Click,
    DoubleClick,
    LongPress,
    KeyDown,
    KeyUp,
    Focus,
    Blur,
    TouchBegin,
    TouchEnd,
    ValueChange,
}

#[derive(Debug, Clone)]
pub struct StateTransition {
    pub page_id: u64,
    pub field_updates: Vec<(usize, Vec<u8>)>,
}

#[derive(Debug, Clone)]
pub struct StateMutation {
    pub page_id: u64,
    pub field_updates: Vec<(usize, Vec<u8>)>,
}

impl TelosState {
    pub fn new() -> Self {
        Self {
            transitions: Vec::new(),
        }
    }

    pub fn process_event(
        &mut self,
        event: &TelosEvent,
        proofs: &[EidosProof],
    ) -> Option<StateMutation> {
        let proof_valid = proofs
            .iter()
            .any(|p| p.target_id == event.target_id && p.valid);
        if !proof_valid {
            return None;
        }

        match event.event_type {
            EventType::Click => Some(StateMutation {
                page_id: event.target_id,
                field_updates: vec![(0, vec![1u8])],
            }),
            EventType::Focus => Some(StateMutation {
                page_id: event.target_id,
                field_updates: vec![(1, vec![1u8])],
            }),
            EventType::Blur => Some(StateMutation {
                page_id: event.target_id,
                field_updates: vec![(1, vec![0u8])],
            }),
            _ => None,
        }
    }
}

pub struct EidosProof {
    pub target_id: u64,
    pub proof_type: ProofType,
    pub valid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProofType {
    LayoutFits,
    TextFitsBounds,
    NoOverflow,
    AccessibilityCompliant,
}
