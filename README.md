# cdbootstrap operator

Seamless integration and automation between datacenter sites are paramount. To streamline this process, we introduce the Kubernetes CDBootstrap Operator, designed specifically for initializing pipeline agents and establishing a robust connection with an external customer DevOps environment, eliminating the need for ingress requirements. The operator facilitates secure communication between the pipeline agent on Kubernetes and the external DevOps environment, employing industry best practices for encryption, authentication, and authorization.

![Alt Text](hack/cdbootstrap-operator.PNG)


```bash
# Create CDBootstrap CRD
kubectl create -f config/crd/cdbootstraps.cndev.nl.yaml
```

```bash
# Run the Operator
KUBECONFIG=~/.kube/k3s.yaml
cargo fmt
cargo run
```

```bash
# apply CDBootstrap sample
kubectl apply -f config/samples/cdbootstrap-example.yaml
```

```bash
# Inject Token in Agent secret
export EPAT=$(echo "<pat_token>" | base64)
kubectl patch secret test-bootstrap -p '{"data":{"AZP_TOKEN": "'"$EPAT"'"}}'
# restart pods
kubectl scale deploy test-bootstrap --replicas=0 && kubectl scale deploy test-bootstrap --replicas=2
```