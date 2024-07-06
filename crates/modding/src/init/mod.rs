mod feature;
pub use feature::*;

/// Access to the Harmony engine in order to add new features and mod existing ones
///
/// This done within the init function.
///
/// The init function of a mod serves several purposes, including:
/// - Generating the manifest
/// - Initializing the system execution runtime
pub struct Harmony {
    features: Vec<FeatureBuilder>,
}

impl Harmony {
    pub fn add_feature<F: Feature>(&mut self, feature: F) -> &mut Self {
        let mut builder = FeatureBuilder {
            name: "Unnammed",
            descriptors: vec![],
            resources: vec![],
        };
        feature.build(&mut builder);
        self.features.push(builder);
        self
    }
}
