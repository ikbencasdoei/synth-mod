use std::{
    any::{Any, TypeId},
    fmt::Debug,
};

use dyn_clone::DynClone;
use eframe::{self, egui::Ui};

use crate::{
    io::{ConnectResult, PortHandle},
    rack::rack::{ProcessContext, ShowContext},
};

pub trait Module: Any + 'static {
    fn describe() -> ModuleDescription
    where
        Self: Sized;

    #[allow(unused)]
    fn show(&mut self, ctx: &ShowContext, ui: &mut Ui) {}

    fn process(&mut self, ctx: &mut ProcessContext);
}

// impl Clone for Box<dyn Module> {
//     fn clone(&self) -> Self {
//         dyn_clone::clone_box(&**self)
//     }
// }

pub trait ModuleClosure: Fn() -> Box<dyn Module> + DynClone + 'static {}

impl<F: Fn() -> Box<dyn Module> + DynClone + 'static> ModuleClosure for F {}

impl Clone for Box<dyn ModuleClosure> {
    fn clone(&self) -> Self {
        dyn_clone::clone_box(&**self)
    }
}

#[derive(Clone)]
pub struct ModuleDescription {
    pub name: String,
    pub instatiate: Box<dyn ModuleClosure>,
    pub inputs: Vec<PortDescription>,
    pub outputs: Vec<PortDescription>,
}

impl ModuleDescription {
    pub fn new<M: Module>(closure: impl Fn() -> M + Clone + 'static) -> Self {
        Self {
            name: std::any::type_name::<M>().to_string(),
            instatiate: Box::new(move || Box::new(closure())),
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    pub fn set_name(mut self, value: &str) -> Self {
        self.name = value.to_string();
        self
    }

    pub fn add_input<I: Input>(mut self) -> Self {
        self.inputs.push(PortDescription::new_input::<I>());
        self
    }

    pub fn add_output<I: Port>(mut self) -> Self {
        self.outputs.push(PortDescription::new_output::<I>());
        self
    }
}

pub trait PortValueBoxed: Any + DynClone + 'static {
    fn to_string(&self) -> String;
    fn as_value(&self) -> f32;
}

// impl<T: Any + DynClone + 'static> PortValueBoxed for T {}

impl Clone for Box<dyn PortValueBoxed> {
    fn clone(&self) -> Self {
        dyn_clone::clone_box(&**self)
    }
}

pub trait Port: 'static {
    type Type: PortValueBoxed + Clone;

    fn name() -> &'static str;

    fn type_name() -> &'static str {
        std::any::type_name::<Self::Type>()
    }

    fn id() -> PortId
    where
        Self: Sized,
    {
        PortId::new::<Self>()
    }
}

pub trait Input: Port {
    fn default() -> Self::Type;

    #[allow(unused)]
    fn show(value: &mut Self::Type, ui: &mut Ui) {}
}

#[derive(Clone, Copy)]
pub enum PortType {
    Input,
    Output,
}

pub trait InputClosureEdit: Fn(PortHandle, &mut ShowContext, &mut Ui) -> () + DynClone {}

impl<F: Fn(PortHandle, &mut ShowContext, &mut Ui) -> () + DynClone> InputClosureEdit for F {}

impl Clone for Box<dyn InputClosureEdit> {
    fn clone(&self) -> Self {
        dyn_clone::clone_box(&**self)
    }
}

pub trait InputClosureValue: Fn(PortHandle, &ShowContext) -> String + DynClone {}

impl<F: Fn(PortHandle, &ShowContext) -> String + DynClone> InputClosureValue for F {}

impl Clone for Box<dyn InputClosureValue> {
    fn clone(&self) -> Self {
        dyn_clone::clone_box(&**self)
    }
}

#[derive(Clone)]
pub struct PortDescription {
    pub name: &'static str,
    pub type_name: &'static str,
    pub port_type: PortType,
    pub id: PortId,
    pub closure_edit: Option<Box<dyn InputClosureEdit>>,
    pub closure_value: Option<Box<dyn InputClosureValue>>,
}

impl PortDescription {
    pub fn new_input<I: Input>() -> Self {
        Self {
            name: I::name(),
            type_name: I::type_name(),
            port_type: PortType::Input,
            id: I::id(),
            closure_edit: Some(Box::new(
                |handle: PortHandle, ctx: &mut ShowContext, ui: &mut Ui| {
                    let mut value = ctx.get_input::<I>(handle);

                    I::show(&mut value, ui);

                    ctx.set_input::<I>(handle, value)
                },
            )),
            closure_value: Some(Box::new(|handle: PortHandle, ctx: &ShowContext| {
                let value = ctx.get_input::<I>(handle);
                value.to_string()
            })),
        }
    }

    pub fn new_output<I: Port>() -> Self {
        Self {
            name: I::name(),
            type_name: I::type_name(),
            port_type: PortType::Output,
            id: I::id(),
            closure_edit: None,
            closure_value: None,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PortId {
    pub id: TypeId,
    pub value_type: TypeId,
}

impl PortId {
    pub fn new<I: Port>() -> Self {
        Self {
            id: TypeId::of::<I>(),
            value_type: TypeId::of::<I::Type>(),
        }
    }

    pub fn is_compatible(&self, other: Self) -> ConnectResult {
        if self.value_type == other.value_type {
            ConnectResult::Ok
        } else {
            ConnectResult::InCompatible
        }
    }
}
