use std::marker::PhantomData;

use eframe::egui::{self, Ui};

use crate::{
    module::{Module, ModuleDescription, Port, PortDescription, PortValueBoxed},
    rack::rack::{ProcessContext, ShowContext},
};

pub struct ValueOutput<T>(PhantomData<T>);

impl<T: PortValueBoxed + Clone> Port for ValueOutput<T> {
    type Type = T;

    fn name() -> &'static str {
        "output"
    }
}

pub struct Value<T> {
    value: T,
    phantom: PhantomData<T>,
}

impl<T: Default> Value<T> {
    pub fn new() -> Self {
        Self {
            value: T::default(),
            phantom: PhantomData,
        }
    }
}

pub trait Edit {
    fn edit(&mut self, ui: &mut Ui);
}

impl Edit for f32 {
    fn edit(&mut self, ui: &mut Ui) {
        ui.add(
            egui::DragValue::new(self)
                .clamp_range(0.0..=f32::MAX)
                .speed(1.0),
        );
    }
}

impl<T: Edit + PortValueBoxed + Clone + Default> Module for Value<T> {
    fn describe() -> ModuleDescription<Self>
    where
        Self: Sized,
    {
        ModuleDescription::new(|| Value::<T>::new())
            .name(&format!("⎙ Value<{}>", T::name()))
            .port(PortDescription::<ValueOutput<T>>::output())
    }

    fn process(&mut self, ctx: &mut ProcessContext) {
        ctx.set_output::<ValueOutput<T>>(self.value.clone())
    }

    fn show(&mut self, _: &ShowContext, ui: &mut Ui) {
        self.value.edit(ui)
    }
}
