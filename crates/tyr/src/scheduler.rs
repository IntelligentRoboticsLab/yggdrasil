use futures::task::{ArcWake, Context, waker_ref, Poll};
use std::sync::{Arc, mpsc::{SyncSender, sync_channel}};
use crate::data::Data;
use crate::system::System;

pub struct Scheduler<D: Data> {
    data: D,
    systems: Vec<System<D>>,
}

impl<D: Data> Scheduler<D> {
    pub fn new(data: D) -> Self {
        Self {
            data,
            systems: Vec::new(),
        }
    }

    pub fn add(&mut self, system: System<D>) {
        self.systems.push(system);
    }

    pub fn run(mut self) {
        loop {
            for system in &mut self.systems {
                let (tx, rx) = sync_channel(1);

                let mut future = unsafe { system.run(&mut self.data as *mut _) };

                let task = Arc::new(Task(tx));
                let waker = waker_ref(&task);

                loop {
                    match future.as_mut().poll(&mut Context::from_waker(&*waker)) {
                        Poll::Ready(()) => break,
                        Poll::Pending => { rx.recv().unwrap() },
                    }
                }
            }
        }
    }
}

struct Task(SyncSender<()>);

impl ArcWake for Task {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        (arc_self.0).send(()).unwrap();
    }
}
