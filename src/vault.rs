use azure_core::new_http_client;
use azure_identity::{ClientSecretCredential, TokenCredentialOptions};
use azure_security_keyvault::prelude::*;
use futures::StreamExt;
use kube::Client;
use std::{env, process, sync::Arc};
use tracing::{error, info};

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

    // test the connection en authentication to the azure keyvault
    pub async fn test_connection(az: &AzureVault, client_secret: &String) {
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

        let secret_result = client.clone().list_secrets().into_stream().next().await;

        match secret_result.map(|result| result.map(|_| ())) {
            Some(Ok(_)) => info!("Connection to Azure KeyVault is Successfull"),
            Some(Err(error)) => eprintln!("Error connecting to Azure KeyVault: {}", error),
            None => eprintln!("Error connecting to Azure KeyVault, returned None"),
        }
    }
}

pub async fn run(client: Client, name: &str, namespace: &str, cr: &CDBootstrap) {
    let sps_result =
        AgentSecret::value_is_set(client.clone(), &name, &namespace, "SPN_SECRET").await;
    let sps = match sps_result {
        Ok(sps) => sps,
        Err(err) => {
            error!("{:?}", err);
            false
        }
    };

    let azp_result =
        AgentSecret::value_is_set(client.clone(), &name, &namespace, "AZP_TOKEN").await;
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
        info!("Testing authentication to the Vault");
        if let Ok(secret_value) =
            AgentSecret::get_value(client.clone(), &name, &namespace, "SPN_SECRET").await
        {
            let azure_vault = AzureVault::new(&cr.spec.tenant, &cr.spec.keyvault, &cr.spec.spn);
            AzureVault::test_connection(&azure_vault, &secret_value.to_string()).await;
        } else {
            // Handle the error
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

pub async fn print_secret_from_vault(az: &AzureVault, secret_name: &str) {
    let config = AzureVault {
        tenant: az.tenant.clone(),
        url: az.url.clone(),
        spn: az.spn.clone(),
    };

    let spn_secret: String = env::var("SPN_SECRET").unwrap_or("none".to_string());

    let creds = Arc::new(ClientSecretCredential::new(
        new_http_client(),
        config.tenant,
        config.spn,
        spn_secret,
        TokenCredentialOptions::default(),
    ));

    let client_result = SecretClient::new(&config.url, creds);
    let client = match client_result {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Error creating new Azure Secret CLient {}", error);
            process::exit(1)
        }
    };

    if secret_name.len() > 0 {
        let secret_result = client.clone().get(secret_name).await;

        let value = match secret_result {
            Ok(s) => s.value,
            Err(error) => {
                eprintln!("Error getting Azure Secrets from Client {}", error);
                process::exit(1);
            }
        };

        println!("\nvalue from keyvault: {}\n", value);
    }
}
