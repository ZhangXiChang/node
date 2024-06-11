use eyre::Result;

pub struct System {}
impl System {
    pub fn new() -> Result<Self> {
        Ok(Self {})
    }
}
impl Drop for System {
    fn drop(&mut self) {}
}
