source ../00-ENV/env.sh

export KEYVAULT_URL='https://kmcs-p-weu-prd.vault.azure.net/'
export SPN='69f74670-5cf9-4cfe-b795-8dc3a6cc975f'
export TENANT='0baeb517-c6ec-4d6c-a394-96a5affa5ada'

cargo test -- --nocapture