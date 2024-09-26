// use enum_dispatch::enum_dispatch;

// use super::state::{Response, StateMachine, Stateful};

// #[derive(Eq, PartialEq, Clone, Debug)]
// pub struct Null;
// #[derive(Eq, PartialEq, Clone, Debug)]
// pub struct Starting;
// #[derive(Eq, PartialEq, Clone, Debug)]
// pub struct Ready;

// #[derive(Eq, PartialEq, Clone, Debug)]
// #[enum_dispatch(Stateful<State, Context>)]
// pub enum State {
//     Null(Null),
//     Starting(Starting),
//     Ready(Ready),
// }

// // impl FsmEnum<State, Context> for State {
// //     fn create(enum_value: &State) -> Box<dyn Stateful<State, Context>> {
// //         match enum_value {
// //             State::Null => Box::new(Null),
// //             State::Starting => Box::new(Starting),
// //             State::Ready => Box::new(Ready),
// //         }
// //     }
// // }

// impl ToString for State {
//     fn to_string(&self) -> String {
//         stringify!(self).to_owned()
//     }
// }

// impl Stateful<State, Context> for Null {
//     fn on_enter(&mut self, context: &mut Context) -> Response<State> {
//         println!("Null state on enter, retries = {}", context.retries);
//         Response::Handled
//     }

//     fn execute(&mut self, context: &mut Context) -> Response<State> {
//         println!("Null state on event");
//         Response::Transition(State::Starting(Starting))
//     }

//     fn on_exit(&mut self, context: &mut Context) {
//         println!("Null state on exit");
//     }
// }

// impl Stateful<State, Context> for Starting {
//     fn on_enter(&mut self, context: &mut Context) -> Response<State> {
//         println!("Starting state on enter");
//         context.retries += 1;
//         Response::Handled
//     }

//     fn execute(&mut self, context: &mut Context) -> Response<State> {
//         println!("Starting state on event retries = {}", context.retries);
//         context.retries += 1;
//         match context.retries {
//             1..=4 => Response::Handled,
//             _ => Response::Transition(State::Ready(Ready)),
//         }
//     }

//     fn on_exit(&mut self, context: &mut Context) {
//         println!("Starting state on exit");
//     }
// }

// impl Stateful<State, Context> for Ready {
//     fn on_enter(&mut self, context: &mut Context) -> Response<State> {
//         println!("Ready state on enter");
//         Response::Handled
//     }

//     fn execute(&mut self, context: &mut Context) -> Response<State> {
//         println!("Ready state on event");
//         // Response::Transition(State::Null)
//         Response::Handled
//     }

//     fn on_exit(&mut self, context: &mut Context) {
//         println!("Ready state on exit");
//     }
// }

// #[derive(Debug)]
// pub struct Context {
//     retries: u32,
// }

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_state_machine() {
//         let mut state_machine = StateMachine::<State, Context>::new(State::Null(Null));

//         // state_machine.init(State::Null).unwrap();
//         let mut context = Context { retries: 0 };

//         for _ in 0..10 {
//             if let Err(e) = state_machine.step(&mut context) {
//                 println!("state machine error : {:?}", e);
//             }
//         }
//     }
// }

use super::state::{FsmEnum, Response, StateMachine, Stateful};

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum State {
    Null,
    Starting,
    Ready,
}

impl FsmEnum<State, Context> for State {
    fn create(enum_value: &State) -> Box<dyn Stateful<State, Context>> {
        match enum_value {
            State::Null => Box::new(Null {}),
            State::Starting => Box::new(Starting {}),
            State::Ready => Box::new(Ready {}),
        }
    }
}

pub struct Null {}
pub struct Starting {}

pub struct Ready {}

impl ToString for State {
    fn to_string(&self) -> String {
        stringify!(self).to_owned()
    }
}

impl Stateful<State, Context> for Null {
    fn on_enter(&mut self, context: &mut Context) -> Response<State> {
        println!("Null state on enter, retries = {}", context.retries);
        Response::Handled
    }

    fn execute(&mut self, context: &mut Context) -> Response<State> {
        println!("Null state on event ");
        Response::Transition(State::Starting)
    }

    fn on_exit(&mut self, context: &mut Context) {
        println!("Null state on exit");
    }
}

impl Stateful<State, Context> for Starting {
    fn on_enter(&mut self, context: &mut Context) -> Response<State> {
        println!("Starting state on enter");
        context.retries = context.retries + 1;
        Response::Handled
    }

    fn execute(&mut self, context: &mut Context) -> Response<State> {
        println!("Starting state on event :");
        Response::Transition(State::Ready)
    }

    fn on_exit(&mut self, context: &mut Context) {
        println!("Starting state on exit");
    }
}

impl Stateful<State, Context> for Ready {
    fn on_enter(&mut self, context: &mut Context) -> Response<State> {
        println!("Ready state on enter");
        Response::Handled
    }

    fn execute(&mut self, context: &mut Context) -> Response<State> {
        println!("Ready state on event :");
        Response::Transition(State::Null)
    }

    fn on_exit(&mut self, context: &mut Context) {
        println!("Ready state on exit");
    }
}

#[derive(Debug)]
pub struct Context {
    retries: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_machine() {
        let mut state_machine =
            StateMachine::<State, Context>::new(State::Null, Context { retries: 0 });

        state_machine.init();

        for i in 0..10 {
            if let Err(e) = state_machine.step() {
                println!("state machine error : {:?}", e);
            }
        }
    }
}
