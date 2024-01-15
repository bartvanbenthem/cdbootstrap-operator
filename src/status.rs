use anyhow::Result;
use kube::api::{Patch, PatchParams, PostParams};
use kube::ResourceExt;
use kube::{client::Client, Api, Error};
use serde_json::{json, Value};
use tracing::*;

use crate::crd::{CDBootstrap, CDBootstrapStatus};

pub async fn patch_test(
    client: Client,
    name: &str,
    namespace: &str,
    success: bool,
) -> Result<CDBootstrap, kube::Error> {
    let api: Api<CDBootstrap> = Api::namespaced(client, namespace);
    let status: Value = json!({
        "status": CDBootstrapStatus { succeeded: success }
    });

    let patch: Patch<&Value> = Patch::Merge(&status);
    api.patch(name, &PatchParams::default(), &patch).await
}

pub async fn patch(client: Client, name: &String, success: bool) -> Result<(), Error> {
    let api: Api<CDBootstrap> = Api::default_namespaced(client);

    let data = json!({
        "status": CDBootstrapStatus { succeeded: success }
    });

    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");
    println!("{:?}", data);
    println!("!!!!!!!!!!!!!!!!!!!!!!!!!!!!!");

    let pp = PatchParams::default(); // json merge patch
    let cdb = api.patch_status(name, &pp, &Patch::Merge(data)).await?;
    info!("Patched status {:?} for {}", cdb.status, cdb.name_any());

    //assert_eq!(cdb.status.expect("NO STATUS FOUND").succeeded, true);

    Ok(())
}

pub async fn get(client: Client, name: &String) -> Result<(), Error> {
    let api: Api<CDBootstrap> = Api::default_namespaced(client);
    info!("Get Status on cdbootstrap instance {}", name);

    let cdb = api.get_status(name).await?;
    info!("Got status {:?} for {}", &cdb.status, &cdb.name_any());

    let cdb = api.get_metadata(name).await?;
    info!(
        "Got namespace {:?} for {}",
        &cdb.metadata.namespace.clone().unwrap(),
        &cdb.name_any()
    );

    Ok(())
}

#[allow(dead_code)]
pub async fn replace(client: Client, name: &String, success: bool) -> Result<(), Error> {
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
    let cdb = api.replace_status(name, &pp, result).await?;

    info!("Replaced status {:?} for {}", cdb.status, cdb.name_any());

    Ok(())
}
