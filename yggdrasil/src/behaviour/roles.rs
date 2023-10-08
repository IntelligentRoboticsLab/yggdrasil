use miette::Result;
use tyr::prelude::*;

pub struct RoleModule;

impl Module for RoleModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_resource(Resource::new(Role::new()))?
            .add_system(assign_role))
    }
}

// TODO: add other roles
pub enum Role {
    Keeper,
}

impl Role {
    // TODO: retrieve role based on player number
    pub fn new() -> Self {
        Role::Keeper
    }
}

#[system]
#[allow(unused_variables)]
fn assign_role(role: &mut Role) -> Result<()> {
    // TODO: update roles
    Ok(())
}
