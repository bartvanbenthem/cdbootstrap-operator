use azure_core::new_http_client;
use azure_identity::{ClientSecretCredential, TokenCredentialOptions};
use azure_security_keyvault::prelude::*;
use std::{env, process, sync::Arc};

use kube::Client;

use crate::crd::CDBootstrap;

#[derive(Debug)]
struct Azure {
    tenant: String,
    keyvault_url: String,
    spn: String,
}

impl Azure {
    #[allow(dead_code)]
    pub async fn print_secret(client: Client, name: &str, namespace: &str, cr: &CDBootstrap) {
        // mitigate warnings
        let _ = client;
        let _ = name;
        let _ = namespace;

        let config = Azure {
            tenant: cr.spec.tenant.clone(),
            keyvault_url: cr.spec.keyvault.clone(),
            spn: cr.spec.spn.clone(),
        };

        let spn_secret: String = env::var("SPN_SECRET").unwrap();
        let secret_name = "default";

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
