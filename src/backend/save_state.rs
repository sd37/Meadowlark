use rusty_daw_core::{SampleRate, Seconds};

use crate::backend::timeline::TempoMap;

use crate::backend::timeline::{
    audio_clip::DEFAULT_AUDIO_CLIP_DECLICK_TIME, TimelineTransportSaveState,
};

/// This struct should contain all information needed to create a "save file"
/// for the backend.
///
/// TODO: Project file format. This will need to be future-proof.
#[derive(Debug, Clone)]
pub struct BackendSaveState {
    pub(super) timeline_transport: TimelineTransportSaveState,
    pub(super) tempo_map: TempoMap,
    // TODO: Make this editable.
    pub(super) audio_clip_declick_time: Seconds,
}

impl Default for BackendSaveState {
    fn default() -> Self {
        Self {
            timeline_transport: TimelineTransportSaveState::default(),
            tempo_map: TempoMap::default(),
            audio_clip_declick_time: DEFAULT_AUDIO_CLIP_DECLICK_TIME,
        }
    }
}

impl BackendSaveState {
    pub fn new(timeline_transport: TimelineTransportSaveState, tempo_map: TempoMap) -> Self {
        Self {
            timeline_transport,
            tempo_map,
            audio_clip_declick_time: DEFAULT_AUDIO_CLIP_DECLICK_TIME,
        }
    }

    pub fn clone_with_sample_rate(&self, sample_rate: SampleRate) -> Self {
        let mut tempo_map = self.tempo_map.clone();
        tempo_map.sample_rate = sample_rate;
        Self {
            timeline_transport: self.timeline_transport.clone(),
            tempo_map,
            audio_clip_declick_time: self.audio_clip_declick_time,
        }
    }

    pub fn timeline_transport(&self) -> &TimelineTransportSaveState {
        &self.timeline_transport
    }

    pub fn tempo_map(&self) -> &TempoMap {
        &self.tempo_map
    }

    pub fn audio_clip_declick_time(&self) -> Seconds {
        self.audio_clip_declick_time
    }
}
