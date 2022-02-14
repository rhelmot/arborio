use lazy_static::lazy_static;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use stable_deref_trait::StableDeref;
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

#[derive(Clone, Debug)]
pub struct InternedMap<T>(HashMap<&'static str, T>);

impl<T> InternedMap<T> {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get(&self, key: &str) -> Option<&T> {
        self.0.get(key)
    }

    pub fn insert(&mut self, key: Interned, value: T) -> Option<T> {
        self.0.insert(key.0, value)
    }

    pub fn iter(&self) -> impl Iterator<Item = (Interned, &T)> {
        self.0.iter().map(|(k, v)| (Interned(k), v))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Interned, &mut T)> {
        self.0.iter_mut().map(|(k, v)| (Interned(k), v))
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    pub fn retain<F: FnMut(&&'static str, &mut T) -> bool>(&mut self, filter: F) {
        self.0.retain(filter)
    }

    pub fn extend<I: IntoIterator<Item = (Interned, T)>>(&mut self, iter: I) {
        self.0.extend(iter.into_iter().map(|(k, v)| (k.0, v)))
    }
}
impl<T: Serialize> Serialize for InternedMap<T> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(s)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for InternedMap<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let owned: HashMap<String, T> = Deserialize::deserialize(d)?;
        Ok(InternedMap(
            owned.into_iter().map(|(k, v)| (intern(&k).0, v)).collect(),
        ))
    }
}

impl<T> FromIterator<(Interned, T)> for InternedMap<T> {
    fn from_iter<I: IntoIterator<Item = (Interned, T)>>(iter: I) -> Self {
        Self(HashMap::from_iter(iter.into_iter().map(|(k, v)| (k.0, v))))
    }
}

impl<T> Default for InternedMap<T> {
    fn default() -> Self {
        Self::new()
    }
}

//impl<T> FromIterator<(&str, T)> for InternedMap<T> {
//    fn from_iter<I: IntoIterator<Item=Interned>>(iter: I) -> Self {
//        Self(HashMap::from_iter(iter.into_iter().map(|(k, v)| (intern(k).0, v))))
//    }
//}
