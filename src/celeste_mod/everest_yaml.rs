use itertools::Itertools;
use serde::de::{Error, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::path::Path;
use std::str::FromStr;
use vizia::prelude::Data;

use crate::assets::Interned;
use crate::celeste_mod::walker::{ConfigSource, ConfigSourceTrait};

pub fn celeste_module_yaml() -> EverestYaml {
    EverestYaml {
        name: "Celeste".into(),
        version: EverestModuleVersion(vec![1, 4, 0, 0]),
        dll: None,
        dependencies: vec![],
    }
}

pub fn arborio_module_yaml() -> EverestYaml {
    EverestYaml {
        name: "Arborio".into(),
        version: EverestModuleVersion(vec![0, 1, 0]),
        dll: None,
        dependencies: vec![EverestYamlDependency {
            name: "Celeste".into(),
            version: EverestModuleVersion(vec![1, 4, 0, 0]),
        }],
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EverestYaml {
    #[serde(rename = "Name")]
    pub name: Interned,
    #[serde(rename = "Version")]
    pub version: EverestModuleVersion,
    #[serde(rename = "DLL", default)]
    pub dll: Option<String>,
    #[serde(rename = "Dependencies", default)]
    pub dependencies: Vec<EverestYamlDependency>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EverestYamlDependency {
    #[serde(rename = "Name")]
    pub name: Interned,
    #[serde(rename = "Version")]
    pub version: EverestModuleVersion,
}

#[derive(PartialEq, Eq, PartialOrd, Clone, Debug, Data)]
pub struct EverestModuleVersion(pub Vec<i32>);

impl<'de> Deserialize<'de> for EverestModuleVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        s.parse()
            .map_err(|e| D::Error::invalid_value(Unexpected::Other(e), &"1.2.3"))
    }
}

impl Serialize for EverestModuleVersion {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(s)
    }
}

impl ToString for EverestModuleVersion {
    fn to_string(&self) -> String {
        self.0.iter().map(|x| x.to_string()).join(".")
    }
}

impl FromStr for EverestModuleVersion {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.split('.')
            .map(|x| x.parse())
            .collect::<Result<Vec<i32>, _>>()
            .map_err(|_| "unable to parse integer")
            .map(EverestModuleVersion)
    }
}

impl EverestYaml {
    pub fn from_config(source: &mut ConfigSource) -> Result<Self, String> {
        if let Some(mut reader) = source.get_file(Path::new("everest.yaml")) {
            let mut data = String::new();
            reader.read_to_string(&mut data).unwrap();
            let everest_yaml: Vec<EverestYaml> =
                match serde_yaml::from_str(data.trim_start_matches('\u{FEFF}')) {
                    Ok(e) => e,
                    Err(e) => {
                        return Err(format!(
                            "Error parsing {}/everest.yaml: {:?}",
                            source
                                .filesystem_root()
                                .unwrap()
                                .to_str()
                                .unwrap_or("<invalid unicode>"),
                            e
                        ));
                    }
                };
            if everest_yaml.len() != 1 {
                return Err(format!(
                    "Error parsing {}/everest.yaml: {} entries",
                    source
                        .filesystem_root()
                        .unwrap()
                        .to_str()
                        .unwrap_or("<invalid unicode>"),
                    everest_yaml.len()
                ));
            }
            Ok(everest_yaml.into_iter().next().unwrap())
        } else {
            Err(format!(
                "No everest.yaml in {}",
                source
                    .filesystem_root()
                    .unwrap()
                    .to_str()
                    .unwrap_or("<invalid unicode>")
            ))
        }
    }

    pub fn save(&self, mod_path: &Path) {
        println!("Saving with name {}", self.name);
        [self]
            .serialize(&mut serde_yaml::Serializer::new(
                std::fs::File::create(&mod_path.join("everest.yaml")).unwrap(),
            ))
            .unwrap();
    }
}
