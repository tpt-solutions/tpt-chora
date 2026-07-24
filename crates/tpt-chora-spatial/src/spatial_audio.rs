use std::collections::HashMap;

use glam::Vec3;

pub struct SpatialAudioEngine {
    listener_pos: Vec3,
    listener_forward: Vec3,
    listener_up: Vec3,
    sources: HashMap<u64, AudioSource>,
    next_id: u64,
}

pub struct AudioSource {
    pub id: u64,
    pub position: Vec3,
    pub gain: f32,
    pub rolloff: f32,
    pub inner_radius: f32,
    pub playing: bool,
    pub sound_data: Vec<u8>,
}

impl SpatialAudioEngine {
    pub fn new() -> Self {
        Self {
            listener_pos: Vec3::ZERO,
            listener_forward: Vec3::Z,
            listener_up: Vec3::Y,
            sources: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn update_listener(
        &mut self,
        position: Vec3,
        forward: Vec3,
        up: Vec3,
    ) {
        self.listener_pos = position;
        self.listener_forward = forward.normalize();
        self.listener_up = up.normalize();
    }

    pub fn add_source(&mut self, position: Vec3, gain: f32, sound_data: Vec<u8>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.sources.insert(
            id,
            AudioSource {
                id,
                position,
                gain,
                rolloff: 1.0,
                inner_radius: 1.0,
                playing: true,
                sound_data,
            },
        );
        id
    }

    pub fn remove_source(&mut self, id: u64) {
        self.sources.remove(&id);
    }

    pub fn update_source_position(&mut self, id: u64, position: Vec3) {
        if let Some(source) = self.sources.get_mut(&id) {
            source.position = position;
        }
    }

    pub fn compute_hrtf(&self, source_id: u64) -> Option<HrtfParams> {
        let source = self.sources.get(&source_id)?;
        if !source.playing {
            return None;
        }

        let to_source = source.position - self.listener_pos;
        let distance = to_source.length();

        let right = self.listener_forward.cross(self.listener_up).normalize();

        let azimuth = to_source.normalize().dot(right).asin();
        let elevation = to_source
            .normalize()
            .dot(self.listener_up.cross(right).normalize())
            .asin();

        let rolloff_gain = source.inner_radius
            / (source.inner_radius + source.rolloff * (distance - source.inner_radius));

        let gain = source.gain * rolloff_gain;

        Some(HrtfParams {
            azimuth,
            elevation,
            distance,
            gain,
        })
    }

    pub fn sources(&self) -> impl Iterator<Item = &AudioSource> {
        self.sources.values()
    }
}

#[derive(Debug, Clone)]
pub struct HrtfParams {
    pub azimuth: f32,
    pub elevation: f32,
    pub distance: f32,
    pub gain: f32,
}
