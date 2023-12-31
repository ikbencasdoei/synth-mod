use std::{
    any::Any,
    sync::mpsc::{Receiver, Sender},
};

use ahash::{HashMap, HashMapExt, HashSet};
use eframe::{
    self,
    egui::{self, Button, Context, Ui},
};

use super::response::RackResponse;
#[cfg(not(target_arch = "wasm32"))]
use crate::modules::file::File;
use crate::{
    frame::Frame,
    instance::{
        instance::{Instance, InstanceHandle, InstanceResponse, TypedInstanceHandle},
        port::PortInstance,
    },
    io::{ConnectResult, ConnectResultWarn, Io, PortHandle},
    module::{Input, Module, ModuleDescriptionDyn, Port, PortValueBoxed},
    modules::{
        audio::Audio, filter::Filter, keyboard::Keyboard, noise::Noise, ops::Operation,
        oscillator::Oscillator, scope::Scope, value::Value,
    },
    types::{Type, TypeDefinitionDyn},
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
                for definition in rack.modules.clone().iter() {
                    if ui.button(&definition.name).clicked() {
                        rack.add_module(definition, index);
                        ui.close_menu();
                    }
                }
            });

            rack.panels.get_mut(index).unwrap().width = ui.min_rect().size().x;
        });

        ui.separator();
    }
}

/// Holds, draws, creates and modifies module instances and their connections.
pub struct Rack {
    pub instances: HashMap<InstanceHandle, Instance>,
    panels: Vec<Panel>,
    pub modules: Vec<ModuleDescriptionDyn>,
    types: Vec<TypeDefinitionDyn>,
    pub io: Io,
    sender: Sender<Frame>,
    receiver: Receiver<Frame>,
}

impl Default for Rack {
    fn default() -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();

        let mut new = Self {
            instances: Default::default(),
            panels: Vec::new(),
            modules: Vec::new(),
            types: Vec::new(),
            io: Io::default(),
            sender,
            receiver,
        };

        new.init_type::<f32>();
        new.init_type::<bool>();
        new.init_type::<Frame>();

        new.init_module::<Oscillator>();
        new.init_module::<Audio>();
        new.init_module::<Operation<f32>>();
        new.init_module::<Value<f32>>();
        new.init_module::<Scope>();
        new.init_module::<Keyboard>();
        #[cfg(not(target_arch = "wasm32"))]
        new.init_module::<File>();
        new.init_module::<Filter>();
        new.init_module::<Noise>();

        new
    }
}

impl Rack {
    fn init_type<T: Type>(&mut self) {
        let definition = T::define().into_dyn();

        for conversion in definition.conversions.iter() {
            self.io.add_conversion(conversion.clone())
        }

        self.types.push(definition)
    }

    fn init_module<T: Module>(&mut self) {
        let def = T::describe().into_dyn();

        for conversion in def.get_conversions() {
            self.io.add_conversion(conversion.clone())
        }

        self.modules.push(def)
    }

    pub fn add_module(
        &mut self,
        description: &ModuleDescriptionDyn,
        panel: usize,
    ) -> InstanceHandle {
        let mut instance = Instance::from_description(description);

        if let Some(audio) = instance.get_module_mut::<Audio>() {
            audio.sender = Some(self.sender.clone());
        }

        let handle = instance.handle;
        self.instances.insert(handle, instance);
        self.panels.get_mut(panel).unwrap().add_instance(handle);
        handle
    }

    pub fn add_panel(&mut self) {
        self.panels.push(Panel::new())
    }

    #[allow(unused)]
    pub fn add_module_typed<T: Module>(&mut self) -> TypedInstanceHandle<T> {
        if self.panels.get(0).is_none() {
            self.panels.push(Panel::new())
        }

        self.add_module(&T::describe().into_dyn(), 0).as_typed()
    }

    pub fn remove_instance(&mut self, handle: InstanceHandle) {
        self.io.remove_instance(handle);

        for panel in self.panels.iter_mut() {
            panel.remove_instance(handle)
        }

        self.instances.remove(&handle);
    }

    pub fn connect(&mut self, from: PortHandle, to: PortHandle) -> Result<(), &'static str> {
        let result = self.io.can_connect(from, to);

        match result {
            ConnectResult::Ok | ConnectResult::Warn(_) => {
                if let ConnectResult::Warn(ConnectResultWarn::Replace(from, to)) = result {
                    self.disconnect(from, to)
                }

                self.io.connect(from, to);

                Ok(())
            }
            ConnectResult::Err(err) => return Err(err.as_str()),
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
    pub fn get_module<T: Module>(&self, handle: TypedInstanceHandle<T>) -> Option<&T> {
        self.instances.get(&handle.as_untyped())?.get_module()
    }

    #[allow(unused)]
    pub fn get_module_mut<T: Module>(&mut self, handle: TypedInstanceHandle<T>) -> Option<&mut T> {
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
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let mut responses = HashMap::new();

                    ui.horizontal_centered(|ui| {
                        for (i, panel) in self.panels.clone().into_iter().enumerate() {
                            panel.show(self, i, ui, &mut responses, sample_rate);
                        }

                        ui.vertical(|ui| {
                            if ui.add(Button::new("➕ Panel").wrap(false)).clicked() {
                                self.add_panel()
                            }
                        });
                    });

                    let response = RackResponse::new(responses);

                    response.show_connections(self, ui);
                    response.show_dragged(self, ui);
                    response.process(self);
                });
        });
    }

    pub fn process_amount(&mut self, sample_rate: u32, amount: usize) -> Vec<Vec<Frame>> {
        puffin::profile_function!();

        let mut frames = Vec::with_capacity(amount);
        let order = self.io.processing_order().clone();

        //to minimize hashmap lookups pointers are used
        //SAFETY: contents of the hashmap should not change and the every handle should be unique.
        let pointers = {
            order
                .iter()
                .flatten()
                .map(|handle| self.instances.get_mut(handle).unwrap() as *mut _)
                .collect::<Vec<_>>()
        };

        {
            puffin::profile_scope!("frames");

            let mut ctx = ProcessContext {
                sample_rate,
                handle: InstanceHandle::new(),
                io: &mut self.io,
            };

            for _ in 0..amount {
                for pointer in pointers.iter() {
                    let instance: &mut Instance = unsafe { &mut **pointer };
                    ctx.handle = instance.handle;

                    instance.module.process(&mut ctx)
                }

                frames.push(self.receiver.try_iter().collect::<Vec<_>>());
            }
        }

        frames
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
        self.io.clear_port(handle);
    }
}
