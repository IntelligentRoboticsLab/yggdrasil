#![allow(dead_code)]

use std::time::{self, Duration};

use bevy::{core::FrameCount, prelude::*};
use tasks::{combinators::Combinators, CommandsExt, TaskPool};

#[derive(Resource, Default)]
pub struct Foo(FrameCount);

fn normal(mut commands: Commands, frame: Res<FrameCount>) {
    commands
        .prepare_task(TaskPool::AsyncCompute)
        .to_resource()
        .spawn({
            let frame = frame.clone();
            async move {
                async_std::task::sleep(Duration::from_secs(3)).await;

                // This returns an Option<Foo>
                Some(Foo(frame))
            }
        });
}

fn timeout_combinator(mut commands: Commands, frame: Res<FrameCount>) {
    commands
        .prepare_task(TaskPool::AsyncCompute)
        .to_resource()
        .spawn({
            let frame = frame.clone();
            async move {
                async_std::task::sleep(Duration::from_secs(3)).await;

                // This returns a Foo!
                Foo(frame)
            }
            // The with_timeout combinator will transform the output to an Option<Foo>.
            // - `None` if the future does not complete in time
            // - `Some(Foo)` if the future completes in time
            //
            // If the future does not complete in time, the task will be cancelled early
            // and nothing gets added to the resource/component storage.
            .with_timeout(time::Duration::from_secs(2))
        });
}

fn main() {}
