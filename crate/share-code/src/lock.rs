use std::sync::{Arc, Mutex, MutexGuard};

pub struct ArcMutex<T> {
    arc_t: Arc<Mutex<T>>,
}
impl<T> ArcMutex<T> {
    pub fn new(t: T) -> Self {
        Self {
            arc_t: Arc::new(Mutex::new(t)),
        }
    }
    pub fn lock(&self) -> MutexGuard<T> {
        self.arc_t.lock().unwrap()
    }
}
impl<T> Clone for ArcMutex<T> {
    fn clone(&self) -> Self {
        Self {
            arc_t: self.arc_t.clone(),
        }
    }
}
