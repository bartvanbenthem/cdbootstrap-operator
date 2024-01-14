use std::sync::Arc;

use futures::stream::StreamExt;
use kube::api::{Patch, PatchParams};
use kube::runtime::watcher::Config;
use kube::{client::Client, runtime::controller::Action, runtime::Controller, Api};
use kube::{Resource, ResourceExt};
use serde_json::json;
use tokio::time::Duration;

use crate::crd::{CDBootstrap, CDBootstrapStatus};

mod cdbootstrap;
pub mod crd;
mod finalizer;

#[tokio::main]
async fn main() {
    // First, a Kubernetes client must be obtained using the `kube` crate
    // The client will later be moved to the custom controller
    let kubernetes_client: Client = Client::try_default()
        .await
        .expect("Expected a valid KUBECONFIG environment variable.");

    // Preparation of resources used by the `kube_runtime::Controller`
    let crd_api: Api<CDBootstrap> = Api::all(kubernetes_client.clone());
    let context: Arc<ContextData> = Arc::new(ContextData::new(kubernetes_client.clone()));

    // The controller comes from the `kube_runtime` crate and manages the reconciliation process.
    // It requires the following information:
    // - `kube::Api<T>` this controller "owns". In this case, `T = CDBootstrap`, as this controller owns the `CDBootstrap` resource,
    // - `kube::runtime::watcher::Config` can be adjusted for precise filtering of `CDBootstrap` resources before the actual reconciliation, e.g. by label,
    // - `reconcile` function with reconciliation logic to be called each time a resource of `CDBootstrap` kind is created/updated/deleted,
    // - `on_error` function to call whenever reconciliation fails.
    Controller::new(crd_api.clone(), Config::default())
        .run(reconcile, on_error, context)
        .for_each(|reconciliation_result| async move {
            match reconciliation_result {
                Ok(custom_resource) => {
                    println!("Reconciliation successful. Resource: {:?}", custom_resource);
                }
                Err(reconciliation_err) => {
                    eprintln!("Reconciliation error: {:?}", reconciliation_err)
                }
            }
        })
        .await;
}

/// Context injected with each `reconcile` and `on_error` method invocation.
struct ContextData {
    /// Kubernetes client to make Kubernetes API requests with. Required for K8S resource management.
    client: Client,
}

impl ContextData {
    /// Constructs a new instance of ContextData.
    ///
    /// # Arguments:
    /// - `client`: A Kubernetes client to make Kubernetes REST API requests with. Resources
    /// will be created and deleted with this client.
    pub fn new(client: Client) -> Self {
        ContextData { client }
    }
}

/// Action to be taken upon an `CDBootstrap` resource during reconciliation
enum CDBootstrapAction {
    /// Create the subresources, this includes spawning `n` pods with CDBootstrap service
    Create,
    /// Delete all subresources created in the `Create` phase
    Delete,
    /// This `CDBootstrap` resource is in desired state and requires no actions to be taken
    NoOp,
}

async fn reconcile(cr: Arc<CDBootstrap>, context: Arc<ContextData>) -> Result<Action, Error> {
    let client: Client = context.client.clone(); // The `Client` is shared -> a clone from the reference is obtained

    // The resource of `CDBootstrap` kind is required to have a namespace set. However, it is not guaranteed
    // the resource will have a `namespace` set. Therefore, the `namespace` field on object's metadata
    // is optional and Rust forces the programmer to check for it's existence first.
    let namespace: String = match cr.namespace() {
        None => {
            // If there is no namespace to deploy to defined, reconciliation ends with an error immediately.
            return Err(Error::UserInputError(
                "Expected CDBootstrap resource to be namespaced. Can't deploy to an unknown namespace."
                    .to_owned(),
            ));
        }
        // If namespace is known, proceed. In a more advanced version of the operator, perhaps
        // the namespace could be checked for existence first.
        Some(namespace) => namespace,
    };

    let name = cr.name_any(); // Name of the CDBootstrap resource is used to name the subresources as well.

    // Performs action as decided by the `determine_action` function.
    return match determine_action(&cr) {
        CDBootstrapAction::Create => {
            // Creates a deployment with `n` CDBootstrap service pods, but applies a finalizer first.
            // Finalizer is applied first, as the operator might be shut down and restarted
            // at any time, leaving subresources in intermediate state. This prevents leaks on
            // the `CDBootstrap` resource deletion.

            // Apply the finalizer first. If that fails, the `?` operator invokes automatic conversion
            // of `kube::Error` to the `Error` defined in this crate.
            finalizer::add(client.clone(), &name, &namespace).await?;
            // Invoke creation of a Kubernetes built-in resource named deployment with `n` CDBootstrap service pods.
            cdbootstrap::deploy(client, &name, &namespace, &cr).await?;
            Ok(Action::requeue(Duration::from_secs(10)))
        }
        CDBootstrapAction::Delete => {
            // Deletes any subresources related to this `CDBootstrap` resources. If and only if all subresources
            // are deleted, the finalizer is removed and Kubernetes is free to remove the `CDBootstrap` resource.

            //First, delete the deployment. If there is any error deleting the deployment, it is
            // automatically converted into `Error` defined in this crate and the reconciliation is ended
            // with that error.
            // Note: A more advanced implementation would check for the Deployment's existence.
            cdbootstrap::delete(client.clone(), &name, &namespace).await?;
            // Once the deployment is successfully removed, remove the finalizer to make it possible
            // for Kubernetes to delete the `CDBootstrap` resource.
            finalizer::delete(client, &name, &namespace).await?;
            Ok(Action::await_change()) // Makes no sense to delete after a successful delete, as the resource is gone
        }
        // The resource is already in desired state, do nothing and re-check after 10 seconds
        CDBootstrapAction::NoOp => {
            update_status(context.clone(), true).await?;

            ////////////////////////////////////////////////////////////////
            get_status(context.clone()).await?;
            ////////////////////////////////////////////////////////////////

            Ok(Action::requeue(Duration::from_secs(10)))
        }
    };
}

/// Resources arrives into reconciliation queue in a certain state. This function looks at
/// the state of given `CDBootstrap` resource and decides which actions needs to be performed.
/// The finite set of possible actions is represented by the `CDBootstrapAction` enum.
///
/// # Arguments
/// - `cdbootstrap`: A reference to `CDBootstrap` being reconciled to decide next action upon.
fn determine_action(cr: &CDBootstrap) -> CDBootstrapAction {
    return if cr.meta().deletion_timestamp.is_some() {
        CDBootstrapAction::Delete
    } else if cr
        .meta()
        .finalizers
        .as_ref()
        .map_or(true, |finalizers| finalizers.is_empty())
    {
        CDBootstrapAction::Create
    } else {
        CDBootstrapAction::NoOp
    };
}

/// Actions to be taken when a reconciliation fails - for whatever reason.
/// Prints out the error to `stderr` and requeues the resource for another reconciliation after
/// five seconds.
///
/// # Arguments
/// - `cdbootstrap`: The erroneous resource.
/// - `error`: A reference to the `kube::Error` that occurred during reconciliation.
/// - `_context`: Unused argument. Context Data "injected" automatically by kube-rs.
fn on_error(cr: Arc<CDBootstrap>, error: &Error, context: Arc<ContextData>) -> Action {
    // Clone the necessary data
    let context_clone = context.clone();

    // Use the existing Tokio runtime to spawn the async task
    tokio::spawn(async move {
        match update_status(context_clone, false).await {
            Ok(_) => {
                // Update status was successful
                // Handle other logic if needed
            }
            Err(e) => {
                // Update status failed, handle the error
                eprintln!("Failed to update status: {:?}", e);
            }
        }
    });

    // Continue with the rest of your on_error logic
    eprintln!("Reconciliation error:\n{:?}.\n{:?}", error, cr);
    Action::requeue(Duration::from_secs(5))
}

/// All errors possible to occur during reconciliation
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Any error originating from the `kube-rs` crate
    #[error("Kubernetes reported error: {source}")]
    KubeError {
        #[from]
        source: kube::Error,
    },
    /// Error in user input or CDBootstrap resource definition, typically missing fields.
    #[error("Invalid CDBootstrap CRD: {0}")]
    UserInputError(String),
}

async fn update_status(context: Arc<ContextData>, success: bool) -> Result<(), Error> {
    let api: Api<CDBootstrap> = Api::default_namespaced(context.client.clone());
    let pp = PatchParams::default(); // json merge patch

    let data = json!({
        "status": {
            "succeeded": CDBootstrapStatus { succeeded: success }
        }
    });

    let cdb = api
        .patch_status("test-bootstrap", &pp, &Patch::Merge(data))
        .await?;
    println!(
        "{:?}",
        cdb.status.unwrap_or(CDBootstrapStatus { succeeded: false })
    );

    //assert_eq!(cdb.status.unwrap().succeeded, true);

    Ok(())
}

async fn get_status(context: Arc<ContextData>) -> Result<(), Error> {
    let api: Api<CDBootstrap> = Api::default_namespaced(context.client.clone());
    println!("Get Status on cdbootstrap instance test-bootstrap");
    
    let o = api.get_status("test-bootstrap").await?;
    println!("Got status {:?} for {}", &o.status, &o.name_any());
    
    let o = api.get_metadata("test-bootstrap").await?;
    println!(
        "Got namespace {:?} for {}",
        &o.metadata.namespace.clone().unwrap(),
        &o.name_any()
    );

    Ok(())
}
