pub struct LinearDamper<T> {
    max_dif: T,
    current: T,
}

impl<T> LinearDamper<T> {
    pub fn new(max_dif: T, start: T) -> Self {
        Self {
            max_dif,
            current: start,
        }
    }
}

impl LinearDamper<f32> {
    ///Creates a damper that can be used to stop some kind of wave on the basis that humans can't hear waves under 20Hz
    pub fn new_cutoff(sample_rate: u32) -> Self {
        Self::new(1.0 / (sample_rate as f32 / 20.0), 0.0)
    }

    pub fn frame(&mut self, input: f32) -> f32 {
        let dif = (input - self.current).clamp(-self.max_dif, self.max_dif);
        self.current += dif;
        self.current
    }
}
