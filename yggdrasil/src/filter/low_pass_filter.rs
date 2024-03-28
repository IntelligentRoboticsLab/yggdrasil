use std::ops::{Add, Mul};

#[derive(Default)]
pub struct LowPassFilter<T: Default + Clone + Copy + Add<Output = T> + Mul<Output = T>> {
    pub state: T,
    params: (T, T),
}

impl<T> LowPassFilter<T>
where
    T: Default + Clone + Copy + Add<Output = T> + Mul<Output = T>,
{
    pub fn new(initial: T, params: (T, T)) -> Self {
        LowPassFilter {
            state: initial,
            params,
        }
    }
    /// Update the current state of this [`LowPassFilter`] using the new value.
    pub fn update(&mut self, value: T) {
        self.state = self.params.0 * self.state + self.params.1 * value;
    }
}

#[cfg(test)]
mod tests {
    use super::LowPassFilter;

    #[test]
    fn update() {
        let mut filter = LowPassFilter::new(0.0, (0.8, 0.2));
        assert_eq!(filter.state, 0.0);

        filter.update(0.5);
        assert_eq!(filter.state, 0.1);

        filter.update(0.5);
        filter.update(0.5);
        filter.update(0.5);
        assert_eq!(filter.state, 0.2952);

        filter.update(10.0);
        assert_eq!(filter.state, 2.23616);

        filter.update(-0.5);
        assert_eq!(filter.state, 1.688928);
    }
}
