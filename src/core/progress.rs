//! Work counters supplied by file readers, decoders and exporters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Progress {
    pub stage: &'static str,
    pub completed: u64,
    pub total: u64,
    pub unit: &'static str,
    pub item: Option<String>,
}
impl Progress {
    pub fn new(stage: &'static str, completed: u64, total: u64, unit: &'static str) -> Self {
        Self {
            stage,
            completed,
            total,
            unit,
            item: None,
        }
    }
    pub fn with_item(mut self, item: String) -> Self {
        self.item = Some(item);
        self
    }
    pub fn fraction(&self) -> Option<f32> {
        (self.total > 0).then(|| (self.completed as f64 / self.total as f64).clamp(0.0, 1.0) as f32)
    }
}
pub type Reporter<'a> = dyn FnMut(Progress) -> Result<(), String> + 'a;
