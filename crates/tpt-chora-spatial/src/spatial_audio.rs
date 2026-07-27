use std::collections::HashMap;
use std::io::Cursor;

use glam::Vec3;

pub struct SpatialAudioEngine {
    listener_pos: Vec3,
    listener_forward: Vec3,
    listener_up: Vec3,
    sources: HashMap<u64, AudioSource>,
    next_id: u64,
    output: Option<SpatialAudioOutput>,
}

pub struct SpatialAudioOutput {
    _stream: rodio::OutputStream,
    sink: rodio::Sink,
}

pub struct AudioSource {
    pub id: u64,
    pub position: Vec3,
    pub gain: f32,
    pub rolloff: f32,
    pub inner_radius: f32,
    pub playing: bool,
    pub sound_data: Vec<u8>,
    pub hrtf_params: Option<HrtfParams>,
}

impl SpatialAudioOutput {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (stream, handle) = rodio::OutputStream::try_default()?;
        let sink = rodio::Sink::try_new(&handle).map_err(|e| format!("{}", e))?;
        Ok(Self {
            _stream: stream,
            sink,
        })
    }

    pub fn set_volume(&self, volume: f32) {
        self.sink.set_volume(volume.clamp(0.0, 1.0));
    }

    pub fn play_wav_data(&self, data: &[u8]) {
        let cursor = Cursor::new(data.to_vec());
        if let Ok(source) = rodio::Decoder::new_wav(cursor) {
            self.sink.append(source);
        }
    }

    pub fn play_source_any_format(&self, data: &[u8]) {
        let cursor = Cursor::new(data.to_vec());
        if let Ok(source) = rodio::Decoder::new(cursor) {
            self.sink.append(source);
        }
    }

    pub fn pause(&self) {
        self.sink.pause();
    }

    pub fn play(&self) {
        self.sink.play();
    }

    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }
}

impl SpatialAudioEngine {
    pub fn new() -> Self {
        let output = SpatialAudioOutput::new().ok();
        Self {
            listener_pos: Vec3::ZERO,
            listener_forward: Vec3::Z,
            listener_up: Vec3::Y,
            sources: HashMap::new(),
            next_id: 0,
            output,
        }
    }

    pub fn output(&self) -> Option<&SpatialAudioOutput> {
        self.output.as_ref()
    }

    pub fn update_listener(&mut self, position: Vec3, forward: Vec3, up: Vec3) {
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
                hrtf_params: None,
            },
        );
        id
    }

    pub fn add_source_and_play(&mut self, position: Vec3, gain: f32, sound_data: Vec<u8>) -> u64 {
        let id = self.add_source(position, gain, sound_data.clone());
        if let Some(params) = self.compute_hrtf(id) {
            if let Some(source) = self.sources.get_mut(&id) {
                source.hrtf_params = Some(params.clone());
            }
            if let Some(ref output) = self.output {
                output.set_volume(params.gain);
                output.play_source_any_format(&sound_data);
            }
        }
        id
    }

    pub fn remove_source(&mut self, id: u64) {
        self.sources.remove(&id);
    }

    pub fn update_source_position(&mut self, id: u64, position: Vec3) {
        if let Some(source) = self.sources.get_mut(&id) {
            source.position = position;
            let source_id = source.id;
            if let Some(params) = self.compute_hrtf(source_id) {
                if let Some(s) = self.sources.get_mut(&id) {
                    s.hrtf_params = Some(params.clone());
                }
                if let Some(ref output) = self.output {
                    output.set_volume(params.gain);
                }
            }
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
        let forward = self.listener_up.cross(right).normalize();

        let to_source_norm = if distance > 0.001 {
            to_source / distance
        } else {
            Vec3::ZERO
        };

        let azimuth = to_source_norm.dot(right).asin();
        let elevation = to_source_norm.dot(forward).asin();

        let rolloff_gain = source.inner_radius
            / (source.inner_radius + source.rolloff * (distance - source.inner_radius).max(0.0));

        let gain = source.gain * rolloff_gain;

        let itd_delay = if distance > 0.001 {
            let signed_dist = to_source.dot(right);
            (signed_dist / 343.0).clamp(-0.0012, 0.0012)
        } else {
            0.0
        };

        Some(HrtfParams {
            azimuth,
            elevation,
            distance,
            gain,
            interaural_time_diff_ms: itd_delay * 1000.0,
            near_field_gain: if distance < source.inner_radius {
                1.0 + (1.0 - distance / source.inner_radius) * 0.5
            } else {
                1.0
            },
        })
    }

    pub fn sources(&self) -> impl Iterator<Item = &AudioSource> {
        self.sources.values()
    }

    pub fn listener_position(&self) -> Vec3 {
        self.listener_pos
    }
}

impl Default for SpatialAudioEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct HrtfParams {
    pub azimuth: f32,
    pub elevation: f32,
    pub distance: f32,
    pub gain: f32,
    pub interaural_time_diff_ms: f32,
    pub near_field_gain: f32,
}
