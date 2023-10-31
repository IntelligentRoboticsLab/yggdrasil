use miette::Result;
use tyr::prelude::*;

pub struct RoleModule;

impl Module for RoleModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_resource(Resource::new(Role::default()))?
            .add_system(assign_role))
    }
}

#[derive(Debug, PartialEq, Default)]
pub enum Role {
    #[default]
    ExampleRole,
}

#[system]
#[allow(unused_variables)]
fn assign_role(role: &mut Role) -> Result<()> {
    // TODO: update roles
    Ok(())
}
