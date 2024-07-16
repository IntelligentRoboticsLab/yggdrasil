use std::io::Write;

use miette::Result;
use serde::{Deserialize, Serialize};

use tyr::prelude::*;

fn main() -> Result<()> {
    let config = Config {
        sleep: 50,
        change: 1,
    };

    App::new()
        .init_inspectable_resource::<Data>()?
        .add_inspectable_resource(Resource::new(config))?
        .add_system(display)
        .add_system(step)
        .run()
}

#[system]
fn display(data: &Data) -> Result<()> {
    #[rustfmt::skip] // death to our ai overlords
    let repr = std::iter::repeat('-').take(data.single)
        .chain(std::iter::repeat('=').take(data.double))
        .collect::<String>();

    print!("\r{}", repr);
    std::io::stdout().flush().unwrap();

    Ok(())
}

#[system]
fn step(config: &Config, data: &mut Data) -> Result<()> {
    data.direction = match data.direction {
        Direction::Left if data.single == 0 => Direction::Right,
        Direction::Right if data.double == 0 => Direction::Left,
        direction => direction,
    };

    match data.direction {
        Direction::Left => {
            let change = config.change.min(data.single);
            data.single -= change;
            data.double += change;
        }
        Direction::Right => {
            let change = config.change.min(data.double);
            data.single += change;
            data.double -= change;
        }
    }

    let sleep = std::time::Duration::from_millis(config.sleep);
    std::thread::sleep(sleep);

    Ok(())
}

#[derive(Serialize, Deserialize, Inspect)]
struct Config {
    sleep: u64,
    change: usize,
}

#[derive(Serialize, Deserialize, Inspect)]
struct Data {
    single: usize,
    double: usize,
    direction: Direction,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
enum Direction {
    Left,
    Right,
}

impl Default for Data {
    fn default() -> Self {
        Self {
            single: 16,
            double: 0,
            direction: Direction::Right,
        }
    }
}
