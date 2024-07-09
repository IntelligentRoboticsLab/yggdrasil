use miette::Result;
use serde::{Deserialize, Serialize};
use tyr::prelude::*;

fn main() -> Result<()> {
    App::new()
        .init_inspectable_resource::<Cheese>()?
        .add_inspectable_resource(Resource::new(Sausage("Salami".to_string())))?
        .add_system(dump_debug_info)
        .add_system(say_hi)
        .add_system(update_cheese)
        .add_system(say_bye.before(update_cheese).after(say_hi))
        .add_system(say_hi_again)
        .run()
}

#[derive(Debug, Default, Serialize, Deserialize, Inspect)]
struct Cheese(String);

#[derive(Debug, Default, Serialize, Deserialize, Inspect)]
struct Sausage(String);

#[system]
fn dump_debug_info(view: &tyr::InspectView) -> Result<()> {
    for res in view.resources() {
        let res = res.read().unwrap();

        let (name, json) = (res.name(), res.to_json());
        println!("{}: {}", name, json);
    }

    Ok(())
}

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
