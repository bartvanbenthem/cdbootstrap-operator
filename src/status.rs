use kube::api::{Patch, PatchParams, PostParams};
use kube::{Api, Client, Error, ResourceExt};
use serde_json::{json, Value};
use tracing::*;

use std::collections::BTreeMap;

use crate::crd::{CDBootstrap, CDBootstrapStatus};

pub async fn patch(
    client: Client,
    name: &String,
    namespace: &String,
    success: bool,
) -> Result<CDBootstrap, Error> {
    let api: Api<CDBootstrap> = Api::namespaced(client, namespace);

    let data: Value = json!({
        "status": CDBootstrapStatus { succeeded: success }
    });

    api.patch_status(name, &PatchParams::default(), &Patch::Merge(&data))
        .await
}

pub async fn print(client: Client, name: &String, namespace: &String) -> Result<(), Error> {
    let api: Api<CDBootstrap> = Api::namespaced(client, namespace);

    let cdb = api.get_metadata(name).await?;
    info!(
        "Got namespace {:?} for {}",
        &cdb.metadata.namespace.clone().unwrap(),
        &cdb.name_any()
    );

    info!("Get Status on cdbootstrap instance {}", name);
    let cdb = api.get_status(name).await?;
    info!("Got status {:?} for {}", &cdb.status, &cdb.name_any());

    Ok(())
}

////////////////////////////////////////////////////
/// TROUBLESHOOTING

#[allow(dead_code)]
pub async fn replace(
    client: Client,
    name: &String,
    namespace: &String,
    success: bool,
) -> Result<CDBootstrap, Error> {
    let api: Api<CDBootstrap> = Api::namespaced(client, namespace);

    let md = api.get(name).await?;

    let data: Value = json!({
        "apiVersion": "cnad.nl/v1beta1",
        "kind": "CDBootstrap",
        "metadata": {
            "name": name,
            // Updates need to provide our last observed version:
            "resourceVersion": md.resource_version(),
        },
        "status": CDBootstrapStatus { succeeded: success }
    });

    let mut cdb = api.get(name).await?; // retrieve partial object
    cdb.status = Some(CDBootstrapStatus::default()); // update the job part

    let result = serde_json::to_vec(&data).expect("Failed to serialize data to JSON");
    api.replace_status(name, &PostParams::default(), result)
        .await
}

#[allow(dead_code)]
pub async fn patch_spec_label_status_debug(
    client: Client,
    name: &str,
    namespace: &str,
) -> Result<CDBootstrap, kube::Error> {
    let api: Api<CDBootstrap> = Api::namespaced(client, namespace);

    let mut labels: BTreeMap<String, String> = BTreeMap::new();
    labels.insert("app".to_owned(), name.to_owned());

    let data: Value = json!({
        "metadata": {
            "labels": labels
        },
        "spec": {
            "replicas": 4
        },
        "status": {
            "succeeded": true
        }
    });

    let patch: Patch<&Value> = Patch::Merge(&data);
    api.patch(name, &PatchParams::default(), &patch).await
}

#[allow(dead_code)]
pub async fn patch_status_debug(
    client: Client,
    name: &str,
    namespace: &str,
) -> Result<CDBootstrap, kube::Error> {
    let api: Api<CDBootstrap> = Api::namespaced(client, namespace);

    // Load the existing resource
    let existing_resource = api.get(name).await?;
    println!("{:?}", existing_resource.status);

    // Create a patch for updating the status
    let status_patch = json!({
        "status": {
            "succeeded": "true"
        }
    });

    // Apply the patch to update the status
    api.patch_status(name, &PatchParams::default(), &Patch::Merge(&status_patch))
        .await
}
