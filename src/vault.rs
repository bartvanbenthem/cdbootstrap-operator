use azure_core::new_http_client;
use azure_identity::{ClientSecretCredential, TokenCredentialOptions};
use azure_security_keyvault::prelude::*;
use k8s_openapi::api::core::v1::Secret;
use kube::{Api, Client};
use std::str::from_utf8;
use std::{env, process, sync::Arc};
use tracing::{error, info};

#[derive(Debug)]
pub struct AzureVault {
    pub tenant: String,
    pub url: String,
    pub spn: String,
}

#[allow(dead_code)]
impl AzureVault {
    pub fn new(tenant: &str, keyvault_url: &str, spn: &str) -> Self {
        Self {
            tenant: tenant.to_string(),
            url: keyvault_url.to_string(),
            spn: spn.to_string(),
        }
    }

    // test the connection en authentication to the azure keyvault
    pub async fn test_connection() {}

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
}

pub async fn run(client: Client, name: &str, namespace: &str) {
    let sps = secret_value_is_set(client.clone(), &name, &namespace, "SPN_SECRET").await;
    let azp = secret_value_is_set(client.clone(), &name, &namespace, "AZP_TOKEN").await;
    if azp == false && sps == false {
        info!("Make sure to inject the AZP_TOKEN, or set the SPN_SECRET to collect a Token from the Vault");
    }

    if sps == true && azp == false {
        info!("Testing authentication to the Vault");
        AzureVault::test_connection().await;
    }

    if azp == true {
        info!("AZP_TOKEN Has been set, check the Agent logs for polling state");
    }
}

async fn secret_value_is_set(client: Client, name: &str, namespace: &str, key: &str) -> bool {
    let mut is_set = false;

    let api: Api<Secret> = Api::namespaced(client.clone(), namespace);
    if let Ok(secret) = api.get(name).await {
        let data = secret.data.unwrap();
        if let Some(value) = data.get(key) {
            let token_decoded = from_utf8(&value.0).unwrap_or("unable to decode Secret value");
            //println!("Found secret data !!!!!!!!! {:?}",token_decoded.replace("\n", ""));
            if token_decoded == "" {
                info!(
                    "{} Secret {} in namespace {} has NOT been set or collected",
                    key, name, namespace
                );
            } else {
                info!(
                    "{} Secret {} in namespace {} has a value",
                    key, name, namespace
                );
                is_set = true;
            }
        }
        is_set
    } else {
        error!("Secret {} in namespace {} not found", name, namespace);
        is_set
    }
}
