// use std::fmt::Debug;

// use enum_dispatch::enum_dispatch;

// use super::behavior::{Context, FsmEnum};

// // // Define the FsmEnum trait, which is used to create new state objects
// // pub tra {
// //     fn create(enum_value: &S) -> Box<dyn Stateful<S, Context>>;
// // }

// // Define the Stateful trait, which contains the event handling methods for each state
// // #[enum_dispatch]
// pub trait Stateful<S: PartialEq + Eq + Clone> {
//     fn on_enter(&mut self, context: &mut Context) -> Response<S>;
//     fn execute(&mut self, context: &mut Context) -> Response<S>;
//     fn on_exit(&mut self, context: &mut Context);
// }

// // Define the Response enum, which is used to handle state transitions
// pub enum Response<S> {
//     Handled,
//     Transition(S),
// }

// // Define the Error enum, which is used to handle errors
// #[derive(Debug)]
// pub enum Error {
//     StateNotFound(String),
//     StateMachineNotInitialized,
// }

// // Define the StateMachine struct, which represents the finite state machine
// pub struct StateMachine<S: PartialEq + Eq + Clone> {
//     // states: HashMap<S, Box<dyn Stateful<S, Context>>>,
//     current_state: Box<dyn Stateful<S>>,
// }

// // Implement methods for the StateMachine struct
// impl<S: PartialEq + FsmEnum<S> + Eq + Clone + Stateful<S>> StateMachine<S> {
//     // Define a constructor for the StateMachine struct
//     pub fn new(initial_state: S) -> Self {
//         Self {
//             current_state: S::create(&initial_state),
//         }
//     }

//     // Define a method to get the current state
//     pub fn get_current_state(&self) -> &S {
//         &self.current_state
//     }

//     // Define a method to process events and transition between states
//     pub fn step(&mut self, context: &mut Context) -> Result<(), Error> {
//         match self.current_state.execute(context) {
//             Response::Handled => {}
//             Response::Transition(new_state) => {
//                 if !matches!(self.current_state.clone(), new_state) {
//                     self.transition_to(context, new_state)?;
//                 }
//             }
//         }

//         Ok(())
//     }

//     // Define a method to handle state transitions
//     fn transition_to(&mut self, context: &mut Context, new_state: S) -> Result<(), Error> {
//         self.current_state.on_exit(context);

//         self.current_state = S::create(&new_state);

//         loop {
//             match self.current_state.on_enter(context) {
//                 Response::Handled => {
//                     break;
//                 }
//                 Response::Transition(s) => {
//                     if s == *self.current_state.get_state() {
//                         break;
//                     } else {
//                         self.transition_to(context, s)?;
//                     }
//                 }
//             }
//         }

//         Ok(())
//     }
// }
use std::fmt::Debug;

// Define the FsmEnum trait, which is used to create new state objects
pub trait FsmEnum<S, CTX> {
    fn create(enum_value: &S) -> Box<dyn Stateful<S, CTX>>;
}

// Define the Stateful trait, which contains the event handling methods for each state
pub trait Stateful<S: PartialEq + Eq + Clone, CTX> {
    fn on_enter(&mut self, context: &mut CTX) -> Response<S>;
    fn execute(&mut self, context: &mut CTX) -> Response<S>;
    fn on_exit(&mut self, context: &mut CTX);
}

// Define the Response enum, which is used to handle state transitions
pub enum Response<S> {
    Handled,
    Transition(S),
}

// Define the Error enum, which is used to handle errors
#[derive(Debug)]
pub enum Error {
    StateNotFound(String),
    StateMachineNotInitialized,
}

// Define the StateMachine struct, which represents the finite state machine
pub struct StateMachine<S: PartialEq + Eq + Clone + FsmEnum<S, CTX>, CTX> {
    state_state: Box<dyn Stateful<S, CTX>>,
    current_state: S,
}

// Implement methods for the StateMachine struct
impl<S: PartialEq + Eq + Clone + FsmEnum<S, CTX>, CTX> StateMachine<S, CTX> {
    // Define a constructor for the StateMachine struct
    pub fn new(cur_state: S) -> Self {
        Self {
            current_state: cur_state.clone(),
            state_state: S::create(&cur_state),
        }
    }

    // Define a method to get the current state
    pub fn get_current_state(&self) -> &S {
        &self.current_state
    }

    // Define a method to process events and transition between states
    pub fn step(&mut self, context: &mut CTX) -> Result<(), Error> {
        match self.state_state.execute(context) {
            Response::Handled => {}
            Response::Transition(new_state) => {
                if new_state != self.current_state {
                    self.transition_to(new_state, context)?;
                }
            }
        }

        Ok(())
    }

    // Define a method to handle state transitions
    fn transition_to(&mut self, new_state: S, context: &mut CTX) -> Result<(), Error> {
        self.state_state.on_exit(context);

        self.current_state = new_state.clone();
        self.state_state = S::create(&self.current_state);
        loop {
            match self.state_state.on_enter(context) {
                Response::Handled => {
                    break;
                }
                Response::Transition(s) => {
                    if s == self.current_state {
                        break;
                    } else {
                        self.current_state = s.clone();
                        self.state_state = S::create(&self.current_state);
                    }
                }
            }
        }

        Ok(())
    }
}
