#[derive(Debug, Default, Clone, Copy)]
pub struct AnonymousVarCounter(pub usize);

impl AnonymousVarCounter {
    pub fn next(self) -> Self {
        Self(self.0 + 1)
    }
}
