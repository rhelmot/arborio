pub mod aggregate;
pub mod config;
pub mod discovery;
pub mod everest_yaml;
pub mod mapstruct_plus_config;
pub mod module;
pub mod selectable;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
