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
    pub fn frame(&mut self, input: f32) -> f32 {
        let dif = (input - self.current).clamp(-self.max_dif, self.max_dif);
        self.current += dif;
        self.current
    }
}
