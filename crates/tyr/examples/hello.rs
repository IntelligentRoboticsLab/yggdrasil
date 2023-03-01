#![allow(clippy::disallowed_names)]
use tyr::data::*;
use tyr::scheduler::*;
use tyr::system::*;

#[derive(Data)]
struct Hello {
    foo: usize,
    bar: i32,
}

#[system(Hello)]
async fn increment_foo_and_bar(foo: &mut usize, bar: &mut i32) {
    *foo += 1;
    *bar += 1;
}

#[system(Hello)]
async fn print_foo_and_wait(foo: &usize) {
    println!("{}", foo);
    std::thread::sleep(std::time::Duration::from_millis(500));
}

#[tokio::main]
async fn main() {
    let mut sched = Scheduler::new(Hello { foo: 0, bar: 42 });

    sched.add(increment_foo_and_bar());
    sched.add(print_foo_and_wait());

    sched.run().await;
}
