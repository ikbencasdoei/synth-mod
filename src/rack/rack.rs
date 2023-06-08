use std::{
    any::Any,
    collections::{HashMap, HashSet},
};

use eframe::{
    self,
    egui::{self, Context, Sense, Ui},
};

use super::response::RackResponse;
use crate::{
    frame::Frame,
    instance::{
        instance::{Instance, InstanceHandle, InstanceResponse, TypedInstanceHandle},
        port::PortInstance,
    },
    io::{ConnectResult, ConversionId, Io, PortHandle},
    module::{Input, Module, ModuleDescription, Port, PortValueBoxed},
    modules::{
        audio::Audio, file::File, keyboard::Keyboard, ops::Operation, oscillator::Oscillator,
        scope::Scope, value::Value,
    },
};

#[derive(Clone)]
struct Panel {
    instances: Vec<InstanceHandle>,
    width: f32,
}

impl Panel {
    pub fn new() -> Self {
        Self {
            instances: Vec::new(),
            width: 0.0,
        }
    }

    pub fn add_instance(&mut self, handle: InstanceHandle) {
        self.instances.push(handle)
    }

    pub fn remove_instance(&mut self, handle: InstanceHandle) {
        self.instances.retain(|&instance| instance != handle);
        self.width = 0.0;
    }

    pub fn show(
        &self,
        rack: &mut Rack,
        index: usize,
        ui: &mut Ui,
        responses: &mut HashMap<InstanceHandle, InstanceResponse>,
        sample_rate: u32,
    ) {
        ui.vertical(|ui| {
            ui.set_min_width(100.0);
            ui.set_max_width(self.width);

            for handle in self.instances.iter() {
                let instance = rack.instances.get_mut(handle).unwrap();
                let mut ctx = ShowContext {
                    io: &mut rack.io,
                    instance: *handle,
                    sample_rate,
                };
                responses.insert(*handle, instance.show(&mut ctx, ui));
            }

            ui.menu_button("➕ Module", |ui| {
                for definition in rack.definitions.clone().iter() {
                    if ui.button(&definition.name).clicked() {
                        rack.add_module(definition, index);
                        ui.close_menu();
                    }
                }
            });

            rack.panels.get_mut(index).unwrap().width = ui.min_rect().size().x;
        });

        let sep_response = ui.separator();
        if ui
            .interact(sep_response.rect, sep_response.id, Sense::drag())
            .dragged()
        {
            // dbg!(index);
        }
    }
}

pub struct Rack {
    pub instances: HashMap<InstanceHandle, Instance>,
    panels: Vec<Panel>,
    definitions: Vec<ModuleDescription>,
    pub io: Io,
}

impl Default for Rack {
    fn default() -> Self {
        let mut new = Self {
            instances: Default::default(),
            panels: Vec::new(),
            definitions: Default::default(),
            io: Io::default(),
        };

        new.init_module::<Oscillator>();
        new.init_module::<Audio>();
        new.init_module::<Operation<f32>>();
        new.init_module::<Value<f32>>();
        new.init_module::<Scope>();
        new.init_module::<Keyboard>();
        new.init_module::<File>();

        new
    }
}

impl Rack {
    pub fn init_module<T: Module>(&mut self) {
        let def = T::describe();
        for input in def.inputs.iter() {
            for (type_id, closure) in &input.conversions {
                self.io.conversions.insert(
                    ConversionId {
                        port: input.id,
                        input_type: *type_id,
                    },
                    closure.clone(),
                );
            }
        }
        self.definitions.push(def)
    }

    pub fn add_module(&mut self, description: &ModuleDescription, panel: usize) -> InstanceHandle {
        let instance = Instance::from_description(description);
        let handle = instance.handle;
        self.instances.insert(handle, instance);
        self.panels.get_mut(panel).unwrap().add_instance(handle);
        handle
    }

    #[allow(unused)]
    pub fn add_module_typed<T: Module>(&mut self) -> TypedInstanceHandle<T> {
        if self.panels.get(0).is_none() {
            self.panels.push(Panel::new())
        }

        self.add_module(&T::describe(), 0).as_typed()
    }

    pub fn remove_instance(&mut self, handle: InstanceHandle) {
        self.io.remove_instance(handle);

        for panel in self.panels.iter_mut() {
            panel.remove_instance(handle)
        }

        self.instances.remove(&handle);
    }

    pub fn connect(&mut self, from: PortHandle, to: PortHandle) -> Result<(), &'static str> {
        match self.io.can_connect(from, to).into_result() {
            Err(err) => return Err(err.as_str()),
            Ok(can_connect) => {
                if let ConnectResult::Replace(from, to) = can_connect {
                    self.disconnect(from, to)
                }

                self.io
                    .connect(from, to)
                    .expect("io.can_connect should prevent this");

                Ok(())
            }
        }
    }

    pub fn can_connect(&self, from: PortHandle, to: PortHandle) -> ConnectResult {
        self.io.can_connect(from, to)
    }

    pub fn disconnect(&mut self, from: PortHandle, to: PortHandle) {
        self.io.disconnect(from, to);
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

    #[allow(unused)]
    pub fn get_port(&self, handle: PortHandle) -> Option<&PortInstance> {
        let instance = self.get_instance(handle.instance)?;
        instance.get_port(handle)
    }

    #[allow(unused)]
    pub fn get_port_mut(&mut self, handle: PortHandle) -> Option<&mut PortInstance> {
        let instance = self.get_instance_mut(handle.instance)?;
        instance.get_port_mut(handle)
    }

    pub fn show(&mut self, ctx: &Context, sample_rate: u32) {
        egui::SidePanel::right("rackplus")
            .exact_width(70.0)
            .resizable(false)
            .show(ctx, |ui| {
                if ui.button("➕ Panel").clicked() {
                    self.panels.push(Panel::new())
                }
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let mut responses = HashMap::new();

                    ui.horizontal_centered(|ui| {
                        for (i, panel) in self.panels.clone().into_iter().enumerate() {
                            panel.show(self, i, ui, &mut responses, sample_rate);
                        }
                    });

                    let response = RackResponse::new(responses);

                    response.show_connections(self, ui);
                    response.show_dragged(self, ui);
                    response.process(self);
                });
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

    pub fn get_input<I: Input>(&self) -> I::Type {
        self.io.get_input::<I>(self.handle)
    }

    pub fn set_output<P: Port>(&mut self, value: P::Type) {
        self.io.set_output::<P>(self.handle, value)
    }
}

pub struct ShowContext<'a> {
    io: &'a mut Io,
    pub instance: InstanceHandle,
    pub sample_rate: u32,
}

impl<'a> ShowContext<'a> {
    fn try_get_input<I: Input>(&self, handle: PortHandle) -> Option<I::Type> {
        let boxed = self.io.get_input_dyn(handle)?;
        let any = &*boxed as &dyn Any;
        Some(any.downcast_ref::<I::Type>()?.clone())
    }

    pub fn get_input_boxed(&self, handle: PortHandle) -> Option<Box<dyn PortValueBoxed>> {
        self.io.get_input_dyn(handle)
    }

    pub fn get_input<I: Input>(&self, handle: PortHandle) -> I::Type {
        if let Some(value) = self.try_get_input::<I>(handle) {
            value
        } else {
            I::default()
        }
    }

    pub fn set_input<P: Port>(&mut self, handle: PortHandle, value: P::Type) {
        self.io.set_input_dyn(handle, Box::new(value))
    }

    pub fn input_connections(&self, handle: PortHandle) -> Option<PortHandle> {
        self.io.input_connection(handle)
    }

    pub fn output_connections(&self, handle: PortHandle) -> HashSet<PortHandle> {
        self.io.output_connections(handle)
    }

    pub fn has_connection(&self, handle: PortHandle) -> bool {
        (!self.output_connections(handle).is_empty()) || self.input_connections(handle).is_some()
    }

    pub fn clear_port(&mut self, handle: PortHandle) {
        self.io.clear_port(handle)
    }
}
