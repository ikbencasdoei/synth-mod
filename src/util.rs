use enum_iterator::{All, Sequence};

pub trait EnumIter: Sized + Sequence {
    fn iter() -> All<Self> {
        enum_iterator::all()
    }
}

impl<T: Sized + Sequence> EnumIter for T {}
