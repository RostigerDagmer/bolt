

pub struct Renderer {
    // ...
    pub dummy: u32,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            dummy: 0,
        }
    }
}