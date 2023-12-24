use eframe::epaint::Hsva;
use enum_iterator::{All, Sequence};
use rand::Rng;

pub trait EnumIter: Sized + Sequence {
    fn iter() -> All<Self> {
        enum_iterator::all()
    }
}

impl<T: Sized + Sequence> EnumIter for T {}

/// Generates a random color that should be readable on a dark background.
pub fn random_color() -> Hsva {
    Hsva::new(
        rand::random(),
        rand::thread_rng().gen_range(0.5..=1.0),
        rand::thread_rng().gen_range(0.3..=1.0),
        1.0,
    )
}
