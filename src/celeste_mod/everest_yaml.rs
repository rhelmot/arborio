use crate::assets::{intern, Interned};
use itertools::Itertools;
use serde::de::{Error, Unexpected};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

pub fn celeste_module_yaml() -> EverestYaml {
    EverestYaml {
        name: intern("Celeste"),
        version: EverestModuleVersion(vec![1, 4, 0, 0]),
        dll: None,
        dependencies: vec![],
    }
}

pub fn arborio_module_yaml() -> EverestYaml {
    EverestYaml {
        name: intern("Arborio"),
        version: EverestModuleVersion(vec![0, 1, 0]),
        dll: None,
        dependencies: vec![EverestYamlDependency {
            name: intern("Celeste"),
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

#[derive(PartialEq, Eq, PartialOrd, Clone, Debug)]
pub struct EverestModuleVersion(pub Vec<i32>);

impl<'de> Deserialize<'de> for EverestModuleVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: String = Deserialize::deserialize(deserializer)?;
        s.split('.')
            .map(|x| x.parse())
            .collect::<Result<Vec<i32>, _>>()
            .map_err(|_| {
                D::Error::invalid_value(Unexpected::Other("unable to parse integer"), &"1.2.3")
            })
            .map(EverestModuleVersion)
    }
}

impl Serialize for EverestModuleVersion {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.iter().map(|x| x.to_string()).join(".").serialize(s)
    }
}
