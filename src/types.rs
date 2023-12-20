use crate::{frame::Frame, module::PortValueBoxed};

/// Trait all inter-module data types must implement.
pub trait Type: Clone + 'static {
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

impl Type for f32 {
    fn name() -> &'static str {
        "f32"
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
