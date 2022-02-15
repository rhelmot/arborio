use lazy_static::lazy_static;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use stable_deref_trait::StableDeref;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::hash::Hash;
use std::ops::Deref;
use std::sync::Mutex;

pub fn next_uuid() -> u32 {
    let mut locked = UUID.lock().unwrap();
    let result = *locked;
    *locked += 1;
    result
}

#[derive(Hash, PartialEq, Eq, Copy, Clone, Debug, PartialOrd, Ord)]
pub struct Interned(&'static str);

pub type InternedMap<T> = HashMap<Interned, T>;

lazy_static! {
    static ref INTERNSHIP: elsa::sync::FrozenMap<&'static str, &'static str> =
        elsa::sync::FrozenMap::new();
    static ref UUID: Mutex<u32> = Mutex::new(0);
}

impl Deref for Interned {
    type Target = &'static str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
unsafe impl StableDeref for Interned {}

impl Default for Interned {
    fn default() -> Self {
        intern("")
    }
}

pub fn intern(s: &str) -> Interned {
    // not sure why this API is missing so much
    Interned(if let Some(res) = INTERNSHIP.get(s) {
        res
    } else {
        let mine = Box::leak(Box::new(s.to_owned()));
        INTERNSHIP.insert(mine, mine)
    })
}

impl Serialize for Interned {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(s)
    }
}

impl<'de> Deserialize<'de> for Interned {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let owned: String = Deserialize::deserialize(d)?;
        Ok(intern(&owned))
    }
}

impl Display for Interned {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Borrow<str> for Interned {
    fn borrow(&self) -> &str {
        self.0
    }
}
