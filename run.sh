KUBECONFIG=~/.kube/k3s.yaml

kubectl create -f config/crd/cdbootstraps.cnad.nl.yaml

cargo fmt
cargo run

# kubectl apply -f config/samples/cdbootstrap-example.yaml