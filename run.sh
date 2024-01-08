KUBECONFIG=~/.kube/k3s.yaml

kubectl apply -f cdbootstraps.cnad.nl.yaml

cargo run

# kubectl apply -f cdbootstrap-example.yaml
# kubectl delete -f cdbootstrap-example.yaml
# kubectl delete -f cdbootstraps.cnad.nl.yaml