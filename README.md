# example-rust-operator
Kubernetes operator example in Rust with kube-rs.

Creates and deletes deployment with desired number of replicas in the crd spec. No full reconcile in this example, because  there is no update behavior implemented!
