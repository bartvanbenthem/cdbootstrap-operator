KUBECONFIG=~/.kube/k3s.yaml

kubectl create -f config/crd/cdbootstraps.cndev.nl.yaml

cargo fmt
cargo run

# kubectl apply -f config/samples/cdbootstrap-example.yaml
# kubectl apply -f config/samples/update.yaml

# kubectl patch secret test-bootstrap -p '{"data":{"AZP_TOKEN":"cGFzc3dvcmQ="}}'