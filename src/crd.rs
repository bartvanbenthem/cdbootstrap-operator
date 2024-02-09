use garde::Validate;
use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Struct corresponding to the Specification (`spec`) part of the `CDBootstrap` resource, directly
/// reflects context of the `cdbootstraps.example.com.yaml` file to be found in this repository.
/// The `CDBootstrap` struct will be generated by the `CustomResource` derive macro.
#[derive(CustomResource, Serialize, Deserialize, Debug, Validate, Clone, JsonSchema)]
#[kube(
    group = "cndev.nl",
    version = "v1beta1",
    kind = "CDBootstrap",
    plural = "cdbootstraps",
    namespaced
)]
#[kube(status = "CDBootstrapStatus")]
pub struct CDBootstrapSpec {
    #[garde(skip)]
    pub replicas: i32,
    #[garde(skip)]
    pub url: String,
    #[garde(skip)]
    pub pool: String,
    #[garde(skip)]
    pub keyvault: String,
    #[garde(skip)]
    pub spn: String,
    #[garde(skip)]
    pub tenant: String,
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, JsonSchema)]
pub struct CDBootstrapStatus {
    pub succeeded: bool,
}
