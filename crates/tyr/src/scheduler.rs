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

    pub async fn run(mut self) {
        loop {
            for system in &mut self.systems {
                unsafe {
                    system.run(&mut self.data as *mut _).await;
                }
            }
        }
    }
}
