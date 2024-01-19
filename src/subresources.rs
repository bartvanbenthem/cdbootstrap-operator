use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{Container, ContainerPort, PodSpec, PodTemplateSpec};
use k8s_openapi::api::networking::v1::NetworkPolicy;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use kube::api::{DeleteParams, ObjectMeta, PostParams};
use kube::{Api, Client, Error, ResourceExt};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use tracing::*;

use crate::crd::CDBootstrap;

pub struct Agent {}

impl Agent {
    /// Deploys a new or updates an existing deployment of `n` pods with the `nginx:latest`,
    /// where `n` is the number of `replicas` given.
    ///
    /// # Arguments
    /// - `client` - A Kubernetes client to create/update the deployment with.
    /// - `name` - Name of the deployment to be created/updated
    /// - `replicas` - Number of pod replicas for the Deployment to contain
    /// - `namespace` - Namespace to create/update the Kubernetes Deployment in.
    pub async fn apply(
        client: Client,
        name: &str,
        namespace: &str,
        cr: &CDBootstrap,
    ) -> Result<Deployment, Error> {
        let image = String::from("ghcr.io/bartvanbenthem/azp-agent-alpine:latest");

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
                                    image: Some(image.to_owned()),
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
                info!(
                    "Deployment {:?} in namespace {} does not exisist, creating new deployment",
                    &name, &namespace
                );
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
                                    image: Some(image.to_owned()),
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

    pub async fn desired_state(
        client: Client,
        cr: &CDBootstrap,
        name: &str,
        namespace: &str,
    ) -> Result<bool, Error> {
        // Fetch the existing deployment
        let deployment_api: Api<Deployment> = Api::namespaced(client.clone(), namespace);
        let existing_deployment_result = deployment_api.get(name).await;

        let existing_deployment = match existing_deployment_result {
            Ok(existing_deployment) => existing_deployment,
            Err(_) => {
                // Handle the case when the deployment is not found
                info!("Not able to find the existing {} deployment", &name);
                return Ok(false);
            }
        };

        let current_replicas = existing_deployment
            .spec
            .and_then(|spec| spec.replicas)
            .unwrap_or(1);

        if current_replicas == cr.spec.replicas {
            return Ok(true);
        } else {
            return Ok(false);
        }
    }
}

pub struct Policy {}

impl Policy {
    pub async fn apply(
        client: Client,
        name: &str,
        namespace: &str,
        cr: &CDBootstrap,
    ) -> Result<NetworkPolicy, Error> {
        // check for existing networkpolicy
        let api: Api<NetworkPolicy> = Api::namespaced(client.clone(), namespace);

        let precise_name = String::from("allow-egress-".to_owned() + name);

        if let Ok(_) = api.get(&precise_name).await {
            info!("NetworkPolicy {} found in namespace {}", name, namespace);
            info!(
                "Update NetworkPolicy {} in namespace {} to desired state",
                name, namespace
            );
            api.replace(
                &precise_name,
                &PostParams::default(),
                &Policy::new(&precise_name, namespace, cr),
            )
            .await
        } else {
            info!(
                "NetworkPolicy {} not found in namespace {}",
                name, namespace
            );
            info!("Creating NetworkPolicy {} in namespace {}", name, namespace);
            api.create(
                &PostParams::default(),
                &Policy::new(&precise_name, namespace, cr),
            )
            .await
        }
    }

    fn new(name: &str, namespace: &str, cr: &CDBootstrap) -> NetworkPolicy {
        let labels: BTreeMap<String, String> = [("app".to_owned(), cr.name_any().to_owned())]
            .iter()
            .cloned()
            .collect();

        // Define the NetworkPolicy configuration as JSON
        let network_policy_json: Value = json!({
            "apiVersion": "networking.k8s.io/v1",
            "kind": "NetworkPolicy",
            "metadata": {
                "name": name,
                "namespace": namespace,
                "labels": labels
            },
            "spec": {
                "podSelector": {
                    "matchLabels": {
                        "app": cr.name_any().to_owned(),
                        // Add other labels as needed
                    }
                },
                "egress": [
                    {
                        "to": [
                            {
                                "ports": [
                                    {
                                        "port": 443,
                                        "protocol": "TCP"
                                    },
                                    {
                                        "port": 443,
                                        "protocol": "UDP"
                                    }
                                ]
                            }
                        ],
                        "to": [
                            {
                                "ipBlock": {
                                    "cidr": "13.107.6.0/24"
                                }
                            },
                            {
                                "ipBlock": {
                                    "cidr": "13.107.9.0/24"
                                }
                            },
                            {
                                "ipBlock": {
                                    "cidr": "13.107.42.0/24"
                                }
                            },
                            {
                                "ipBlock": {
                                    "cidr": "13.107.43.0/24"
                                }
                            }
                        ]
                    }
                ],
                "policyTypes": ["Egress"]
            }
        });

        // Convert the JSON to NetworkPolicy struct using serde
        let network_policy_result: Result<NetworkPolicy, serde_json::Error> =
            serde_json::from_value(network_policy_json);
        let network_policy = match network_policy_result {
            Ok(network_policy) => network_policy,
            Err(err) => {
                error!(
                    "Error creating network policy {} applying default",
                    kube::Error::SerdeError(err)
                );
                let default_network_policy: NetworkPolicy = Default::default();
                return default_network_policy;
            }
        };
        network_policy
    }

    /// Deletes an existing NetworkPolicy.
    ///
    /// # Arguments:
    /// - `client` - A Kubernetes client to delete the NetworkPolicy with
    /// - `name` - Name of the deployment to delete
    /// - `namespace` - Namespace the existing NetworkPolicy resides in
    ///
    /// Note: It is assumed the deployment exists for simplicity. Otherwise returns an Error.
    pub async fn delete(client: Client, name: &str, namespace: &str) -> Result<(), Error> {
        let precise_name = String::from("allow-egress-".to_owned() + name);
        let api: Api<NetworkPolicy> = Api::namespaced(client, namespace);
        api.delete(&precise_name, &DeleteParams::default()).await?;
        Ok(())
    }
}
