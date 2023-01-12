use arborio_utils::vizia::prelude::Data;
use itertools::Itertools;
use serde::de::{Error, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};
use std::io::Read;
use std::path::Path;
use std::str::FromStr;

use arborio_walker::{ConfigSource, ConfigSourceTrait, EmbeddedSource};

pub fn celeste_module_yaml() -> EverestYaml {
    EverestYaml {
        name: "Celeste".to_string(),
        version: EverestModuleVersion(vec![1, 4, 0, 0]),
        dll: None,
        dependencies: vec![],
    }
}

pub fn arborio_module_yaml() -> EverestYaml {
    EverestYaml::from_config(&mut EmbeddedSource().into()).unwrap()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EverestYaml {
    #[serde(rename = "Name")]
    pub name: String,
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
    pub name: String,
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
            .map_err(|e| Error::invalid_value(Unexpected::Other(e), &"1.2.3"))
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

#[derive(Debug)]
pub enum EverestYamlLoadError {
    ParseError(serde_yaml::Error),
    NotOneEntry(usize),
    Missing,
}

impl Display for EverestYamlLoadError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            EverestYamlLoadError::ParseError(e) => e.fmt(f),
            EverestYamlLoadError::NotOneEntry(n) => write!(f, "found array of {n}, expected 1"),
            EverestYamlLoadError::Missing => write!(f, "No such file"),
        }
    }
}

impl EverestYaml {
    pub fn from_config(source: &mut ConfigSource) -> Result<Self, EverestYamlLoadError> {
        for filename in ["everest.yaml", "everest.yml"].into_iter() {
            if let Some(mut reader) = source.get_file(Path::new(filename)) {
                return Self::from_reader(&mut reader);
            }
        }
        Err(EverestYamlLoadError::Missing)
    }

    pub fn from_reader<R: Read>(reader: &mut R) -> Result<Self, EverestYamlLoadError> {
        let mut data = String::new();
        reader.read_to_string(&mut data).unwrap();
        let everest_yaml: Vec<EverestYaml> =
            match serde_yaml::from_str(data.trim_start_matches('\u{FEFF}')) {
                Ok(e) => e,
                Err(e) => {
                    return Err(EverestYamlLoadError::ParseError(e));
                }
            };
        if everest_yaml.len() != 1 {
            return Err(EverestYamlLoadError::NotOneEntry(everest_yaml.len()));
        }
        Ok(everest_yaml.into_iter().next().unwrap())
    }

    pub fn save(&self, mod_path: &Path) {
        [self]
            .serialize(&mut serde_yaml::Serializer::new(
                std::fs::File::create(mod_path.join("everest.yaml")).unwrap(),
            ))
            .unwrap();
    }
}
