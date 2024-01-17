use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{Container, ContainerPort, PodSpec, PodTemplateSpec};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use kube::api::{DeleteParams, ObjectMeta, PostParams};
use kube::{Api, Client, Error};
use tracing::info;
use std::collections::BTreeMap;

use crate::crd::CDBootstrap;

/// Creates a new deployment of `n` pods with the `nginx:latest`,
/// where `n` is the number of `replicas` given.
///
/// # Arguments
/// - `client` - A Kubernetes client to create the deployment with.
/// - `name` - Name of the deployment to be created
/// - `replicas` - Number of pod replicas for the Deployment to contain
/// - `namespace` - Namespace to create the Kubernetes Deployment in.
pub async fn deploy(
    client: Client,
    name: &str,
    namespace: &str,
    cr: &CDBootstrap,
) -> Result<Deployment, Error> {
    let mut labels: BTreeMap<String, String> = BTreeMap::new();
    labels.insert("app".to_owned(), name.to_owned());

    // Definition of the deployment. Alternatively, a YAML representation could be used as well.
    let deployment: Deployment = Deployment {
        metadata: ObjectMeta {
            name: Some(name.to_owned()),
            namespace: Some(namespace.to_owned()),
            labels: Some(labels.clone()),
            ..ObjectMeta::default()
        },
        spec: Some(DeploymentSpec {
            replicas: Some(cr.spec.replicas),
            selector: LabelSelector {
                match_expressions: None,
                match_labels: Some(labels.clone()),
            },
            template: PodTemplateSpec {
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: name.to_owned(),
                        image: Some("nginx:latest".to_owned()),
                        ports: Some(vec![ContainerPort {
                            container_port: 8080,
                            ..ContainerPort::default()
                        }]),
                        ..Container::default()
                    }],
                    ..PodSpec::default()
                }),
                metadata: Some(ObjectMeta {
                    labels: Some(labels),
                    ..ObjectMeta::default()
                }),
            },
            ..DeploymentSpec::default()
        }),
        ..Deployment::default()
    };

    // Create the deployment defined above
    let deployment_api: Api<Deployment> = Api::namespaced(client, namespace);
    deployment_api
        .create(&PostParams::default(), &deployment)
        .await
}

/// Deletes an existing deployment.
///
/// # Arguments:
/// - `client` - A Kubernetes client to delete the Deployment with
/// - `name` - Name of the deployment to delete
/// - `namespace` - Namespace the existing deployment resides in
///
/// Note: It is assumed the deployment exists for simplicity. Otherwise returns an Error.
pub async fn delete(client: Client, name: &str, namespace: &str) -> Result<(), Error> {
    let api: Api<Deployment> = Api::namespaced(client, namespace);
    api.delete(name, &DeleteParams::default()).await?;
    Ok(())
}

pub async fn update(
    client: Client,
    name: &str,
    namespace: &str,
    cr: &CDBootstrap,
) -> Result<Deployment, Error> {
    let mut labels: BTreeMap<String, String> = BTreeMap::new();
    labels.insert("app".to_owned(), name.to_owned());

    // Fetch the existing deployment
    let deployment_api: Api<Deployment> = Api::namespaced(client.clone(), namespace);
    let existing_deployment = deployment_api.get(name).await;

    // Handle the case where the deployment doesn't exist yet
    if let Ok(existing_deployment) = existing_deployment {
        // Update the existing deployment
        let updated_deployment: Deployment = Deployment {
            metadata: ObjectMeta {
                name: Some(name.to_owned()),
                namespace: Some(namespace.to_owned()),
                labels: Some(labels.clone()),
                ..ObjectMeta::default()
            },
            spec: Some(DeploymentSpec {
                replicas: Some(cr.spec.replicas),
                selector: LabelSelector {
                    match_expressions: None,
                    match_labels: Some(labels.clone()),
                },
                template: PodTemplateSpec {
                    spec: Some(PodSpec {
                        containers: vec![Container {
                            name: name.to_owned(),
                            image: Some("nginx:latest".to_owned()),
                            ports: Some(vec![ContainerPort {
                                container_port: 8080,
                                ..ContainerPort::default()
                            }]),
                            ..Container::default()
                        }],
                        ..PodSpec::default()
                    }),
                    metadata: Some(ObjectMeta {
                        labels: Some(labels),
                        ..ObjectMeta::default()
                    }),
                },
                ..DeploymentSpec::default()
            }),
            ..existing_deployment
        };

        // Update the deployment
        deployment_api
            .replace(name, &PostParams::default(), &updated_deployment)
            .await
    } else {
        // Handle the case where the deployment doesn't exist yet
        deploy(client, name, namespace, cr).await
    }
}

/// Deploys a new or updates an existing deployment of `n` pods with the `nginx:latest`,
/// where `n` is the number of `replicas` given.
///
/// # Arguments
/// - `client` - A Kubernetes client to create/update the deployment with.
/// - `name` - Name of the deployment to be created/updated
/// - `replicas` - Number of pod replicas for the Deployment to contain
/// - `namespace` - Namespace to create/update the Kubernetes Deployment in.
pub async fn deploy_or_update(
    client: Client,
    name: &str,
    namespace: &str,
    cr: &CDBootstrap,
) -> Result<Deployment, Error> {
    let mut labels: BTreeMap<String, String> = BTreeMap::new();
    labels.insert("app".to_owned(), name.to_owned());

    // Fetch the existing deployment
    let deployment_api: Api<Deployment> = Api::namespaced(client.clone(), namespace);
    let existing_deployment = deployment_api.get(name).await;

    // Create or update the deployment
    match existing_deployment {
        Ok(existing) => {
            // Update the existing deployment
            let updated_deployment: Deployment = Deployment {
                metadata: ObjectMeta {
                    name: Some(name.to_owned()),
                    namespace: Some(namespace.to_owned()),
                    labels: Some(labels.clone()),
                    ..ObjectMeta::default()
                },
                spec: Some(DeploymentSpec {
                    replicas: Some(cr.spec.replicas),
                    selector: LabelSelector {
                        match_expressions: None,
                        match_labels: Some(labels.clone()),
                    },
                    template: PodTemplateSpec {
                        spec: Some(PodSpec {
                            containers: vec![Container {
                                name: name.to_owned(),
                                image: Some("nginx:latest".to_owned()),
                                ports: Some(vec![ContainerPort {
                                    container_port: 8080,
                                    ..ContainerPort::default()
                                }]),
                                ..Container::default()
                            }],
                            ..PodSpec::default()
                        }),
                        metadata: Some(ObjectMeta {
                            labels: Some(labels),
                            ..ObjectMeta::default()
                        }),
                    },
                    ..DeploymentSpec::default()
                }),
                ..existing
            };

            // Update the deployment
            deployment_api
                .replace(name, &PostParams::default(), &updated_deployment)
                .await
        }
        Err(_) => {
            // Create a new deployment
            info!("The Deployment for {:?} does not exisist creating new deployment", &name);
            let mut labels: BTreeMap<String, String> = BTreeMap::new();
    labels.insert("app".to_owned(), name.to_owned());

    // Definition of the deployment. Alternatively, a YAML representation could be used as well.
    let deployment: Deployment = Deployment {
        metadata: ObjectMeta {
            name: Some(name.to_owned()),
            namespace: Some(namespace.to_owned()),
            labels: Some(labels.clone()),
            ..ObjectMeta::default()
        },
        spec: Some(DeploymentSpec {
            replicas: Some(cr.spec.replicas),
            selector: LabelSelector {
                match_expressions: None,
                match_labels: Some(labels.clone()),
            },
            template: PodTemplateSpec {
                spec: Some(PodSpec {
                    containers: vec![Container {
                        name: name.to_owned(),
                        image: Some("nginx:latest".to_owned()),
                        ports: Some(vec![ContainerPort {
                            container_port: 8080,
                            ..ContainerPort::default()
                        }]),
                        ..Container::default()
                    }],
                    ..PodSpec::default()
                }),
                metadata: Some(ObjectMeta {
                    labels: Some(labels),
                    ..ObjectMeta::default()
                }),
            },
            ..DeploymentSpec::default()
        }),
        ..Deployment::default()
    };

    // Create the deployment defined above
    let deployment_api: Api<Deployment> = Api::namespaced(client, namespace);
    deployment_api
        .create(&PostParams::default(), &deployment)
        .await
        }
    }
}
