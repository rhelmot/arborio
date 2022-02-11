use lazy_static::lazy_static;
use std::sync::Mutex;

lazy_static! {
    static ref INTERNSHIP: elsa::sync::FrozenMap<&'static str, &'static str> =
        elsa::sync::FrozenMap::new();
    static ref UUID: Mutex<u32> = Mutex::new(0);
}

pub fn next_uuid() -> u32 {
    let mut locked = UUID.lock().unwrap();
    let result = *locked;
    *locked += 1;
    result
}

pub fn intern(s: &str) -> &'static str {
    // not sure why this API is missing so much
    if let Some(res) = INTERNSHIP.get(s) {
        res
    } else {
        let mine = Box::leak(Box::new(s.to_owned()));
        INTERNSHIP.insert(mine, mine)
    }
}
