source ../00-ENV/env.sh
export EPAT=$(echo $PAT | base64)
kubectl patch secret test-bootstrap -p '{"data":{"AZP_TOKEN": "'"$EPAT"'"}}'
export SPAT=$(echo $SPN_SECRET | base64)
kubectl patch secret test-bootstrap -p '{"data":{"SPN_SECRET": "'"$SPAT"'"}}'
kubectl scale deploy test-bootstrap --replicas=0 && kubectl scale deploy test-bootstrap --replicas=2