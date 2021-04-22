mod map_struct;

use std::fs;
use std::error::Error;
use celeste;


fn offset_of(base: &[u8], position: &[u8]) -> isize {
    let result = position.as_ptr() as isize - base.as_ptr() as isize;
    if result < 0 || result > base.len() as isize {
        panic!("Provided position that is not part of base")
    }
    return result;
}

fn main() -> Result<(), Box<dyn Error>> {
    let path = "/home/audrey/games/celeste/Content/Maps/1-ForsakenCity.bin";
    let buf = fs::read(path)?;
    let slice = buf.as_slice();
    //let slice = include_bytes!("/home/audrey/games/celeste/Content/Maps/1-ForsakenCity.bin");
    let (_, parsed) = celeste::binel::parser::take_file(slice)
        .map_err(|e| { format!("Parse error: {}", e) })?;
    let map = map_struct::from_binfile(parsed)?;

    return Ok(());
}
