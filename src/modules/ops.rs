use std::{
    marker::PhantomData,
    ops::{Add, Div, Mul, Sub},
};

use eframe::egui::{self, Ui};
use enum_iterator::Sequence;

use crate::{
    module::{Input, Module, ModuleDescription, Port, PortDescription, PortValueBoxed},
    rack::rack::{ProcessContext, ShowContext},
    util::EnumIter,
};

pub struct InValueA<T>(PhantomData<T>);

impl<T: PortValueBoxed + Clone> Port for InValueA<T> {
    type Type = T;

    fn name() -> &'static str {
        "a"
    }
}

impl Input for InValueA<f32> {
    fn default() -> Self::Type {
        Self::Type::default()
    }

    fn show(value: &mut Self::Type, ui: &mut Ui) {
        ui.add(
            egui::DragValue::new(value)
                .clamp_range(0.0..=f32::MAX)
                .speed(1.0),
        );
    }
}

pub struct InValueB<T>(PhantomData<T>);

impl<T: PortValueBoxed + Clone> Port for InValueB<T> {
    type Type = T;

    fn name() -> &'static str {
        "b"
    }
}

impl Input for InValueB<f32> {
    fn default() -> Self::Type {
        Self::Type::default()
    }

    fn show(value: &mut Self::Type, ui: &mut Ui) {
        ui.add(
            egui::DragValue::new(value)
                .clamp_range(0.0..=f32::MAX)
                .speed(1.0),
        );
    }
}

pub struct OutValue<T>(PhantomData<T>);

impl<T: PortValueBoxed + Clone> Port for OutValue<T> {
    type Type = T;

    fn name() -> &'static str {
        "out"
    }
}

#[derive(Clone, Copy, Default, PartialEq, Sequence)]
enum Operator {
    #[default]
    Add,
    Sub,
    Mul,
    Div,
}

impl Operator {
    pub fn as_str(&self) -> &str {
        match self {
            Operator::Add => "Add ➕",
            Operator::Sub => "Subtract ➖",
            Operator::Mul => "Multiply ✖",
            Operator::Div => "Divide ➗",
        }
    }
}

pub struct Operation<T> {
    operator: Operator,
    phantom: PhantomData<T>,
}

impl<T> Default for Operation<T> {
    fn default() -> Self {
        Self {
            operator: Operator::default(),
            phantom: PhantomData,
        }
    }
}

impl<T> Module for Operation<T>
where
    T: PortValueBoxed + Clone,
    InValueA<T>: Input,
    InValueB<T>: Input,
    <InValueA<T> as Port>::Type: Add<<InValueB<T> as Port>::Type, Output = T>,
    <InValueA<T> as Port>::Type: Sub<<InValueB<T> as Port>::Type, Output = T>,
    <InValueA<T> as Port>::Type: Mul<<InValueB<T> as Port>::Type, Output = T>,
    <InValueA<T> as Port>::Type: Div<<InValueB<T> as Port>::Type, Output = T>,
{
    fn describe() -> ModuleDescription<Self> {
        ModuleDescription::default()
            .name(&format!("➕✖Operation<{}>", T::name()))
            .port(PortDescription::<InValueA<T>>::input())
            .port(PortDescription::<InValueB<T>>::input())
            .port(PortDescription::<OutValue<T>>::output())
    }

    fn show(&mut self, ctx: &ShowContext, ui: &mut Ui) {
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_source(ctx.instance)
                .selected_text(format!("{:?}", self.operator.as_str()))
                .show_ui(ui, |ui| {
                    for operator in Operator::iter() {
                        ui.selectable_value(&mut self.operator, operator, operator.as_str());
                    }
                });
        });
    }

    fn process(&mut self, ctx: &mut ProcessContext) {
        let a = ctx.get_input::<InValueA<T>>();
        let b = ctx.get_input::<InValueB<T>>();

        ctx.set_output::<OutValue<T>>(match self.operator {
            Operator::Add => a + b,
            Operator::Sub => a - b,
            Operator::Mul => a * b,
            Operator::Div => a / b,
        })
    }
}
