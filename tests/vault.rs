use cdbootstrap::vault::*;
use std::env;

#[tokio::test]
async fn print_secret_works() {
    let tenant = env::var("TENANT").unwrap();
    let keyvault_url = env::var("KEYVAULT_URL").unwrap();
    let spn = env::var("SPN").unwrap();

    let secret_name = "default";

    let azure = Azure::new(&tenant, &keyvault_url, &spn);

    Azure::print_secret(&azure, secret_name).await;
}
