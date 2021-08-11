use atomic_refcell::AtomicRefCell;
use basedrop::{Handle, Shared};

use super::{node::AudioGraphNode, MonoAudioBlockBuffer, StereoAudioBlockBuffer};

#[derive(Clone)]
pub struct GraphResourcePool {
    // Using AtomicRefCell because these resources are only ever borrowed by
    // the rt thread. We keep these pointers in a non-rt thread so we can
    // cheaply clone and reconstruct a new schedule to send to the rt thread whenever the
    // graph is recompiled (only need to copy pointers instead of whole Vecs).
    pub(super) nodes: Vec<Shared<AtomicRefCell<Box<dyn AudioGraphNode>>>>,
    pub(super) mono_audio_buffers: Vec<Shared<AtomicRefCell<MonoAudioBlockBuffer>>>,
    pub(super) stereo_audio_buffers: Vec<Shared<AtomicRefCell<StereoAudioBlockBuffer>>>,

    coll_handle: Handle,
}

/*
impl Clone for GraphResourcePool {
    fn clone(&self) -> Self {
        let mut nodes =
            Vec::<Shared<AtomicRefCell<Box<dyn AudioGraphNode>>>>::with_capacity(self.nodes.len());
        let mut mono_audio_buffers =
            Vec::<Shared<AtomicRefCell<MonoAudioBlockBuffer>>>::with_capacity(
                self.mono_audio_buffers.len(),
            );
        let mut stereo_audio_buffers =
            Vec::<Shared<AtomicRefCell<StereoAudioBlockBuffer>>>::with_capacity(
                self.stereo_audio_buffers.len(),
            );

        for node in self.nodes.iter() {
            nodes.push(Shared::clone(node));
        }
        for audio_buffer in self.mono_audio_buffers.iter() {
            mono_audio_buffers.push(Shared::clone(audio_buffer));
        }
        for audio_buffer in self.stereo_audio_buffers.iter() {
            stereo_audio_buffers.push(Shared::clone(audio_buffer));
        }

        Self {
            nodes,
            mono_audio_buffers,
            stereo_audio_buffers,
            coll_handle: self.coll_handle.clone(),
        }
    }
}
*/

impl GraphResourcePool {
    /// Create a new resource pool. Only to be used by the non-rt thread.
    pub(super) fn new(coll_handle: Handle) -> Self {
        Self {
            nodes: Vec::new(),
            mono_audio_buffers: Vec::new(),
            stereo_audio_buffers: Vec::new(),
            coll_handle: coll_handle,
        }
    }

    /// Add a new audio graph nodes to the pool. Only to be used by the non-rt thread.
    pub(super) fn add_node(&mut self, new_node: Box<dyn AudioGraphNode>) {
        self.nodes
            .push(Shared::new(&self.coll_handle, AtomicRefCell::new(new_node)));
    }

    /// Remove nodes from the pool. Only to be used by the non-rt thread.
    pub(super) fn remove_node(&mut self, node_index: usize) -> Result<(), ()> {
        if node_index < self.nodes.len() {
            self.nodes.remove(node_index);
            Ok(())
        } else {
            Err(())
        }
    }

    /// Replaces a node in the pool. Only to be used by the non-rt thread.
    pub(super) fn replace_node(
        &mut self,
        node_index: usize,
        new_node: Box<dyn AudioGraphNode>,
    ) -> Result<(), ()> {
        if node_index < self.nodes.len() {
            self.nodes[node_index] = Shared::new(&self.coll_handle, AtomicRefCell::new(new_node));
            Ok(())
        } else {
            Err(())
        }
    }

    /// Add new mono audio port buffer to the pool. Only to be used by the non-rt thread.
    pub(super) fn add_mono_audio_port_buffers(&mut self, n_new_port_buffers: usize) {
        for _ in 0..n_new_port_buffers {
            self.mono_audio_buffers.push(Shared::new(
                &self.coll_handle,
                AtomicRefCell::new(MonoAudioBlockBuffer::new()),
            ));
        }
    }

    /// Add new stereo audio port buffer to the pool. Only to be used by the non-rt thread.
    pub(super) fn add_stereo_audio_port_buffers(&mut self, n_new_port_buffers: usize) {
        for _ in 0..n_new_port_buffers {
            self.stereo_audio_buffers.push(Shared::new(
                &self.coll_handle,
                AtomicRefCell::new(StereoAudioBlockBuffer::new()),
            ));
        }
    }

    /// Remove audio buffers from the pool. Only to be used by the non-rt thread.
    ///
    /// * `range` - The range of indexes (`start <= x < end`) of the buffers to remove.
    ///
    /// This will return an Error instead if the given range is empty or if it contains an index that is
    /// out of range.
    pub(super) fn remove_mono_audio_buffers(&mut self, n_to_remove: usize) -> Result<(), ()> {
        if n_to_remove <= self.mono_audio_buffers.len() {
            for _ in 0..n_to_remove {
                let _ = self.mono_audio_buffers.pop();
            }
            Ok(())
        } else {
            Err(())
        }
    }

    /// Remove audio buffers from the pool. Only to be used by the non-rt thread.
    ///
    /// * `range` - The range of indexes (`start <= x < end`) of the buffers to remove.
    ///
    /// This will return an Error instead if the given range is empty or if it contains an index that is
    /// out of range.
    pub(super) fn remove_stereo_audio_buffers(&mut self, n_to_remove: usize) -> Result<(), ()> {
        if n_to_remove <= self.stereo_audio_buffers.len() {
            for _ in 0..n_to_remove {
                let _ = self.stereo_audio_buffers.pop();
            }
            Ok(())
        } else {
            Err(())
        }
    }

    /// Only to be used by the rt thread.
    pub fn clear_all_buffers(&mut self) {
        for b in self.mono_audio_buffers.iter() {
            // Should not panic because the rt thread is the only thread that ever borrows resources.
            let b = &mut *AtomicRefCell::borrow_mut(b);

            b.clear();
        }
        for b in self.stereo_audio_buffers.iter() {
            // Should not panic because the rt thread is the only thread that ever borrows resources.
            let b = &mut *AtomicRefCell::borrow_mut(b);

            b.clear();
        }
    }
}