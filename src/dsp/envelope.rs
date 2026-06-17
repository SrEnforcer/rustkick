#[derive(Default)]
pub struct Envelope {
    position: f32,
    active: bool,
}

impl Envelope {
    pub fn trigger(&mut self) {
        self.position = 0.0;
        self.active = true;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Advances position and returns normalized progress (0.0 = start, 1.0 = end).
    pub fn tick(&mut self, decay_samples: f32) -> f32 {
        if !self.active {
            return 1.0;
        }

        self.position += 1.0 / decay_samples;

        if self.position >= 1.0 {
            self.position = 1.0;
            self.active = false;
        }

        self.position
    }
}
