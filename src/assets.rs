use lazy_static::lazy_static;
use parking_lot::Mutex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::{Borrow, Cow};
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::ops::Deref;
use std::sync::atomic::{AtomicU32, Ordering};
use vizia::prelude::*;

pub fn next_uuid() -> u32 {
    static UUID: AtomicU32 = AtomicU32::new(1);
    UUID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Hash, PartialEq, Eq, Copy, Clone, Debug, PartialOrd, Ord, Data)]
pub struct Interned(&'static str);

pub type InternedMap<T> = HashMap<Interned, T>;

lazy_static! {
    static ref INTERNSHIP: Mutex<HashSet<&'static str>> = Default::default();
}

impl Deref for Interned {
    type Target = &'static str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
//unsafe impl StableDeref for Interned {}

impl Default for Interned {
    fn default() -> Self {
        "".into()
    }
}

fn intern_impl<S: Borrow<str>>(s: S, f: impl FnOnce(S) -> &'static str) -> Interned {
    let mut locked = INTERNSHIP.lock();
    Interned(if let Some(res) = locked.get(s.borrow()) {
        res
    } else {
        let s = f(s);
        locked.insert(s);
        s
    })
}

pub fn intern_static(s: &'static str) -> Interned {
    intern_impl(s, |s| s)
}

impl From<&'static str> for Interned {
    fn from(s: &'static str) -> Self {
        intern_static(s)
    }
}
impl From<String> for Interned {
    fn from(s: String) -> Self {
        intern_owned(s)
    }
}

pub fn intern_str(s: &str) -> Interned {
    intern_impl(s, |s| Box::leak(s.to_owned().into()))
}

pub fn intern_owned(s: impl Into<Box<str>>) -> Interned {
    let s = s.into();
    #[allow(clippy::redundant_closure)] // false positive
    intern_impl(s, |s| Box::leak(s))
}

impl Serialize for Interned {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(s)
    }
}

impl<'de> Deserialize<'de> for Interned {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let cow: Cow<str> = Deserialize::deserialize(d)?;

        Ok(match cow {
            Cow::Borrowed(s) => intern_str(s),
            Cow::Owned(s) => intern_owned(s),
        })
    }
}

impl Display for Interned {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Borrow<str> for Interned {
    fn borrow(&self) -> &'static str {
        self.0
    }
}

#[macro_export]
macro_rules! uuid_cls {
    ($name:ident) => {
        use vizia::prelude::Data;
        #[derive(PartialEq, Eq, Hash, Debug, Copy, Clone, Data)]
        pub struct $name(u32);
        #[allow(unused)]
        impl $name {
            pub fn new() -> Self {
                Self($crate::assets::next_uuid())
            }

            pub fn null() -> Self {
                return Self(0);
            }
        }
    };
}
