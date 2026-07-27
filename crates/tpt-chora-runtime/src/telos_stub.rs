pub trait TelosBackend {
    fn process_event(
        &mut self,
        event: &TelosEvent,
        proofs: &[EidosProof],
    ) -> Option<StateMutation>;
    fn transitions(&self) -> &[StateTransition];
    fn transition_count(&self) -> usize;
}

pub struct TelosState {
    transitions: Vec<StateTransition>,
}

pub struct TelosEvent {
    pub target_id: u64,
    pub event_type: EventType,
    pub timestamp: u64,
    pub payload: Vec<u8>,
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

        let mutation = match event.event_type {
            EventType::Click => Some(StateMutation {
                page_id: event.target_id,
                field_updates: vec![(0, vec![1u8])],
            }),
            EventType::DoubleClick => Some(StateMutation {
                page_id: event.target_id,
                field_updates: vec![(0, vec![2u8])],
            }),
            EventType::LongPress => Some(StateMutation {
                page_id: event.target_id,
                field_updates: vec![(0, vec![3u8])],
            }),
            EventType::Focus => Some(StateMutation {
                page_id: event.target_id,
                field_updates: vec![(1, vec![1u8])],
            }),
            EventType::Blur => Some(StateMutation {
                page_id: event.target_id,
                field_updates: vec![(1, vec![0u8])],
            }),
            EventType::KeyDown => {
                let key_byte = event.payload.first().copied().unwrap_or(0);
                Some(StateMutation {
                    page_id: event.target_id,
                    field_updates: vec![(2, vec![key_byte])],
                })
            }
            EventType::KeyUp => Some(StateMutation {
                page_id: event.target_id,
                field_updates: vec![(2, vec![0u8])],
            }),
            EventType::TouchBegin => {
                let touch_id = event.payload.first().copied().unwrap_or(0);
                Some(StateMutation {
                    page_id: event.target_id,
                    field_updates: vec![(3, vec![touch_id])],
                })
            }
            EventType::TouchEnd => Some(StateMutation {
                page_id: event.target_id,
                field_updates: vec![(3, vec![0u8])],
            }),
            EventType::ValueChange => {
                let value_bytes = event.payload.clone();
                Some(StateMutation {
                    page_id: event.target_id,
                    field_updates: vec![(4, value_bytes)],
                })
            }
        };

        if let Some(ref m) = mutation {
            self.transitions.push(StateTransition {
                page_id: m.page_id,
                field_updates: m.field_updates.clone(),
            });
        }

        mutation
    }

    pub fn transitions(&self) -> &[StateTransition] {
        &self.transitions
    }

    pub fn transition_count(&self) -> usize {
        self.transitions.len()
    }
}

impl Default for TelosState {
    fn default() -> Self {
        Self::new()
    }
}

impl TelosBackend for TelosState {
    fn process_event(
        &mut self,
        event: &TelosEvent,
        proofs: &[EidosProof],
    ) -> Option<StateMutation> {
        TelosState::process_event(self, event, proofs)
    }

    fn transitions(&self) -> &[StateTransition] {
        TelosState::transitions(self)
    }

    fn transition_count(&self) -> usize {
        TelosState::transition_count(self)
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
