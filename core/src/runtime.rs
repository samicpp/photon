use tokio::{runtime::Runtime, task::JoinHandle};
use std::sync::Arc;

use crate::atomic_arc::AtomicArc;

pub struct AsyncRT(pub AtomicArc<Runtime>);
pub static RT: AsyncRT = AsyncRT::new();

impl AsyncRT{
    pub const fn new() -> Self {
        Self(AtomicArc::empty())
    }

    pub fn spawn<F: Future + Send + 'static>(&self, future: F) -> Option<JoinHandle<F::Output>> 
    where F::Output: Send + 'static 
    {
        match self.0.load() {
            Some(rt) => Some(rt.spawn(future)),
            None => None,
        }
    }

    pub fn block_on<F: Future>(&self, future: F) -> Option<F::Output> {
        match self.0.load() {
            Some(rt) => Some(rt.block_on(future)),
            None => None,
        }
    }

    pub fn isset(&self) -> bool {
        self.0.load().is_some()
    }
    pub fn set(&self, rt: Runtime) {
        self.0.store(Some(Arc::new(rt)));
    }
    pub fn unset(&self) {
        self.0.store(None);
    }
}

#[derive(Debug, Clone)]
pub struct TokioSettings {
    pub multi_threaded: bool,
    pub worker_threads: Option<usize>,
    pub thread_name: Option<String>,
    pub event_interval: Option<u32>,
    pub max_io_events_per_tick: Option<usize>,
    pub global_queue_interval: Option<u32>,
    pub thread_keep_alive_ns: Option<u64>,
    pub thread_stack_size: Option<usize>,
    pub max_blocking_threads: Option<usize>,
}
impl TokioSettings {
    pub const fn default() -> Self {
        Self { 
            multi_threaded: true,
            worker_threads: None,
            thread_name: None,
            event_interval: None,
            max_io_events_per_tick: None,
            global_queue_interval: None,
            thread_keep_alive_ns: None,
            thread_stack_size: None,
            max_blocking_threads: None,
        }
    }
    pub const fn new_unset(multi_threaded: bool) -> Self {
        Self { 
            multi_threaded,
            worker_threads: None,
            thread_name: None,
            event_interval: None,
            max_io_events_per_tick: None,
            global_queue_interval: None,
            thread_keep_alive_ns: None,
            thread_stack_size: None,
            max_blocking_threads: None,
        }
    }
}
impl Default for TokioSettings {
    fn default() -> Self {
        Self::default()
    }
}

pub fn spawn_task<F: Future<Output = ()> + Send + 'static>(future: F) {
    RT.spawn(future);
}
