use miette::Result;
use tyr::prelude::*;

pub struct GamePhaseModule;

impl Module for GamePhaseModule {
    fn initialize(self, app: App) -> miette::Result<App> {
        Ok(app
            .add_resource(Resource::new(GamePhase::Normal))?
            .add_system(update_game_phase))
    }
}

#[allow(dead_code)]
pub enum GamePhase {
    Normal,
    PenaltyShootout,
    Overtime,
    Timeout,
}

#[system]
#[allow(unused_variables)]
fn update_game_phase(game_state: &mut GamePhase) -> Result<()> {
    // TODO: update game phase based on game controller
    Ok(())
}
