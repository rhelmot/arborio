#![allow(clippy::uninlined_format_args)] // my editor can't handle refactoring these yet
pub mod atlas_img;
pub mod autotiler;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
