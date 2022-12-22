use std::sync::atomic::{AtomicU32, Ordering};

pub fn next_uuid() -> u32 {
    static UUID: AtomicU32 = AtomicU32::new(1);
    UUID.fetch_add(1, Ordering::Relaxed)
}

#[macro_export]
macro_rules! uuid_cls {
    ($name:ident) => {
        #[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
        pub struct $name(u32);
        impl $name {
            pub fn new() -> Self {
                Self($crate::uuid::next_uuid())
            }

            pub fn null() -> Self {
                return Self(0);
            }
        }
    };
}
