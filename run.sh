KUBECONFIG=~/.kube/k3s.yaml

kubectl create -f config/crd/cdbootstraps.cndev.nl.yaml

cargo fmt

# test
./tests/vault.sh

cargo run

# kubectl apply -f config/samples/cdbootstrap-example.yaml && ./inject_secrets.sh

# kubectl apply -f config/samples/update.yaml