use std::any::Any;

use ahash::{HashMap, HashMapExt, HashSet, HashSetExt};
use topological_sort::TopologicalSort;

use crate::{
    instance::instance::InstanceHandle,
    module::{Input, Port, PortId, PortValueBoxed},
};

#[derive(Clone, Copy, Debug)]
pub enum ConnectResultWarn {
    Replace(PortHandle, PortHandle),
}

impl ConnectResultWarn {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectResultWarn::Replace(..) => "replace",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ConnectResultErr {
    SameInstance,
    InCompatible,
}

impl ConnectResultErr {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectResultErr::SameInstance => "same instance",
            ConnectResultErr::InCompatible => "incompatible",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ConnectResult {
    Ok,
    Warn(ConnectResultWarn),
    Err(ConnectResultErr),
}

impl ConnectResult {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectResult::Ok => "ok",
            ConnectResult::Warn(w) => w.as_str(),
            ConnectResult::Err(e) => e.as_str(),
        }
    }
}

/// Facilitates the data interaction between modules.
#[derive(Default)]
pub struct Io {
    inputs: HashMap<PortHandle, Box<dyn PortValueBoxed>>,
    connections: HashMap<PortHandle, HashSet<PortHandle>>,
    processing_order: Vec<Vec<InstanceHandle>>,
}

impl Io {
    /// Gets the boxed input data.
    pub fn get_input_dyn(&self, port: PortHandle) -> Option<Box<dyn PortValueBoxed>> {
        self.inputs.get(&port).cloned()
    }

    /// Sets the data for an input port. Only should be used outside Io when this port is not connected.
    pub fn set_input_dyn(&mut self, port: PortHandle, value: Box<dyn PortValueBoxed>) {
        self.inputs.insert(port, value);
    }

    /// Tries to get the input data.
    fn try_get_input<I: Input>(&self, instance: InstanceHandle) -> Option<I::Type> {
        let boxed = self.get_input_dyn(PortHandle::new(I::id(), instance))?;
        let any = &*boxed as &dyn Any;
        any.downcast_ref::<I::Type>().cloned()
    }

    /// Gets input data or default value.
    pub fn get_input<I: Input>(&self, instance: InstanceHandle) -> I::Type {
        if let Some(value) = self.try_get_input::<I>(instance) {
            value
        } else {
            I::default()
        }
    }

    /// Propagates data to all connected ports
    pub fn set_output_dyn(&mut self, port: PortHandle, value: Box<dyn PortValueBoxed>) {
        if let Some(connections) = self.connections.get(&port) {
            for connected in connections.clone().into_iter() {
                self.set_input_dyn(connected, value.clone())
            }
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

    /// Connect two ports.
    pub fn connect(&mut self, from: PortHandle, to: PortHandle) -> ConnectResult {
        let can_connect = self.can_connect(from, to);
        if let ConnectResult::Err(_) = can_connect {
            return can_connect;
        }

        let connections = self.connections.entry(from).or_insert_with(HashSet::new);

        connections.insert(to);

        self.update_instances_processing_order();

        can_connect
    }

    pub fn can_connect(&self, from: PortHandle, to: PortHandle) -> ConnectResult {
        let result = from.is_compatible(to);
        if let ConnectResult::Ok | ConnectResult::Warn(_) = result {
            if let Some(connection) = self.input_connection(to) {
                ConnectResult::Warn(ConnectResultWarn::Replace(connection, to))
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
            self.update_instances_processing_order();
        }
    }

    /// Returns a [`HashSet`] containing the handles to all connected input ports.
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

    pub fn get_instances_dependencies(&self) -> HashMap<InstanceHandle, HashSet<InstanceHandle>> {
        let mut map = HashMap::new();

        for (&from, connections) in self.connections.iter() {
            for &to in connections {
                map.entry(to.instance)
                    .or_insert(HashSet::new())
                    .insert(from.instance);
            }
        }

        map
    }

    pub fn compute_instances_processing_order(&self) -> Result<Vec<Vec<InstanceHandle>>, &str> {
        let mut topo = TopologicalSort::<InstanceHandle>::new();
        let mut added = HashSet::new();
        for (instance, deps) in self.get_instances_dependencies() {
            for dep in deps {
                if !added.contains(&instance) || !added.contains(&dep) {
                    topo.add_dependency(dep, instance);
                    added.insert(dep);
                    added.insert(instance);
                }
            }
        }

        let mut list = Vec::new();
        while !topo.is_empty() {
            let elements = topo.pop_all();
            if elements.is_empty() {
                return Err("cyclic dependency");
            }
            list.push(elements)
        }

        Ok(list)
    }

    pub fn update_instances_processing_order(&mut self) {
        self.processing_order = self.compute_instances_processing_order().unwrap();
    }

    pub fn connections(&self) -> &HashMap<PortHandle, HashSet<PortHandle>> {
        &self.connections
    }

    pub fn processing_order(&self) -> &Vec<Vec<InstanceHandle>> {
        &self.processing_order
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PortHandle {
    pub id: PortId,
    pub instance: InstanceHandle,
}

impl PortHandle {
    pub fn new(id: PortId, instance: impl Into<InstanceHandle>) -> Self {
        Self {
            id,
            instance: instance.into(),
        }
    }

    pub fn is_compatible(&self, other: Self) -> ConnectResult {
        if self.instance == other.instance {
            ConnectResult::Err(ConnectResultErr::SameInstance)
        } else {
            self.id.is_compatible(other.id)
        }
    }
}
