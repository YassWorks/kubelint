//! `KL0xxx`: the file cannot be read as YAML.

mod duplicate_key;
mod invalid_yaml;

pub use duplicate_key::DuplicateKey;
pub use invalid_yaml::InvalidYaml;
