KUBECONFIG=~/.kube/k3s.yaml

kubectl create -f cdbootstraps.cnad.nl.yaml

cargo fmt
cargo run

# kubectl apply -f cdbootstrap-example.yaml