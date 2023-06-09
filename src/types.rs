use std::marker::PhantomData;

use crate::{frame::Frame, io::Conversion, module::PortValueBoxed};

pub trait Type: Clone + 'static {
    fn define() -> TypeDefinition<Self>
    where
        Self: Sized;

    fn name() -> &'static str;
    fn to_string(&self) -> String;
    fn as_value(&self) -> f32;
}

impl<T: Type> PortValueBoxed for T {
    fn name() -> &'static str {
        T::name()
    }
    fn to_string(&self) -> String {
        self.to_string()
    }

    fn as_value(&self) -> f32 {
        self.as_value()
    }
}

pub struct TypeDefinitionDyn {
    pub conversions: Vec<Conversion>,
}

impl TypeDefinitionDyn {
    fn from_typed<T>(definition: TypeDefinition<T>) -> Self {
        Self {
            conversions: definition.conversions,
        }
    }
}

pub struct TypeDefinition<T> {
    conversions: Vec<Conversion>,
    phantom: PhantomData<T>,
}

impl<T: PortValueBoxed> TypeDefinition<T> {
    fn new() -> Self {
        Self {
            conversions: Vec::new(),
            phantom: PhantomData,
        }
    }

    fn add_conversion<I: PortValueBoxed + Clone>(
        mut self,
        closure: impl Fn(I) -> T + Clone + 'static,
    ) -> Self {
        self.conversions.push(Conversion::new_type(closure));
        self
    }

    pub fn into_dyn(self) -> TypeDefinitionDyn {
        TypeDefinitionDyn::from_typed(self)
    }
}

impl Type for f32 {
    fn name() -> &'static str {
        "f32"
    }

    fn define() -> TypeDefinition<Self>
    where
        Self: Sized,
    {
        TypeDefinition::new().add_conversion(|frame: Frame| frame.as_f32_mono())
    }

    fn to_string(&self) -> String {
        format!("{:.2}", self)
    }
    fn as_value(&self) -> f32 {
        *self
    }
}

impl Type for bool {
    fn name() -> &'static str {
        "bool"
    }

    fn define() -> TypeDefinition<Self>
    where
        Self: Sized,
    {
        TypeDefinition::new()
    }

    fn to_string(&self) -> String {
        format!("{}", self)
    }

    fn as_value(&self) -> f32 {
        if *self {
            1.0
        } else {
            0.0
        }
    }
}

impl Type for Frame {
    fn name() -> &'static str {
        "Frame"
    }

    fn define() -> TypeDefinition<Self>
    where
        Self: Sized,
    {
        TypeDefinition::new().add_conversion(|value: f32| Frame::Mono(value))
    }

    fn to_string(&self) -> String {
        match self {
            Frame::Mono(sample) => format!("Mono({})", sample),
            Frame::Stereo(a, b) => {
                format!("Stereo({},{})", a, b)
            }
        }
    }

    fn as_value(&self) -> f32 {
        self.as_f32_mono()
    }
}
