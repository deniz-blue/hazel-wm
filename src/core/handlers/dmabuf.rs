use smithay::{
    backend::{allocator::dmabuf::Dmabuf, renderer::ImportDma},
    wayland::dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier},
};

use crate::core::Hazel;

const MAX_PENDING_DMABUF_IMPORTS: usize = 256;

impl DmabufHandler for Hazel {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.compositor.smithay.dmabuf_state
    }

    fn dmabuf_imported(&mut self, _: &DmabufGlobal, dmabuf: Dmabuf, notifier: ImportNotifier) {
        if self.compositor.pending_dmabuf_imports.len() >= MAX_PENDING_DMABUF_IMPORTS {
            notifier.failed();
            return;
        }
        self.compositor.pending_dmabuf_imports.push((dmabuf, notifier));
    }
}

impl Hazel {
    pub fn answer_dmabuf_imports(&mut self, renderer: &mut impl ImportDma) {
        let pending = std::mem::take(&mut self.compositor.pending_dmabuf_imports);
        for (dmabuf, notifier) in pending {
            match renderer.import_dmabuf(&dmabuf, None) {
                Ok(_) => {
                    let _ = notifier.successful::<Hazel>();
                }
                Err(_) => notifier.failed(),
            }
        }
    }
}

