use arborio_maploader::from_binel::{bin_el_fuzzy_equal, TryFromBinEl};
use arborio_maploader::map_struct::CelesteMap;
use arborio_modloader::discovery;
use arborio_state::data::AppConfig;
use arborio_walker::{ConfigSourceTrait, FolderSource};
use celeste::binel::BinEl;
use std::ffi::OsStr;
use std::path::Path;

#[test]
fn test_saving_all_mods() {
    let cfg: AppConfig = confy::load("arborio").unwrap_or_default();
    if let Some(root) = &cfg.celeste_root {
        println!("Beginning test.");
        assert!(root.is_dir(), "Arborio is misconfigured");
        let mut config = FolderSource::new(&root.join("Content")).unwrap();
        for path in config.list_all_files(Path::new("Maps")) {
            println!("testing Celeste {path:?}");

            let mut reader = config.get_file(&path).unwrap();
            let mut file = vec![];
            reader.read_to_end(&mut file).unwrap();
            let (_, binfile) = celeste::binel::parser::take_file(file.as_slice()).unwrap();

            test_saving_one_mod(&binfile.root);
        }
        discovery::for_each_mod(root, |_, _, name, mut config| {
            for path in config.list_all_files(Path::new("Maps")) {
                if path.extension() == Some(OsStr::new("bin")) {
                    let brokens = [
                        Path::new("Maps/KaydenFox/FactoryMod/1-Factory.bin"),
                        Path::new("Maps/SpringCollab2020/4-Expert/Mun.bin"),
                    ];
                    if brokens.contains(&path.as_path()) {
                        println!("Skipping {name} {path:?}");
                        continue;
                    }
                    println!("testing {name} {path:?}");

                    let mut reader = config.get_file(&path).unwrap();
                    let mut file = vec![];
                    reader.read_to_end(&mut file).unwrap();
                    let (_, binfile) = celeste::binel::parser::take_file(file.as_slice()).unwrap();

                    test_saving_one_mod(&binfile.root);
                }
            }
        });
    } else {
        println!("TODO: bundle celeste skeleton for tests")
    }
}

fn test_saving_one_mod(bin: &BinEl) {
    let structured = CelesteMap::try_from_bin_el(bin).unwrap();
    let saved = structured.to_binel();
    assert!(bin_el_fuzzy_equal("", bin, &saved));
}
