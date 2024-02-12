use azure_core::new_http_client;
use azure_identity::{ClientSecretCredential, TokenCredentialOptions};
use azure_security_keyvault::prelude::*;
use std::{env, process, sync::Arc};

use kube::Client;

use crate::crd::CDBootstrap;

#[derive(Debug)]
pub struct Azure {
    pub tenant: String,
    pub keyvault_url: String,
    pub spn: String,
}

impl Azure {
    #[allow(dead_code)]
    pub fn new(tenant: &str, keyvault_url: &str, spn: &str) -> Self {
        Self {
            tenant: tenant.to_string(),
            keyvault_url: keyvault_url.to_string(),
            spn: spn.to_string(),
        }
    }

    #[allow(dead_code, unused_variables)]
    pub async fn get_secret(client: Client, name: &str, namespace: &str, cr: &CDBootstrap) {}

    #[allow(dead_code)]
    pub async fn print_secret(az: &Azure, secret_name: &str) {
        let config = Azure {
            tenant: az.tenant.clone(),
            keyvault_url: az.keyvault_url.clone(),
            spn: az.spn.clone(),
        };

        let spn_secret: String = env::var("SPN_SECRET").unwrap();

        let creds = Arc::new(ClientSecretCredential::new(
            new_http_client(),
            config.tenant,
            config.spn,
            spn_secret,
            TokenCredentialOptions::default(),
        ));

        let client_result = SecretClient::new(&config.keyvault_url, creds);
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

            println!("{:?}", value);
        }
    }
}
