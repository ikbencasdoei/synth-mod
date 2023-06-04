use std::ops::{Add, AddAssign, Mul};

#[derive(Clone, Copy, Debug)]
pub enum Frame {
    Mono(f32),
    #[allow(unused)]
    Stereo(f32, f32),
}

impl Default for Frame {
    fn default() -> Self {
        Self::Mono(0.0)
    }
}

impl Frame {
    pub const ZERO: Frame = Frame::Mono(0.0);

    #[allow(unused)]
    pub fn as_f32_mono(self) -> f32 {
        match self {
            Frame::Mono(sample) => sample,
            Frame::Stereo(a, b) => (a + b) / 2.0,
        }
    }

    pub fn as_f32_tuple(self) -> (f32, f32) {
        match self {
            Frame::Mono(sample) => (sample, sample),
            Frame::Stereo(a, b) => (a, b),
        }
    }
}

impl Mul<f32> for Frame {
    type Output = Frame;

    fn mul(self, rhs: f32) -> Self::Output {
        match self {
            Frame::Mono(a) => Frame::Mono(a * rhs),
            Frame::Stereo(a, b) => Frame::Stereo(a * rhs, b * rhs),
        }
    }
}

impl Add for Frame {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match self {
            Frame::Mono(frame) => match rhs {
                Frame::Mono(other) => Frame::Mono(frame + other),
                Frame::Stereo(a, b) => Frame::Stereo(a + frame, b + frame),
            },
            Frame::Stereo(a, b) => match rhs {
                Frame::Mono(other) => Frame::Stereo(a + other, b + other),
                Frame::Stereo(other_a, other_b) => Frame::Stereo(other_a + a, other_b + b),
            },
        }
    }
}

impl AddAssign for Frame {
    fn add_assign(&mut self, rhs: Self) {
        *self = Self::add(*self, rhs)
    }
}
