use basedrop::{Collector, Handle, Shared, SharedCell};
use rusty_daw_audio_graph::{CompiledGraph, GraphInterface, GraphStateRef};
use rusty_daw_core::SampleRate;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, LockResult, Mutex,
};
use std::time::Duration;

use crate::backend::resource_loader::ResourceLoader;
use crate::backend::save_state::BackendSaveState;
use crate::backend::timeline::{
    AudioClipResourceCache, TimelineTransport, TimelineTransportHandle, TimelineTransportSaveState,
};

use super::MAX_BLOCKSIZE;

static COLLECT_INTERVAL: Duration = Duration::from_secs(3);

pub struct ResourceCache {
    pub(crate) resource_loader: Arc<Mutex<ResourceLoader>>,
    pub(crate) audio_clip_resource_cache: Arc<Mutex<AudioClipResourceCache>>,
}

impl Clone for ResourceCache {
    fn clone(&self) -> Self {
        Self {
            resource_loader: Arc::clone(&self.resource_loader),
            audio_clip_resource_cache: Arc::clone(&self.audio_clip_resource_cache),
        }
    }
}

pub struct GlobalNodeData {
    pub transport: TimelineTransport,
}

pub struct BackendHandle {
    graph_interface: GraphInterface<GlobalNodeData, MAX_BLOCKSIZE>,

    resource_cache: ResourceCache,

    /*
    timeline_track_handles: Vec<TimelineTrackHandle>,
    timeline_track_node_refs: Vec<NodeRef>,
    */
    timeline_transport: TimelineTransportHandle,

    sample_rate: SampleRate,

    coll_handle: Handle,
    running: Arc<AtomicBool>,
}

impl BackendHandle {
    pub fn new(
        sample_rate: SampleRate,
    ) -> (Self, Shared<SharedCell<CompiledGraph<GlobalNodeData, MAX_BLOCKSIZE>>>) {
        let collector = Collector::new();
        let coll_handle = collector.handle();

        let resource_loader =
            Arc::new(Mutex::new(ResourceLoader::new(collector.handle(), sample_rate)));
        let resource_loader_clone = Arc::clone(&resource_loader);

        let audio_clip_resource_cache =
            Arc::new(Mutex::new(AudioClipResourceCache::new(collector.handle(), sample_rate)));
        let audio_clip_r_c_clone = Arc::clone(&audio_clip_resource_cache);

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        std::thread::spawn(|| {
            run_collector(collector, resource_loader_clone, audio_clip_r_c_clone, running_clone)
        });

        let (timeline_transport, timeline_transport_handle) =
            TimelineTransport::new(coll_handle.clone(), sample_rate);

        let (graph_interface, rt_graph_interface) = GraphInterface::new(
            sample_rate,
            coll_handle.clone(),
            GlobalNodeData { transport: timeline_transport },
        );

        (
            Self {
                graph_interface,

                resource_cache: ResourceCache { resource_loader, audio_clip_resource_cache },

                /*
                timeline_track_handles: Vec::<TimelineTrackHandle>::new(),
                timeline_track_node_refs: Vec::<NodeRef>::new(),
                */
                timeline_transport: timeline_transport_handle,

                sample_rate,
                coll_handle,

                running,
            },
            rt_graph_interface,
        )
    }

    pub fn from_save_state(
        sample_rate: SampleRate,
        save_state: &mut BackendSaveState,
    ) -> (Self, Shared<SharedCell<CompiledGraph<GlobalNodeData, MAX_BLOCKSIZE>>>) {
        save_state.tempo_map.sample_rate = sample_rate;

        let collector = Collector::new();
        let coll_handle = collector.handle();

        let resource_loader =
            Arc::new(Mutex::new(ResourceLoader::new(collector.handle(), sample_rate)));
        let resource_loader_clone = Arc::clone(&resource_loader);

        let audio_clip_resource_cache =
            Arc::new(Mutex::new(AudioClipResourceCache::new(collector.handle(), sample_rate)));
        let audio_clip_r_c_clone = Arc::clone(&audio_clip_resource_cache);

        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);
        std::thread::spawn(|| {
            run_collector(collector, resource_loader_clone, audio_clip_r_c_clone, running_clone)
        });

        let (timeline_transport, mut timeline_transport_handle) =
            TimelineTransport::new(coll_handle.clone(), sample_rate);

        timeline_transport_handle._update_tempo_map(save_state.tempo_map.clone());
        timeline_transport_handle
            .seek_to(save_state.timeline_transport.seek_to, &mut save_state.timeline_transport);
        if let Err(_) = timeline_transport_handle.set_loop_state(
            save_state.timeline_transport.loop_state.clone(),
            &mut save_state.timeline_transport,
        ) {
            log::error!(
                "Failed to set loop state on timeline transport: {:?}",
                save_state.timeline_transport.loop_state
            );
        }

        let (graph_interface, rt_graph_interface) = GraphInterface::new(
            sample_rate,
            coll_handle.clone(),
            GlobalNodeData { transport: timeline_transport },
        );

        (
            Self {
                graph_interface,

                resource_cache: ResourceCache { resource_loader, audio_clip_resource_cache },

                /*
                timeline_track_handles: Vec::<TimelineTrackHandle>::new(),
                timeline_track_node_refs: Vec::<NodeRef>::new(),
                */
                timeline_transport: timeline_transport_handle,

                sample_rate,
                coll_handle,

                running,
            },
            rt_graph_interface,
        )
    }

    pub fn set_bpm(&mut self, bpm: f64, save_state: &mut BackendSaveState) {
        assert!(bpm > 0.0 && bpm <= 100_000.0);

        save_state.tempo_map.set_bpm(bpm);

        self.timeline_transport._update_tempo_map(save_state.tempo_map.clone());
    }

    // We are using a closure for all modifications to the graph instead of using individual methods to act on
    // the graph. This is so the graph only gets compiled once after the user is done, instead of being recompiled
    // after every method.
    // TODO: errors and reverting to previous working state
    pub fn modify_graph<
        F: FnOnce(GraphStateRef<'_, GlobalNodeData, MAX_BLOCKSIZE>, &ResourceCache),
    >(
        &mut self,
        f: F,
    ) -> Result<(), ()> {
        let resource_cache = self.resource_cache.clone();
        self.graph_interface.modify_graph(|g| f(g, &resource_cache))
    }

    pub fn timeline_transport<'a>(
        &self,
        save_state: &'a BackendSaveState,
    ) -> (&TimelineTransportHandle, &'a TimelineTransportSaveState) {
        (&self.timeline_transport, &save_state.timeline_transport)
    }

    pub fn timeline_transport_mut<'a>(
        &mut self,
        save_state: &'a mut BackendSaveState,
    ) -> (&mut TimelineTransportHandle, &'a mut TimelineTransportSaveState) {
        (&mut self.timeline_transport, &mut save_state.timeline_transport)
    }

    pub fn resource_cache(&self) -> &ResourceCache {
        &self.resource_cache
    }
}

impl Drop for BackendHandle {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

fn run_collector(
    mut collector: Collector,
    resource_loader: Arc<Mutex<ResourceLoader>>,
    audio_clip_resource_cache: Arc<Mutex<AudioClipResourceCache>>,
    running: Arc<AtomicBool>,
) {
    while running.load(Ordering::Relaxed) {
        std::thread::sleep(COLLECT_INTERVAL);

        {
            match audio_clip_resource_cache.lock() {
                LockResult::Ok(mut cache) => {
                    cache.collect();
                }
                LockResult::Err(e) => {
                    log::error!("{}", e);
                    break;
                }
            }
        }

        {
            match resource_loader.lock() {
                LockResult::Ok(mut res_loader) => {
                    res_loader.collect();
                }
                LockResult::Err(e) => {
                    log::error!("{}", e);
                    break;
                }
            }
        }

        collector.collect();
    }
    log::info!("shutting down collector");
}
