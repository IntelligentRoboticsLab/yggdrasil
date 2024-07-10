use std::env;
use std::io;
use std::os::unix::net::UnixListener;

use serde::{Deserialize, Serialize};

use crate::schedule::DependencySystem;
use crate::schedule::Schedule;
use crate::storage::InspectableResource;
use crate::InspectView;

pub struct ControlSocket(UnixListener);

impl ControlSocket {
    pub fn new() -> io::Result<Self> {
        let path = env::var("TYR_SOCK").unwrap_or_else(|_| Self::runtime_path("tyr.sock"));

        let _ = std::fs::remove_file(&path);
        let sock = UnixListener::bind(path)?;
        sock.set_nonblocking(true)?;

        Ok(Self(sock))
    }

    pub fn tick(&self, schedule: &mut Schedule, view: &mut InspectView) -> io::Result<()> {
        for conn in self.0.incoming() {
            let stream = match conn {
                Ok(stream) => stream,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e),
            };

            let reader = io::BufReader::new(&stream);
            let mut de = serde_json::Deserializer::from_reader(reader);

            loop {
                match ControlMessage::deserialize(&mut de) {
                    Ok(ControlMessage::Resources) => {
                        let list: Vec<_> =
                            view.resources().map(|r| r.read().unwrap().name()).collect();

                        serde_json::to_writer(&stream, &list)?;
                    }
                    Ok(ControlMessage::Systems) => {
                        let list = schedule.list_systems();

                        serde_json::to_writer(&stream, &list)?;
                    }
                    Ok(ControlMessage::Get(name)) => {
                        if let Some(resource) = name.lookup_resource(view) {
                            let resource = resource.read().unwrap();
                            serde_json::to_writer(&stream, &resource.to_json())?;
                        } else {
                            break;
                        }
                    }
                    Ok(ControlMessage::Set { name, data }) => {
                        if let Some(resource) = name.lookup_resource(view) {
                            let mut resource = resource.write().unwrap();
                            resource.try_update_from_json(data);
                        } else {
                            break;
                        }
                    }
                    Ok(ControlMessage::Enable(name)) => {
                        if let Some(system) = name.lookup_system(schedule) {
                            system.enable(true);
                        }
                    }
                    Ok(ControlMessage::Disable(name)) => {
                        if let Some(system) = name.lookup_system(schedule) {
                            system.enable(false);
                        }
                    }
                    Err(e) if e.is_eof() => break,
                    Err(e) => return Err(e.into()),
                }
            }
        }

        Ok(())
    }

    pub fn runtime_path(name: &str) -> String {
        format!(
            "{}/{}",
            env::var("XDG_RUNTIME_DIR").as_deref().unwrap_or("/tmp"),
            name,
        )
    }
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
enum Name {
    Ident(String),
    Index(usize),
}

impl Name {
    fn lookup_resource<'a>(&self, view: &'a mut InspectView) -> Option<&'a InspectableResource> {
        match self {
            Self::Ident(name) => view.by_name(name),
            Self::Index(index) => view.by_index(*index),
        }
    }

    fn lookup_system<'a>(
        &self,
        schedule: &'a mut Schedule,
    ) -> Option<&'a mut DependencySystem<()>> {
        match self {
            Self::Ident(name) => schedule.get_system_by_name(name),
            Self::Index(index) => schedule.get_system_by_index(*index),
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum ControlMessage {
    Resources,
    Systems,
    Get(Name),
    Set { name: Name, data: serde_json::Value },
    Enable(Name),
    Disable(Name),
}
