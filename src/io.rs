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
    pub connections: HashMap<PortHandle, HashSet<PortHandle>>,
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
            Some(self.try_convert::<I>(boxed).expect("should have this"))
        }
    }

    fn try_convert<I: Input>(&self, boxed: Box<dyn PortValueBoxed>) -> Option<I::Type> {
        let conversion = self.get_conversion::<I>((*boxed).type_id())?;
        let converted: Box<dyn Any> = (conversion)(boxed);
        let any = &*converted;
        Some(
            any.downcast_ref::<I::Type>()
                .expect("should be correct type")
                .clone(),
        )
    }

    fn get_conversion<I: Input>(&self, from_type: TypeId) -> Option<&Box<dyn ConversionClosure>> {
        let id = I::id();
        let conversion_id = ConversionId {
            from_type,
            to_type: id.value_type,
            to_port: Some(id),
        };
        self.conversions
            .get(&conversion_id)
            .or_else(|| self.conversions.get(&conversion_id.into_general()))
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

    ///Verifies whether the provided input port is connected, and if it is, it returns the handle of the output port.
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
            let conversion_id = ConversionId::from_ports(from, to);
            if self.conversions.contains_key(&conversion_id)
                || self.conversions.contains_key(&conversion_id.into_general())
            {
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

    pub fn output_connections(&self, handle: PortHandle) -> HashSet<PortHandle> {
        self.connections.get(&handle).cloned().unwrap_or_default()
    }

    pub fn clear_port(&mut self, handle: PortHandle) {
        if let Some(output) = self.input_connection(handle) {
            self.disconnect(output, handle)
        }

        for input in self.output_connections(handle) {
            self.disconnect(handle, input)
        }
    }

    pub fn instance_ports(&self, instance: InstanceHandle) -> Vec<PortHandle> {
        self.connections
            .iter()
            .flat_map(|(from, connections)| connections.iter().chain(std::iter::once(from)))
            .filter(|handle| handle.instance == instance)
            .copied()
            .collect()
    }

    pub fn remove_instance(&mut self, instance: InstanceHandle) {
        for port in self.instance_ports(instance) {
            self.clear_port(port)
        }
    }

    pub fn add_conversion(&mut self, conversion: Conversion) {
        self.conversions.insert(conversion.id, conversion.closure);
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
        self.id.is_compatible(other.id)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ConversionId {
    pub from_type: TypeId,
    pub to_type: TypeId,
    pub to_port: Option<PortId>,
}

impl ConversionId {
    pub fn from_ports(from: PortHandle, to: PortHandle) -> Self {
        Self {
            from_type: from.id.value_type,
            to_type: to.id.value_type,
            to_port: Some(to.id),
        }
    }

    pub fn into_general(self) -> Self {
        Self {
            from_type: self.from_type,
            to_type: self.to_type,
            to_port: None,
        }
    }
}

#[derive(Clone)]
pub struct Conversion {
    pub id: ConversionId,
    closure: Box<dyn ConversionClosure>,
}

impl Conversion {
    pub fn new_input<I: PortValueBoxed + Clone, O: PortValueBoxed>(
        port: PortId,
        closure: impl Fn(I) -> O + Clone + 'static,
    ) -> Option<Self> {
        if TypeId::of::<O>() != port.value_type {
            return None;
        }

        Some(Self {
            id: ConversionId {
                from_type: TypeId::of::<I>(),
                to_type: port.value_type,
                to_port: Some(port),
            },
            closure: Box::new(move |boxed: Box<dyn Any>| {
                let any = &*boxed as &dyn Any;
                Box::new(closure(any.downcast_ref::<I>().unwrap().clone()))
            }),
        })
    }

    pub fn new_type<I: PortValueBoxed + Clone, O: PortValueBoxed>(
        closure: impl Fn(I) -> O + Clone + 'static,
    ) -> Self {
        Self {
            id: ConversionId {
                from_type: TypeId::of::<I>(),
                to_type: TypeId::of::<O>(),
                to_port: None,
            },
            closure: Box::new(move |boxed: Box<dyn Any>| {
                let any = &*boxed as &dyn Any;
                Box::new(closure(any.downcast_ref::<I>().unwrap().clone()))
            }),
        }
    }
}
