use azure_core::new_http_client;
use azure_identity::{ClientSecretCredential, TokenCredentialOptions};
use azure_security_keyvault::SecretClient;
use cdbootstrap::vault::*;
use std::sync::Arc;
use std::{env, process};

pub async fn print_secret_from_vault(az: &AzureVault, namespace: &str) {
    let config = AzureVault {
        oid: az.oid.clone(),
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

    let key = format!("{}-{}", az.oid, namespace);
    if namespace.len() > 0 {
        let secret_result = client.clone().get(&key).await;

        let value = match secret_result {
            Ok(s) => s.value,
            Err(error) => {
                eprintln!("Error getting Azure Secrets from Client {}", error);
                String::default()
            }
        };

        println!(
            "\nvalue from KeyVault key={} value={}...\n",
            &key,
            &value[0..5]
        );
    }
}

#[tokio::test]
async fn print_secret_works() {
    let oid = env::var("OID").unwrap_or("none".to_string());
    let tenant = env::var("TENANT").unwrap_or("none".to_string());
    let keyvault_url = env::var("KEYVAULT_URL").unwrap_or("none".to_string());
    let spn = env::var("SPN").unwrap_or("none".to_string());
    let namespace = env::var("NAMESPACE").unwrap_or("none".to_string());

    let azure = AzureVault::new(&oid, &tenant, &keyvault_url, &spn);
    print_secret_from_vault(&azure, &namespace).await;
}
