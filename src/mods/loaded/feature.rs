use anyhow::Result;
use bevy_platform::collections::HashMap;

use super::schedule::LoadedSchedules;

// These fields are read by a debug macro
#[allow(dead_code)]
#[derive(Debug)]
pub struct LoadedFeature {
    pub name: String,
    pub resources: HashMap<common::StableId, Vec<u8>>,
    pub schedules: LoadedSchedules,
}

impl LoadedFeature {
    pub fn try_from_descriptor(descriptor: &common::FeatureDescriptor) -> Result<Self> {
        let schedules = LoadedSchedules::try_from_schedule_descriptors(&descriptor.schedules)?;

        Ok(Self {
            name: descriptor.name.to_owned(),
            resources: descriptor
                .resources
                .iter()
                .map(|(id, bytes)| (id.to_owned(), bytes.to_owned()))
                .collect(),
            schedules,
        })
    }
}
