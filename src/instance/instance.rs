use std::{any::Any, marker::PhantomData, ops::Index};

use ahash::{HashMap, HashMapExt};
use eframe::{
    egui::{self, Sense, Ui},
    epaint::Hsva,
};
use indexmap::IndexMap;
use uuid::Uuid;

use super::port::{PortInstance, PortResponse};
use crate::{
    io::PortHandle,
    module::{Module, ModuleDescriptionDyn},
    rack::rack::ShowContext,
    util::random_color,
};

/// Holds an instance of a [`Module`].
pub struct Instance {
    pub module: Box<dyn Module>,
    pub description: ModuleDescriptionDyn,
    pub handle: InstanceHandle,
    pub inputs: IndexMap<PortHandle, PortInstance>,
    pub outputs: IndexMap<PortHandle, PortInstance>,
    handle_color: Hsva,
}

impl Instance {
    pub fn from_description(description: &ModuleDescriptionDyn) -> Self {
        let handle = InstanceHandle::new();

        let inputs = description
            .inputs
            .iter()
            .map(|description| {
                (
                    PortHandle::new(description.id, handle),
                    PortInstance::from_description(description, handle),
                )
            })
            .collect::<IndexMap<_, _>>();

        let outputs = description
            .outputs
            .iter()
            .map(|description| {
                (
                    PortHandle::new(description.id, handle),
                    PortInstance::from_description(description, handle),
                )
            })
            .collect::<IndexMap<_, _>>();

        Self {
            module: (description.instantiate)(),
            description: description.clone(),
            handle,
            inputs,
            outputs,
            handle_color: random_color(),
        }
    }

    #[allow(unused)]
    pub fn get_port(&self, handle: PortHandle) -> Option<&PortInstance> {
        self.inputs
            .get(&handle)
            .or_else(|| self.outputs.get(&handle))
    }

    #[allow(unused)]
    pub fn get_port_mut(&mut self, handle: PortHandle) -> Option<&mut PortInstance> {
        self.inputs
            .get_mut(&handle)
            .or_else(|| self.outputs.get_mut(&handle))
    }

    pub fn get_module<M: Module>(&self) -> Option<&M> {
        (&*self.module as &dyn Any).downcast_ref()
    }

    pub fn get_module_mut<M: Module>(&mut self) -> Option<&mut M> {
        (&mut *self.module as &mut dyn Any).downcast_mut()
    }

    pub fn show(&mut self, ctx: &mut ShowContext, ui: &mut Ui) -> InstanceResponse {
        let mut response = InstanceResponse::new(self);
        ui.horizontal(|ui| {
            ui.heading(&self.description.name);

            let handle_response = ui.add(
                egui::Label::new(
                    egui::RichText::new(self.handle.to_string()).color(self.handle_color),
                )
                .sense(Sense::click()),
            );

            if handle_response.clicked() {
                self.handle_color = random_color()
            }

            ui.menu_button("🗑", |ui| {
                if ui.button("Are you sure?").clicked() {
                    response.remove = true;
                    ui.close_menu();
                }
            });
        });

        self.module.show(ctx, ui);

        ui.horizontal(|ui| {
            for port in self.inputs.values_mut() {
                response.ports.insert(port.handle, port.show(ctx, ui));
            }

            if !self.inputs.is_empty() && !self.outputs.is_empty() {
                ui.separator();
            }

            for port in self.outputs.values_mut() {
                response.ports.insert(port.handle, port.show(ctx, ui));
            }
        });

        ui.separator();

        response
    }
}

/// Contains useful data after drawing an instance's ui.
pub struct InstanceResponse {
    pub handle: InstanceHandle,
    pub remove: bool,
    pub ports: HashMap<PortHandle, PortResponse>,
}

impl InstanceResponse {
    pub fn new(instance: &Instance) -> Self {
        Self {
            handle: instance.handle,
            remove: false,
            ports: HashMap::new(),
        }
    }

    pub fn get_port_response(&self, handle: PortHandle) -> Option<&PortResponse> {
        self.ports.get(&handle)
    }
}

/// Identifies an [`Instance`]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct InstanceHandle {
    id: Uuid,
}

impl InstanceHandle {
    /// Generates a random handle.
    pub fn new() -> Self {
        Self { id: Uuid::new_v4() }
    }

    pub fn from_typed<T>(typed: &TypedInstanceHandle<T>) -> Self {
        Self { id: typed.id }
    }

    pub fn to_string(&self) -> String {
        self.id.as_simple().to_string().index(..8).to_string()
    }

    pub fn as_typed<T>(&self) -> TypedInstanceHandle<T> {
        TypedInstanceHandle::from_untyped(*self)
    }
}

impl<T> From<TypedInstanceHandle<T>> for InstanceHandle {
    fn from(value: TypedInstanceHandle<T>) -> Self {
        value.as_untyped()
    }
}

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct TypedInstanceHandle<T> {
    id: Uuid,
    phantom: PhantomData<T>,
}

impl<T> Clone for TypedInstanceHandle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            phantom: self.phantom.clone(),
        }
    }
}

impl<T> Copy for TypedInstanceHandle<T> {}

impl<T> TypedInstanceHandle<T> {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            phantom: PhantomData,
        }
    }

    pub fn from_untyped(untyped: InstanceHandle) -> Self {
        Self {
            id: untyped.id,
            phantom: PhantomData,
        }
    }

    pub fn as_untyped(&self) -> InstanceHandle {
        InstanceHandle::from_typed(self)
    }
}
