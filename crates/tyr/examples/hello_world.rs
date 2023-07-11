use color_eyre::Result;
use tyr::{system, App, IntoSystemOrdering, Resource};

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    color_eyre::install()?;

    App::new()
        .add_resource(Resource::<Cheese>::default())?
        .add_resource(Resource::new(Sausage("Salami".to_string())))?
        .add_system(say_hi)
        .add_system(update_cheese)
        .add_system(say_bye.before(update_cheese).after(say_hi))
        .add_system(say_hi_again)
        .build()?
        .run()?;

    Ok(())
}

#[derive(Default)]
struct Cheese(String);

#[derive(Default)]
struct Sausage(String);

#[system]
fn say_hi(cheese: &Cheese) -> Result<()> {
    println!("Hi, currently the cheese is `{}`!", cheese.0);
    Ok(())
}

#[system]
fn say_hi_again(cheese: &Cheese) -> Result<()> {
    println!("Hello, currently the cheese is `{}`!", cheese.0);
    Ok(())
}

#[system]
fn update_cheese(cheese: &mut Cheese) -> Result<()> {
    cheese.0 = "Parmigiano Reggiano".to_string();
    Ok(())
}

#[system]
fn say_bye(cheese: &Cheese, sausage: &Sausage) -> Result<()> {
    println!(
        "Bye! the cheese was `{}`. The sausage was `{}`.",
        cheese.0, sausage.0
    );
    Ok(())
}
