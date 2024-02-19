use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::core::v1::{
    ConfigMap, Container, ContainerPort, PodSpec, PodTemplateSpec, Secret,
};
use k8s_openapi::api::networking::v1::NetworkPolicy;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use kube::api::{DeleteParams, ObjectMeta, Patch, PatchParams, PostParams};
use kube::{Api, Client, Error, ResourceExt};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::str::from_utf8;
use tracing::*;

use crate::crd::CDBootstrap;

pub struct Agent {}

impl Agent {
    /// Deploys a new or updates an existing deployment of `n` pods with the `nginx:latest`,
    /// where `n` is the number of `replicas` given.
    ///
    /// # Arguments
    /// - `client` - A Kubernetes client to create/update the Deployment with.
    /// - `name` - Name of the Deployment to be created/updated
    /// - `replicas` - Number of pod replicas for the Deployment to contain
    /// - `namespace` - Namespace to create/update the Kubernetes Deployment in.
    pub async fn apply(
        client: Client,
        name: &str,
        namespace: &str,
        cr: &CDBootstrap,
    ) -> Result<Deployment, Error> {
        // check for existing Deployment
        let api: Api<Deployment> = Api::namespaced(client.clone(), namespace);

        if let Ok(_) = api.get(name).await {
            info!("Deployment {} found in namespace {}", name, namespace);
            info!(
                "Update Deployment {} in namespace {} to desired state",
                name, namespace
            );
            api.replace(
                name,
                &PostParams::default(),
                &Agent::new(name, namespace, cr),
            )
            .await
        } else {
            info!("Deployment {} not found in namespace {}", name, namespace);
            info!("Creating Deployment {} in namespace {}", name, namespace);
            api.create(&PostParams::default(), &Agent::new(name, namespace, cr))
                .await
        }
    }

    fn new(name: &str, namespace: &str, cr: &CDBootstrap) -> Deployment {
        let labels: BTreeMap<String, String> = [("app".to_owned(), cr.name_any().to_owned())]
            .iter()
            .cloned()
            .collect();

        let image = String::from("ghcr.io/bartvanbenthem/azp-agent-alpine:latest");

        // Define the NetworkPolicy configuration as JSON
        let deployment_json: Value = json!({
            "apiVersion": "apps/v1",
            "kind": "Deployment",
            "metadata": {
                "name": name,
                "namespace": namespace,
                "labels": labels
            },
            "spec": {
                "replicas": cr.spec.replicas,
                "selector": {
                    "matchLabels": {
                        "app": "example"
                    }
                },
                "template": {
                    "metadata": {
                        "labels": {
                            "app": "example"
                        }
                    },
                    "spec": {
                        "containers": [
                            {
                                "name": name,
                                "image": image.clone(),
                                "env": [
                                    {
                                        "name": "AZP_TOKEN",
                                        "valueFrom": {
                                            "secretKeyRef": {
                                                "name": name,
                                                "key": "AZP_TOKEN",
                                                "optional": true,
                                            },
                                        },
                                    },
                                    {
                                        "name": "SPN_SECRET",
                                        "valueFrom": {
                                            "secretKeyRef": {
                                                "name": name,
                                                "key": "SPN_SECRET",
                                                "optional": true,
                                            },
                                        },
                                    },
                                    {
                                        "name": "AZP_URL",
                                        "valueFrom": {
                                            "configMapKeyRef": {
                                                "name": name,
                                                "key": "AZP_URL",
                                                "optional": true,
                                            },
                                        },
                                    },
                                    {
                                        "name": "AZP_POOL",
                                        "valueFrom": {
                                            "configMapKeyRef": {
                                                "name": name,
                                                "key": "AZP_POOL",
                                                "optional": true,
                                            },
                                        },
                                    },
                                ]
                            }
                        ]
                    }
                }
            }
        });

        // Convert the JSON to Deployment struct using serde
        let deployment_result: Result<Deployment, serde_json::Error> =
            serde_json::from_value(deployment_json);
        let deployment = match deployment_result {
            Ok(deployment) => deployment,
            Err(err) => {
                error!(
                    "Error creating Deployment {} applying default",
                    kube::Error::SerdeError(err)
                );
                let default_deployment: Deployment = Default::default();
                return default_deployment;
            }
        };
        deployment
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
                info!("Not able to find the existing {} deployment", name);
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

pub struct AgentConfig {}

impl AgentConfig {
    pub async fn apply(
        client: Client,
        name: &str,
        namespace: &str,
        cr: &CDBootstrap,
    ) -> Result<ConfigMap, Error> {
        // check for existing ConfigMap
        let api: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);

        if let Ok(_) = api.get(&name).await {
            info!("ConfigMap {} found in namespace {}", name, namespace);
            info!(
                "Update ConfigMap {} in namespace {} to desired state",
                name, namespace
            );
            api.replace(
                name,
                &PostParams::default(),
                &AgentConfig::new(name, namespace, cr),
            )
            .await
        } else {
            info!("ConfigMap {} not found in namespace {}", name, namespace);
            info!("Creating ConfigMap {} in namespace {}", name, namespace);
            api.create(
                &PostParams::default(),
                &AgentConfig::new(name, namespace, cr),
            )
            .await
        }
    }

    fn new(name: &str, namespace: &str, cr: &CDBootstrap) -> ConfigMap {
        let labels: BTreeMap<String, String> = [("app".to_owned(), cr.name_any().to_owned())]
            .iter()
            .cloned()
            .collect();

        let url = cr.spec.url.clone();
        let pool = cr.spec.pool.clone();

        // Define the NetworkPolicy configuration as JSON
        let configmap_json: Value = json!({
               "apiVersion": "v1",
               "kind": "ConfigMap",
               "metadata": {
                "name": name,
                "namespace": namespace,
                "labels": labels,
               },
                "data": {
                  "AZP_POOL": pool,
                  "AZP_URL": url,
                  //"AZP_WORK": "placeholder",
                  //"AZP_AGENT_NAME": "placeholder",
                  //"AGENT_MTU_VALUE": "placeholder"
                }

        });

        // Convert the JSON to NetworkPolicy struct using serde
        let configmap_result: Result<ConfigMap, serde_json::Error> =
            serde_json::from_value(configmap_json);
        let configmap = match configmap_result {
            Ok(configmap) => configmap,
            Err(err) => {
                error!(
                    "Error creating ConfigMap {} applying default",
                    kube::Error::SerdeError(err)
                );
                let default_configmap: ConfigMap = Default::default();
                return default_configmap;
            }
        };
        configmap
    }

    /// Deletes an existing ConfigMap.
    ///
    /// # Arguments:
    /// - `client` - A Kubernetes client to delete the ConfigMap with
    /// - `name` - Name of the deployment to delete
    /// - `namespace` - Namespace the existing ConfigMap resides in
    ///
    /// Note: It is assumed the deployment exists for simplicity. Otherwise returns an Error.
    pub async fn delete(client: Client, name: &str, namespace: &str) -> Result<(), Error> {
        let api: Api<ConfigMap> = Api::namespaced(client, namespace);
        api.delete(&name, &DeleteParams::default()).await?;
        Ok(())
    }
}

pub struct AgentSecret {}

impl AgentSecret {
    pub async fn apply(
        client: Client,
        name: &str,
        namespace: &str,
        cr: &CDBootstrap,
    ) -> Result<Secret, Error> {
        // check for existing Secret
        let api: Api<Secret> = Api::namespaced(client.clone(), namespace);

        if let Ok(_) = api.get(name).await {
            info!("Secret {} found in namespace {}", name, namespace);
            info!(
                "Update Secret {} in namespace {} to desired state",
                name, namespace
            );
            api.replace(
                name,
                &PostParams::default(),
                &AgentSecret::new(name, namespace, cr),
            )
            .await
        } else {
            info!("Secret {} not found in namespace {}", name, namespace);
            info!("Creating Secret {} in namespace {}", name, namespace);
            api.create(
                &PostParams::default(),
                &AgentSecret::new(name, namespace, cr),
            )
            .await
        }
    }

    fn new(name: &str, namespace: &str, cr: &CDBootstrap) -> Secret {
        let labels: BTreeMap<String, String> = [("app".to_owned(), cr.name_any().to_owned())]
            .iter()
            .cloned()
            .collect();

        // Define the NetworkPolicy configuration as JSON
        let secret_json: Value = json!({
               "apiVersion": "v1",
               "kind": "Secret",
               "metadata": {
                "name": name,
                "namespace": namespace,
                "labels": labels,
               },
                "data": {
                  "AZP_TOKEN": null,
                  "SPN_SECRET": null,
                }

        });

        // Convert the JSON to NetworkPolicy struct using serde
        let secret_result: Result<Secret, serde_json::Error> = serde_json::from_value(secret_json);
        let secret = match secret_result {
            Ok(secret) => secret,
            Err(err) => {
                error!(
                    "Error creating Secret {} applying default",
                    kube::Error::SerdeError(err)
                );
                let default_secret: Secret = Default::default();
                return default_secret;
            }
        };
        secret
    }

    /// Deletes an existing Secret.
    ///
    /// # Arguments:
    /// - `client` - A Kubernetes client to delete the Secret with
    /// - `name` - Name of the deployment to delete
    /// - `namespace` - Namespace the existing Secret resides in
    ///
    /// Note: It is assumed the deployment exists for simplicity. Otherwise returns an Error.
    pub async fn delete(client: Client, name: &str, namespace: &str) -> Result<(), Error> {
        let api: Api<Secret> = Api::namespaced(client, namespace);
        api.delete(&name, &DeleteParams::default()).await?;
        Ok(())
    }

    pub async fn value_is_set(
        client: Client,
        name: &str,
        namespace: &str,
        key: &str,
    ) -> Result<bool, Error> {
        let mut is_set = false;
        let api: Api<Secret> = Api::namespaced(client.clone(), namespace);

        match api.get(name).await {
            Ok(secret) => {
                if let Some(data) = secret.data {
                    if let Some(value) = data.get(key) {
                        let token_decoded_result = from_utf8(&value.0);
                        match token_decoded_result {
                            Ok(t) => is_set = !t.is_empty(),
                            Err(_) => {
                                error!(
                                    "Error Getting value from {} in namespace {}",
                                    key, namespace
                                );
                            }
                        }
                    }
                }
            }
            Err(_) => {
                error!("Secret {} in namespace {} NOT found", name, namespace);
            }
        }

        Ok(is_set)
    }

    pub async fn get_value(
        client: Client,
        name: &str,
        namespace: &str,
        key: &str,
    ) -> Result<String, Error> {
        let mut client_secret = String::new();

        let api: Api<Secret> = Api::namespaced(client.clone(), namespace);

        match api.get(name).await {
            Ok(secret) => {
                if let Some(data) = secret.data {
                    if let Some(value) = data.get(key) {
                        let token_decoded_result = from_utf8(&value.0);
                        match token_decoded_result {
                            Ok(t) => client_secret = t.to_string(),
                            Err(_) => {
                                error!(
                                    "Error Getting value from {} in namespace {}",
                                    key, namespace
                                );
                            }
                        }
                    }
                }
            }
            Err(_) => {
                error!("Error getting Secret {} in namespace {}", name, namespace);
            }
        }

        Ok(client_secret)
    }

    #[allow(dead_code, unused_variables)]
    pub async fn set_azp_token(
        client: Client,
        name: &str,
        namespace: &str,
        value: &str,
    ) -> Result<(), Error> {
        // Retrieve the existing Secret
        let api: Api<Secret> = Api::namespaced(client.clone(), namespace);

        let existing_secret = match api.get(name).await {
            Ok(secret) => secret,
            Err(_) => {
                error!("Error getting Secret {} in namespace {}", name, namespace);
                Secret::default()
            }
        };

        let mut client_secret = String::default();
        if let Some(data) = existing_secret.data {
            if let Some(value) = data.get("SPN_SECRET") {
                let token_decoded_result = from_utf8(&value.0);
                match token_decoded_result {
                    Ok(t) => client_secret = t.to_string(),
                    Err(_) => {
                        error!("Error getting SPN_SECRET value in namespace {}", namespace);
                    }
                }
            }
        }

        // Create a BTreeMap<String, String>
        let mut data_patch: BTreeMap<String, String> = BTreeMap::new();

        // Add key-value pairs to the BTreeMap
        data_patch.insert("AZP_TOKEN".to_string(), value.to_string());
        data_patch.insert("SPN_SECRET".to_string(), client_secret);

        let result = api
            .patch(
                &name,
                &PatchParams::apply("cdbootstrap-operator"),
                &Patch::Apply(Secret {
                    metadata: ObjectMeta {
                        name: Some(name.to_owned()),
                        namespace: Some(namespace.to_owned()),
                        ..ObjectMeta::default()
                    },
                    string_data: Some(data_patch.clone()),
                    ..Secret::default()
                }),
            )
            .await?;

        Ok(())
    }
}

pub struct AgentPolicy {}

impl AgentPolicy {
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
                &AgentPolicy::new(&precise_name, namespace, cr),
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
                &AgentPolicy::new(&precise_name, namespace, cr),
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

////////////////////////////////////////////////////
/// NOT USED

#[allow(dead_code)]
pub async fn apply_old(
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
