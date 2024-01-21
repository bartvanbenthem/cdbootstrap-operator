KUBECONFIG=~/.kube/k3s.yaml

kubectl create -f config/crd/cdbootstraps.cndev.nl.yaml

cargo fmt
cargo run

# kubectl apply -f config/samples/cdbootstrap-example.yaml
# kubectl apply -f config/samples/update.yaml

# source ../00-ENV/env.sh
# export EPAT=$(echo $PAT | base64)
# kubectl patch secret test-bootstrap -p '{"data":{"AZP_TOKEN": "'"$EPAT"'"}}'
# kubectl scale deploy test-bootstrap --replicas=0 && kubectl scale deploy test-bootstrap --replicas=2