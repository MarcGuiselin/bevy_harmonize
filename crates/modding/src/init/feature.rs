use bevy_ecs::schedule::{IntoSystemConfigs, SystemConfigs};

pub trait Feature {
    fn build(&self, feature: &mut NewFeature);
}

pub struct NewFeature {
    name: &'static str,
    resources: Vec<(&'static str, Vec<u8>)>,
    systems: Vec<(&'static str, SystemConfigs)>,
}

pub trait HasStableId {
    const STABLE_ID: &'static str;
}

pub trait ScheduleLabel: HasStableId {
    fn id(&self) -> &'static str {
        Self::STABLE_ID
    }
}

pub trait Resource: HasStableId + bitcode::Encode + Default {}

impl NewFeature {
    pub fn set_name(&mut self, name: &'static str) -> &mut Self {
        self.name = name;
        self
    }

    pub fn insert_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.resources
            .push((R::STABLE_ID, bitcode::encode(&resource)));
        self
    }

    pub fn add_systems<S: ScheduleLabel, M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self {
        self.systems.push((schedule.id(), systems.into_configs()));
        self
    }
}
