use std::ops::{Add, Mul};

/// A low-pass smoothing filter.
///
/// This filter is used to smooth out a signal by computing a weighted average using
/// the current value and the previous value.
///
/// The filter is defined by the following formula:
/// ```ignore
/// new_state = high * old + low * new
/// ```
#[derive(Default, Clone, Copy, Debug)]
pub struct LowPassFilter<T: Default + Clone + Copy + Add<Output = T> + Mul<Output = T>> {
    pub state: T,
    params: (T, T),
}

impl<T> LowPassFilter<T>
where
    T: Default + Clone + Copy + Add<Output = T> + Mul<Output = T>,
{
    /// Create a new [`LowPassFilter`].
    ///
    /// The state in this filter will be updated according to this formula:
    /// ```ignore
    /// new_state = high * old + low * new
    /// ```
    pub fn new(initial: T, high: T, low: T) -> Self {
        LowPassFilter {
            state: initial,
            params: (high, low),
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
        let mut filter = LowPassFilter::new(0.0, 0.8, 0.2);
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
