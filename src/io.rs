use std::collections::{HashMap, HashSet};

use crate::{
    instance::instance::InstanceHandle,
    module::{PortId, PortValueBoxed},
};

#[derive(Debug)]
pub enum ConnectResult {
    Ok,
    Replace(PortHandle, PortHandle),
    SameInstance,
    InCompatible,
}

impl ConnectResult {
    pub fn as_result(self) -> Result<ConnectResult, ConnectResult> {
        match self {
            ConnectResult::Ok => Ok(self),
            ConnectResult::Replace(..) => Ok(self),
            ConnectResult::SameInstance => Err(self),
            ConnectResult::InCompatible => Err(self),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectResult::Ok => "ok",
            ConnectResult::Replace(..) => "replace",
            ConnectResult::SameInstance => "same instance",
            ConnectResult::InCompatible => "incompatible",
        }
    }
}

#[derive(Default)]
pub struct Io {
    inputs: HashMap<PortHandle, Box<dyn PortValueBoxed>>,
    connections: HashMap<PortHandle, HashSet<PortHandle>>,
}

impl Io {
    pub fn get_input(&self, port: PortHandle) -> Option<&Box<dyn PortValueBoxed>> {
        self.inputs.get(&port)
    }

    pub fn set_input(&mut self, port: PortHandle, value: Box<dyn PortValueBoxed>) {
        self.inputs.insert(port, value);
    }

    pub fn set_output(&mut self, port: PortHandle, value: Box<dyn PortValueBoxed>) {
        if let Some(connections) = self.connections.get(&port) {
            for connected in connections.clone().into_iter() {
                self.set_input(connected, value.clone())
            }
        }
    }

    pub fn input_connection(&self, input: PortHandle) -> Option<PortHandle> {
        for (from, connections) in self.connections.iter() {
            if connections.iter().find(|&&value| value == input).is_some() {
                return Some(*from);
            }
        }
        None
    }

    pub fn connect(
        &mut self,
        from: PortHandle,
        to: PortHandle,
    ) -> Result<ConnectResult, ConnectResult> {
        let can_connect = self.can_connect(from, to).as_result();
        if can_connect.is_err() {
            return can_connect;
        }

        if !self.connections.contains_key(&from) {
            self.connections.insert(from, HashSet::new());
        }
        let connections = self.connections.get_mut(&from).unwrap();

        connections.insert(to);

        can_connect
    }

    pub fn can_connect(&self, from: PortHandle, to: PortHandle) -> ConnectResult {
        let result = from.is_compatible(to);
        if let ConnectResult::Ok = result {
            if let Some(connection) = self.input_connection(to) {
                ConnectResult::Replace(connection, to)
            } else {
                result
            }
        } else {
            result
        }
    }

    pub fn disconnect(&mut self, from: PortHandle, to: PortHandle) {
        if let Some(connections) = self.connections.get_mut(&from) {
            connections.remove(&to);
            self.inputs.remove(&to);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PortHandle {
    pub id: PortId,
    pub instance: InstanceHandle,
}

impl PortHandle {
    pub fn new(id: PortId, instance: InstanceHandle) -> Self {
        Self { id, instance }
    }

    pub fn is_compatible(&self, other: Self) -> ConnectResult {
        if self.instance != other.instance {
            self.id.is_compatible(other.id)
        } else {
            ConnectResult::SameInstance
        }
    }
}
