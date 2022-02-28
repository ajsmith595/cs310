use cs310_shared::store::Store;

use crate::gst_process::ProcessPool;

pub struct State {
    pub store: Store,
    pub gstreamer_processes: ProcessPool,
}
