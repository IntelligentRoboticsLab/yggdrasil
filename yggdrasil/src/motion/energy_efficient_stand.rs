use miette::Result;
use tyr::prelude::*;
use nidhogg::NaoState;

#[derive(Default)]
struct EnergySaver {
    activate: bool,
}

#[system]
fn Efficient_stand(nao_state: &NaoState, 
                   energy_saver: &EnergySaver) -> Result<()> {
    let current = nao_state.current.clone();
}

impl Module for FooModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_resource(Resource::<i32>::new(32))?
            .add_system(bar_system))
    }
}

fn main() -> Result<()> {
    App::new().add_module(FooModule)?.run()
}
