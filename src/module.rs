use std::{
    any::{Any, TypeId},
    fmt::Debug,
    marker::PhantomData,
};

use dyn_clone::DynClone;
use eframe::{self, egui::Ui};

use crate::{
    io::{ConnectResult, Conversion, PortHandle},
    rack::rack::{ProcessContext, ShowContext},
};

pub trait Module: Any + 'static {
    fn describe() -> ModuleDescription<Self>
    where
        Self: Sized;

    fn process(&mut self, ctx: &mut ProcessContext);

    #[allow(unused)]
    fn show(&mut self, ctx: &ShowContext, ui: &mut Ui) {}
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

/// Non generic module description. Contains all data necessary for creating an instance.
#[derive(Clone)]
pub struct ModuleDescriptionDyn {
    pub name: String,
    pub instantiate: Box<dyn ModuleClosure>,
    pub inputs: Vec<PortDescriptionDyn>,
    pub outputs: Vec<PortDescriptionDyn>,
}

impl ModuleDescriptionDyn {
    pub fn from_typed<M>(description: ModuleDescription<M>) -> Self {
        Self {
            name: description.name,
            instantiate: description.instantiate,
            inputs: description.inputs,
            outputs: description.outputs,
        }
    }

    pub fn get_conversions(&self) -> Vec<&Conversion> {
        self.inputs
            .iter()
            .flat_map(|input| input.conversions.iter())
            .collect()
    }
}

pub struct ModuleDescription<M> {
    name: String,
    instantiate: Box<dyn ModuleClosure>,
    inputs: Vec<PortDescriptionDyn>,
    outputs: Vec<PortDescriptionDyn>,
    phantom: PhantomData<M>,
}

impl<M: Module> ModuleDescription<M> {
    pub fn new(closure: impl Fn() -> M + Clone + 'static) -> Self {
        Self {
            name: std::any::type_name::<M>().to_string(),
            instantiate: Box::new(move || Box::new(closure())),
            inputs: Vec::new(),
            outputs: Vec::new(),
            phantom: PhantomData,
        }
    }

    pub fn name(mut self, value: &str) -> Self {
        self.name = value.to_string();
        self
    }

    pub fn port<P: Port>(mut self, port: PortDescription<P>) -> Self {
        match port.port_type {
            PortType::Input => self.inputs.push(port.into_dyn()),
            PortType::Output => self.outputs.push(port.into_dyn()),
        }
        self
    }

    pub fn into_dyn(self) -> ModuleDescriptionDyn {
        ModuleDescriptionDyn::from_typed(self)
    }
}

pub trait PortValueBoxed: Any + DynClone + 'static {
    fn name() -> &'static str
    where
        Self: Sized;
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
        Self::Type::name()
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

pub trait InputClosureEdit: Fn(PortHandle, &mut ShowContext, &mut Ui) + DynClone {}

impl<F: Fn(PortHandle, &mut ShowContext, &mut Ui) + DynClone> InputClosureEdit for F {}

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

pub trait ConversionClosure: Fn(Box<dyn Any>) -> Box<dyn Any> + DynClone + 'static {}

impl<F: Fn(Box<dyn Any>) -> Box<dyn Any> + DynClone + 'static> ConversionClosure for F {}

impl Clone for Box<dyn ConversionClosure> {
    fn clone(&self) -> Self {
        dyn_clone::clone_box(&**self)
    }
}

#[derive(Clone)]
pub struct PortDescriptionDyn {
    pub name: &'static str,
    pub type_name: &'static str,
    pub port_type: PortType,
    pub id: PortId,
    pub closure_edit: Option<Box<dyn InputClosureEdit>>,
    pub closure_value: Option<Box<dyn InputClosureValue>>,
    pub conversions: Vec<Conversion>,
}

impl PortDescriptionDyn {
    pub fn from_typed<P: Port>(description: PortDescription<P>) -> Self {
        Self {
            name: P::name(),
            type_name: P::type_name(),
            port_type: description.port_type,
            id: P::id(),
            closure_edit: description.closure_edit,
            closure_value: description.closure_value,
            conversions: description.conversions,
        }
    }
}

pub struct PortDescription<P> {
    port_type: PortType,
    closure_edit: Option<Box<dyn InputClosureEdit>>,
    closure_value: Option<Box<dyn InputClosureValue>>,
    conversions: Vec<Conversion>,
    phantom: PhantomData<P>,
}

impl<P: Port> PortDescription<P> {
    pub fn input() -> Self
    where
        P: Input,
    {
        Self {
            port_type: PortType::Input,
            closure_edit: Some(Box::new(
                |handle: PortHandle, ctx: &mut ShowContext, ui: &mut Ui| {
                    let mut value = ctx.get_input::<P>(handle);

                    P::show(&mut value, ui);

                    ctx.set_input::<P>(handle, value)
                },
            )),
            closure_value: Some(Box::new(|handle: PortHandle, ctx: &ShowContext| {
                let value = ctx.get_input::<P>(handle);
                value.to_string()
            })),
            conversions: Vec::new(),
            phantom: PhantomData,
        }
    }

    pub fn output() -> Self {
        Self {
            port_type: PortType::Output,
            closure_edit: None,
            closure_value: None,
            conversions: Vec::new(),
            phantom: PhantomData,
        }
    }

    pub fn conversion<I: PortValueBoxed + Clone>(
        mut self,
        closure: impl Fn(I) -> P::Type + Clone + 'static,
    ) -> Self {
        let conversion = Conversion::new_input(P::id(), closure);
        self.conversions.push(conversion.unwrap());
        self
    }

    pub fn into_dyn(self) -> PortDescriptionDyn {
        PortDescriptionDyn::from_typed(self)
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
