use azure_core::new_http_client;
use azure_identity::{ClientSecretCredential, TokenCredentialOptions};
use azure_security_keyvault::SecretClient;
use cdbootstrap::vault::*;
use std::sync::Arc;
use std::{env, process};

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
                String::default()
            }
        };

        println!("\nvalue from keyvault: {}\n", value);
    }
}

#[tokio::test]
async fn print_secret_works() {
    let tenant = env::var("TENANT").unwrap_or("none".to_string());
    let keyvault_url = env::var("KEYVAULT_URL").unwrap_or("none".to_string());
    let spn = env::var("SPN").unwrap_or("none".to_string());
    let secret_name = env::var("SECRET_NAME").unwrap_or("none".to_string());

    let azure = AzureVault::new(&tenant, &keyvault_url, &spn);
    print_secret_from_vault(&azure, &secret_name).await;
}
