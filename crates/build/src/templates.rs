#[derive(bart_derive::BartDisplay)]
#[template = "templates/import/Cargo.toml.template"]
pub struct ImportsCargo<'a> {
    pub file_name: &'a str,
    pub modloader_version: &'a str,
    pub dev_mode: &'a str,
    pub name: &'a str,
}

#[derive(bart_derive::BartDisplay)]
#[template = "templates/import/lib.rs.template"]
pub struct ImportsLib<'a> {
    pub components: &'a [ImportsComponent<'a>],
}

pub struct ImportsComponent<'a> {
    pub name: &'a str,
    pub id: u32,
    pub address: u32,
}

#[derive(bart_derive::BartDisplay)]
#[template = "templates/source/Cargo.toml.template"]
pub struct SourceCargo<'a> {
    pub file_name: &'a str,
    pub modloader_version: &'a str,
    pub dev_mode: &'a str,
    pub name: &'a str,
    pub source_file: &'a str,
}

#[derive(bart_derive::BartDisplay)]
#[template = "templates/export/manifest/Cargo.toml.template"]
pub struct ExportsManifestCargo<'a> {
    pub file_name: &'a str,
    pub modloader_version: &'a str,
    pub dev_mode: &'a str,
    pub name: &'a str,
}

#[derive(bart_derive::BartDisplay)]
#[template = "templates/export/manifest/lib.rs.template"]
pub struct ExportsManifestLib {}

#[derive(bart_derive::BartDisplay)]
#[template = "templates/export/systems/Cargo.toml.template"]
pub struct ExportsSystemsCargo<'a> {
    pub file_name: &'a str,
    pub modloader_version: &'a str,
    pub dev_mode: &'a str,
    pub name: &'a str,
}

#[derive(bart_derive::BartDisplay)]
#[template = "templates/export/systems/lib.rs.template"]
pub struct ExportsSystemsLib<'a> {
    pub systems: &'a [ExportsSystem<'a>],
}

pub struct ExportsSystem<'a> {
    pub id: u32,
    pub name: &'a str,
}
