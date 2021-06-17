use std::ops::{DerefMut, Deref};

pub struct AutoSaver<T> {
    value: T,
    saver: fn(&mut T),
}
impl <T> AutoSaver<T> {
    pub fn new(value: T, saver: fn(&mut T)) -> Self {
        Self {
            value,
            saver,
        }
    }
    pub fn borrow_mut(&mut self) -> MutRef<T> {
        MutRef { auto_saver: self }
    }
}
impl<T> Deref for AutoSaver<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

pub struct MutRef<'a, T> {
    auto_saver: &'a mut AutoSaver<T>,
}
impl <T> Drop for MutRef<'_, T> {
    fn drop(&mut self) {
        // Where the magic happens
        (self.auto_saver.saver)(&mut self.auto_saver.value);
    }
}
impl <T> Deref for MutRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.auto_saver.value
    }
}
impl <T> DerefMut for MutRef<'_, T> {

    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.auto_saver.value
    }
}
