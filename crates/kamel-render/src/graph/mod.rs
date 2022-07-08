pub struct RecordedPass {
    pub name: String,
    pub index: usize
}

impl RecordedPass {
    fn new(name: impl Into<String>, index: usize) -> Self {
        Self {
            name: name.into(), index
        }
    }
}

pub struct RenderGraph {
    passes: Vec<RecordedPass>
}
