use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
};

use crate::{
    instance::instance::InstanceHandle,
    module::{ConversionClosure, Input, Port, PortId, PortValueBoxed},
};

#[derive(Debug)]
pub enum ConnectResult {
    Ok,
    Replace(PortHandle, PortHandle),
    SameInstance,
    InCompatible,
}

impl ConnectResult {
    pub fn into_result(self) -> Result<ConnectResult, ConnectResult> {
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
    pub conversions: HashMap<ConversionId, Box<dyn ConversionClosure>>,
}

impl Io {
    pub fn get_input_dyn(&self, port: PortHandle) -> Option<Box<dyn PortValueBoxed>> {
        self.inputs.get(&port).cloned()
    }

    pub fn set_input_dyn(&mut self, port: PortHandle, value: Box<dyn PortValueBoxed>) {
        self.inputs.insert(port, value);
    }

    pub fn set_output_dyn(&mut self, port: PortHandle, value: Box<dyn PortValueBoxed>) {
        if let Some(connections) = self.connections.get(&port) {
            for connected in connections.clone().into_iter() {
                self.set_input_dyn(connected, value.clone())
            }
        }
    }

    fn try_get_input<I: Input>(&self, instance: InstanceHandle) -> Option<I::Type> {
        let boxed = self.get_input_dyn(PortHandle::new(I::id(), instance))?;

        if let Some(result) = {
            let any = &*boxed as &dyn Any;
            any.downcast_ref::<I::Type>()
        } {
            Some(result.clone())
        } else {
            let boxed: Box<dyn Any> = (self
                .conversions
                .get(&ConversionId {
                    port: I::id(),
                    input_type: (*boxed).type_id(),
                })
                .expect("should have this"))(boxed);

            let any = &*boxed;
            Some(any.downcast_ref::<I::Type>().unwrap().clone())
        }
    }

    pub fn get_input<I: Input>(&self, instance: InstanceHandle) -> I::Type {
        if let Some(value) = self.try_get_input::<I>(instance) {
            value
        } else {
            I::default()
        }
    }

    pub fn set_output<P: Port>(&mut self, instance: InstanceHandle, value: P::Type) {
        self.set_output_dyn(PortHandle::new(P::id(), instance), Box::new(value))
    }

    pub fn input_connection(&self, input: PortHandle) -> Option<PortHandle> {
        for (from, connections) in self.connections.iter() {
            if connections.iter().any(|&value| value == input) {
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
        let can_connect = self.can_connect(from, to).into_result();
        if can_connect.is_err() {
            return can_connect;
        }

        let connections = self.connections.entry(from).or_insert_with(HashSet::new);

        connections.insert(to);

        can_connect
    }

    pub fn can_connect(&self, from: PortHandle, to: PortHandle) -> ConnectResult {
        let mut result = from.is_compatible(to);

        if let ConnectResult::InCompatible = result {
            if self.conversions.contains_key(&ConversionId {
                port: to.id,
                input_type: from.id.value_type,
            }) {
                result = ConnectResult::Ok;
            }
        }

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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ConversionId {
    pub port: PortId,
    pub input_type: TypeId,
}
