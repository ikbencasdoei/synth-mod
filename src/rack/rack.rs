use std::{any::Any, collections::HashMap};

use eframe::{
    self,
    egui::{self, Ui},
};
use indexmap::IndexMap;

use super::response::RackResponse;
use crate::{
    frame::Frame,
    instance::{
        instance::{Instance, InstanceHandle, TypedInstanceHandle},
        port::PortInstance,
    },
    io::{ConnectResult, Io, PortHandle},
    module::{Input, Module, ModuleDescription, Port, PortValueBoxed},
    modules::{audio::Audio, ops::Operation, oscillator::Oscillator, scope::Scope, value::Value},
};

pub struct Rack {
    pub instances: IndexMap<InstanceHandle, Instance>,
    definitions: Vec<ModuleDescription>,
    io: Io,
}

impl Default for Rack {
    fn default() -> Self {
        let mut new = Self {
            instances: Default::default(),
            definitions: Default::default(),
            io: Io::default(),
        };

        new.init_module::<Oscillator>();
        new.init_module::<Audio>();
        new.init_module::<Operation<f32>>();
        new.init_module::<Value<f32>>();
        new.init_module::<Scope>();

        new
    }
}

impl Rack {
    pub fn init_module<T: Module>(&mut self) {
        self.definitions.push(T::describe())
    }

    pub fn add_module(&mut self, description: &ModuleDescription) -> InstanceHandle {
        let instance = Instance::from_description(description);
        let handle = instance.handle;
        self.instances.insert(handle, instance);
        handle
    }

    #[allow(unused)]
    pub fn add_module_typed<T: Module>(&mut self) -> TypedInstanceHandle<T> {
        self.add_module(&T::describe()).as_typed()
    }

    pub fn remove_module(&mut self, handle: InstanceHandle) {
        if !self.instances.contains_key(&handle) {
            return;
        }
        //remove connections
        //connections from module
        let connections_from = {
            let module = self.instances.get(&handle).unwrap();
            module
                .outputs
                .iter()
                .map(|(&handle, port)| {
                    (
                        handle,
                        port.connections.clone().into_iter().collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>()
        };

        for (from, connections) in connections_from {
            for to in connections {
                self.disconnect(from, to)
            }
        }

        //connections to module
        let connections_to = {
            let module = self.instances.get(&handle).unwrap();
            module
                .inputs
                .iter()
                .map(|(&handle, port)| {
                    (
                        handle,
                        port.connections.clone().into_iter().collect::<Vec<_>>(),
                    )
                })
                .collect::<Vec<_>>()
        };

        for (to, connections) in connections_to {
            for from in connections {
                self.disconnect(from, to)
            }
        }

        self.instances.shift_remove(&handle);
    }

    pub fn connect(&mut self, from: PortHandle, to: PortHandle) -> Result<(), &'static str> {
        match self.io.can_connect(from, to).as_result() {
            Err(err) => return Err(err.as_str()),
            Ok(can_connect) => {
                if let ConnectResult::Replace(from, to) = can_connect {
                    self.disconnect(from, to)
                }

                self.io
                    .connect(from, to)
                    .expect("io.can_connect should prevent this");

                if let Some(from_port) = self.get_port_mut(from) {
                    from_port.connections.insert(to);
                }

                if let Some(to_port) = self.get_port_mut(to) {
                    to_port.connections.insert(from);
                }

                Ok(())
            }
        }
    }

    pub fn can_connect(&self, from: PortHandle, to: PortHandle) -> ConnectResult {
        self.io.can_connect(from, to)
    }

    pub fn disconnect(&mut self, from: PortHandle, to: PortHandle) {
        self.io.disconnect(from, to);

        if let Some(port) = self.get_port_mut(from) {
            port.connections.remove(&to);
        }

        if let Some(port) = self.get_port_mut(to) {
            port.connections.remove(&from);
        }
    }

    pub fn get_instance(&self, handle: InstanceHandle) -> Option<&Instance> {
        self.instances.get(&handle)
    }

    pub fn get_instance_mut(&mut self, handle: InstanceHandle) -> Option<&mut Instance> {
        self.instances.get_mut(&handle)
    }

    #[allow(unused)]
    pub fn get_module<T: Module>(&self, handle: &TypedInstanceHandle<T>) -> Option<&T> {
        self.instances.get(&handle.as_untyped())?.get_module()
    }

    #[allow(unused)]
    pub fn get_module_mut<T: Module>(&mut self, handle: &TypedInstanceHandle<T>) -> Option<&mut T> {
        self.instances
            .get_mut(&handle.as_untyped())?
            .get_module_mut()
    }

    pub fn get_port(&self, handle: PortHandle) -> Option<&PortInstance> {
        let instance = self.get_instance(handle.instance)?;
        instance.get_port(handle)
    }

    pub fn get_port_mut(&mut self, handle: PortHandle) -> Option<&mut PortInstance> {
        let instance = self.get_instance_mut(handle.instance)?;
        instance.get_port_mut(handle)
    }

    pub fn show(&mut self, ui: &mut Ui, sample_rate: u32) {
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show(ui, |ui| {
                let responses = self
                    .instances
                    .iter_mut()
                    .map(|(handle, instance)| {
                        (*handle, {
                            let mut ctx = ShowContext {
                                io: &mut self.io,
                                instance: *handle,
                                sample_rate,
                            };
                            instance.show(&mut ctx, ui)
                        })
                    })
                    .collect::<HashMap<_, _>>();

                let response = RackResponse::new(responses);

                response.show_connections(&self, ui);
                response.show_dragged(self, ui);
                response.process(self);

                ui.menu_button("➕", |ui| {
                    for definition in self.definitions.clone().iter() {
                        if ui.button(&definition.name).clicked() {
                            self.add_module(definition);
                            ui.close_menu();
                        }
                    }
                })
            });
    }

    pub fn process(&mut self, sample_rate: u32) -> Vec<Frame> {
        for instance in self.instances.values_mut() {
            let mut ctx = ProcessContext {
                sample_rate,
                handle: instance.handle,
                io: &mut self.io,
            };

            instance.module.process(&mut ctx)
        }

        let outputs = self
            .instances
            .values()
            .flat_map(|instance| (&*instance.module as &dyn Any).downcast_ref::<Audio>())
            .collect::<Vec<_>>();

        outputs
            .iter()
            .flat_map(|output| output.current_frame())
            .collect::<Vec<_>>()
    }
}

pub struct ProcessContext<'a> {
    sample_rate: u32,
    handle: InstanceHandle,
    io: &'a mut Io,
}

impl<'a> ProcessContext<'a> {
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    fn try_get_input<I: Input>(&self) -> Option<I::Type> {
        let boxed = self.io.get_input(PortHandle::new(I::id(), self.handle))?;
        let any = &**boxed as &dyn Any;
        Some(any.downcast_ref::<I::Type>()?.clone())
    }

    pub fn get_input<I: Input>(&self) -> I::Type {
        if let Some(value) = self.try_get_input::<I>() {
            value
        } else {
            I::default()
        }
    }

    pub fn set_output<P: Port>(&mut self, value: P::Type) {
        self.io
            .set_output(PortHandle::new(P::id(), self.handle), Box::new(value))
    }
}

pub struct ShowContext<'a> {
    io: &'a mut Io,
    pub instance: InstanceHandle,
    pub sample_rate: u32,
}

impl<'a> ShowContext<'a> {
    fn try_get_input<I: Input>(&self, handle: PortHandle) -> Option<I::Type> {
        let boxed = self.io.get_input(handle)?;
        let any = &**boxed as &dyn Any;
        Some(any.downcast_ref::<I::Type>()?.clone())
    }

    pub fn get_input_boxed(&self, handle: PortHandle) -> Option<&Box<dyn PortValueBoxed>> {
        self.io.get_input(handle)
    }

    pub fn get_input<I: Input>(&self, handle: PortHandle) -> I::Type {
        if let Some(value) = self.try_get_input::<I>(handle) {
            value
        } else {
            I::default()
        }
    }

    pub fn set_input<P: Port>(&mut self, handle: PortHandle, value: P::Type) {
        self.io.set_input(handle, Box::new(value))
    }
}
