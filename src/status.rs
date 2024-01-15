use anyhow::Result;
use kube::api::{Patch, PatchParams, PostParams};
use kube::{Api, Client, Error, ResourceExt};
use serde_json::{json, Value};
use tracing::*;

use std::collections::BTreeMap;

use crate::crd::{CDBootstrap, CDBootstrapStatus};

pub async fn patch(client: Client, name: &String, success: bool) -> Result<CDBootstrap, Error> {
    let api: Api<CDBootstrap> = Api::default_namespaced(client);

    let data = json!({
        "status": CDBootstrapStatus { succeeded: success }
    });

    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("{:?}", &data);
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");

    let pp = PatchParams::default(); // json merge patch
    api.patch_status(name, &pp, &Patch::Merge(&data)).await
}

pub async fn get(client: Client, name: &String) -> Result<(), Error> {
    let api: Api<CDBootstrap> = Api::default_namespaced(client);

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

#[allow(dead_code)]
pub async fn replace(client: Client, name: &String, success: bool) -> Result<CDBootstrap, Error> {
    let api: Api<CDBootstrap> = Api::default_namespaced(client);

    let md = api.get(name).await?;

    let data = json!({
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
    let pp = PostParams::default();

    let result = serde_json::to_vec(&data).expect("Failed to serialize data to JSON");
    api.replace_status(name, &pp, result).await
}

#[allow(dead_code)]
pub async fn patch_spec_test(
    client: Client,
    name: &str,
    namespace: &str,
) -> Result<CDBootstrap, kube::Error> {
    let api: Api<CDBootstrap> = Api::namespaced(client, namespace);

    let mut labels: BTreeMap<String, String> = BTreeMap::new();
    labels.insert("app".to_owned(), name.to_owned());

    let replicas: Value = json!({
        "metadata": {
            "labels": labels
        },
        "spec": {
            "replicas": 4
        }
    });

    let patch: Patch<&Value> = Patch::Merge(&replicas);
    api.patch(name, &PatchParams::default(), &patch).await
}
