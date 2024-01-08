use crate::crd::CDBootstrap;
use kube::api::{Patch, PatchParams};
use kube::{Api, Client, Error};
use serde_json::{json, Value};

/// Adds a finalizer record into an `CDBootstrap` kind of resource. If the finalizer already exists,
/// this action has no effect.
///
/// # Arguments:
/// - `client` - Kubernetes client to modify the `CDBootstrap` resource with.
/// - `name` - Name of the `CDBootstrap` resource to modify. Existence is not verified
/// - `namespace` - Namespace where the `CDBootstrap` resource with given `name` resides.
///
/// Note: Does not check for resource's existence for simplicity.
pub async fn add(client: Client, name: &str, namespace: &str) -> Result<CDBootstrap, Error> {
    let api: Api<CDBootstrap> = Api::namespaced(client, namespace);
    let finalizer: Value = json!({
        "metadata": {
            "finalizers": ["cdbootstraps.cnad.nl/finalizer"]
        }
    });

    let patch: Patch<&Value> = Patch::Merge(&finalizer);
    api.patch(name, &PatchParams::default(), &patch).await
}

/// Removes all finalizers from an `CDBootstrap` resource. If there are no finalizers already, this
/// action has no effect.
///
/// # Arguments:
/// - `client` - Kubernetes client to modify the `CDBootstrap` resource with.
/// - `name` - Name of the `CDBootstrap` resource to modify. Existence is not verified
/// - `namespace` - Namespace where the `CDBootstrap` resource with given `name` resides.
///
/// Note: Does not check for resource's existence for simplicity.
pub async fn delete(client: Client, name: &str, namespace: &str) -> Result<CDBootstrap, Error> {
    let api: Api<CDBootstrap> = Api::namespaced(client, namespace);
    let finalizer: Value = json!({
        "metadata": {
            "finalizers": null
        }
    });

    let patch: Patch<&Value> = Patch::Merge(&finalizer);
    api.patch(name, &PatchParams::default(), &patch).await
}
