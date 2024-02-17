use anyhow::Error;
use azure_core::new_http_client;
use azure_identity::{ClientSecretCredential, TokenCredentialOptions};
use azure_security_keyvault::prelude::*;
use futures::StreamExt;
use kube::Client;
use std::{process, sync::Arc};
use tracing::{error, info, warn};

use crate::crd::CDBootstrap;
use crate::subresources::AgentSecret;

#[derive(Debug)]
pub struct AzureVault {
    pub tenant: String,
    pub url: String,
    pub spn: String,
}

impl AzureVault {
    pub fn new(tenant: &str, keyvault_url: &str, spn: &str) -> Self {
        Self {
            tenant: tenant.to_string(),
            url: keyvault_url.to_string(),
            spn: spn.to_string(),
        }
    }

    pub async fn new_client(
        az: &AzureVault,
        client_secret: &String,
    ) -> Result<SecretClient, Error> {
        let creds = Arc::new(ClientSecretCredential::new(
            new_http_client(),
            az.tenant.clone(),
            az.spn.clone(),
            client_secret.clone(),
            TokenCredentialOptions::default(),
        ));

        let client_result = SecretClient::new(&az.url, creds);
        let client = match client_result {
            Ok(client) => client,
            Err(error) => {
                eprintln!("Error creating new Azure Secret CLient {}", error);
                process::exit(1)
            }
        };

        Ok(client)
    }

    // test the connection en authentication to the azure keyvault
    pub async fn test_connection(az: &AzureVault, client_secret: &String) -> Result<bool, Error> {
        let client = AzureVault::new_client(az, client_secret).await?;
        client
            .clone()
            .list_secrets()
            .into_stream()
            .next()
            .await
            .unwrap()?;
        Ok(true)
    }

    pub async fn get_value(
        az: &AzureVault,
        client_secret: &String,
        namespace: &str,
    ) -> Result<String, Error> {
        let client = AzureVault::new_client(az, client_secret).await?;
        let secret_response = client.clone().get(namespace).await?;
        Ok(secret_response.value)
    }
}

pub async fn run(client: Client, name: &str, namespace: &str, cr: &CDBootstrap) {
    let sps_result = AgentSecret::value_is_set(client.clone(), name, namespace, "SPN_SECRET").await;
    let sps = match sps_result {
        Ok(sps) => sps,
        Err(err) => {
            error!("{:?}", err);
            false
        }
    };

    let azp_result = AgentSecret::value_is_set(client.clone(), name, namespace, "AZP_TOKEN").await;
    let azp = match azp_result {
        Ok(azp) => azp,
        Err(err) => {
            error!("{:?}", err);
            false
        }
    };

    if azp == false && sps == false {
        info!("Make sure to inject the AZP_TOKEN in Namespace {}, or set the SPN_SECRET to collect a Token from the Vault",
        namespace);
    }

    if sps == true && azp == false {
        info!("SPN_SECRET value in Namespace {} Has been set", namespace);
        if let Ok(secret_value) =
            AgentSecret::get_value(client.clone(), name, namespace, "SPN_SECRET").await
        {
            info!("Testing authentication to the Vault");
            let azure_vault = AzureVault::new(&cr.spec.tenant, &cr.spec.keyvault, &cr.spec.spn);
            let connection_result =
                AzureVault::test_connection(&azure_vault, &secret_value.to_string()).await;
            match connection_result {
                Ok(true) => {
                    info!("Connection to the Azure KeyVault is successful");
                    let vault_secret_result =
                        AzureVault::get_value(&azure_vault, &secret_value.to_string(), namespace)
                            .await;
                    info!("AZP_TOKEN Collected from the Keyvault for Namespace {}", namespace);
                    let vault_secret = match vault_secret_result {
                        Ok(s) => s,
                        Err(error) => {
                            warn!(
                                "Connection to the Azure KeyVault is unsuccessful: {:?}",
                                error
                            );
                            String::default()
                        }
                    };

                    let _ =
                        AgentSecret::set_azp_token(client, name, namespace, &vault_secret).await;
                    info!("AZP_TOKEN Secret value Set in Namespace {}", namespace);
                }
                Ok(false) => {
                    warn!("Connection to the Azure KeyVault is unsuccessful");
                }
                Err(err) => {
                    warn!("Connection to the Azure KeyVault is unsuccessful: {}", err);
                }
            }
        } else {
            error!(
                "Error retrieving SPN_SECRET value in Namespace {}",
                namespace
            );
        }
    }

    if azp == true {
        info!("AZP_TOKEN value in Namespace {} has been SET", namespace);
        info!("Check the Pod logs to see if the Agent is polling")
    }
}
