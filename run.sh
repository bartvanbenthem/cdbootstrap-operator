KUBECONFIG=~/.kube/k3s.yaml

kubectl create -f config/crd/cdbootstraps.wasm.cloud.yaml

cargo fmt
cargo run

# kubectl apply -f config/samples/cdbootstrap-example.yaml
# kubectl apply -f config/samples/update.yaml