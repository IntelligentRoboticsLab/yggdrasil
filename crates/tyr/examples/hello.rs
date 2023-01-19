use tyr::data::*;
use tyr::system::*;
use tyr::scheduler::*;

#[derive(Data)]
struct Hello {
    foo: usize,
}

#[system(Hello)]
async fn increment_foo(foo: &mut usize) {
    *foo += 1;
}

#[system(Hello)]
async fn print_foo_and_wait(foo: &usize) {
    println!("{}", foo);
    std::thread::sleep(std::time::Duration::from_millis(500));
}

fn main() {
    let mut sched = Scheduler::new(Hello { foo: 0 });

    sched.add(increment_foo());
    sched.add(print_foo_and_wait());

    sched.run();
}
