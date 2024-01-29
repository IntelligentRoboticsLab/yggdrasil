use miette::Result;
use tyr::prelude::*;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    App::new()
        .add_startup_system(initialize_first)?
        .add_startup_system(initialize_second)?
        .run()?;

    Ok(())
}

#[derive(Debug, Default)]
struct A(String);

#[derive(Debug, Default)]
struct B(String);

#[derive(Debug, Default)]
struct C(String);

#[startup_system]
fn initialize_first(storage: &mut Storage) -> Result<()> {
    let a = A("a".to_string());
    let b = B("b".to_string());

    storage.add_resource(Resource::new(a))?;
    storage.add_resource(Resource::new(b))?;

    Ok(())
}

#[startup_system]
fn initialize_second(storage: &mut Storage, a: &A, b: &mut B) -> Result<()> {
    b.0 = "🅱️".to_string();

    let c = C("c".to_string());

    println!("{}", a.0);
    println!("{}", b.0);

    storage.add_resource(Resource::new(c))?;

    Ok(())
}
