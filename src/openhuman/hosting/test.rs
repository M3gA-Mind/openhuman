//! Tests for hosting account resolution, workspace containment, and the tools.
//!
//! The provider itself is TinyHosts' problem and is tested there against a mock
//! of its REST API. What is tested here is the seam: whether an account resolves
//! from configuration, whether a path an agent named can escape the workspace,
//! and whether each tool's schema says what its `execute` actually reads.

use serde_json::json;

use super::*;
use crate::openhuman::config::Config;
use crate::openhuman::tools::traits::Tool;

fn config_with(workspace: &std::path::Path, enabled: bool, api_key: &str) -> Config {
    let mut config = Config::default();
    config.workspace_dir = workspace.to_path_buf();
    config.hosting.enabled = enabled;
    config.hosting.api_key = api_key.to_string();
    config
}

#[test]
fn hosting_off_yields_no_account() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let config = config_with(workspace.path(), false, "token");

    assert!(Account::from_config(&config)
        .expect("resolution does not fail")
        .is_none());
}

#[test]
fn a_configured_key_yields_an_account() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let config = config_with(workspace.path(), true, "token");

    let account = Account::from_config(&config)
        .expect("resolution does not fail")
        .expect("an account, since a key is configured");

    assert_eq!(account.host().kind().as_str(), "vercel");
    assert_eq!(account.workspace_dir(), workspace.path());
}

#[test]
fn an_unknown_provider_is_an_error_rather_than_a_silent_skip() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let mut config = config_with(workspace.path(), true, "token");
    config.hosting.provider = "heroku".to_string();

    let error = Account::from_config(&config).expect_err("an unknown provider fails");

    assert!(
        error.to_string().contains("heroku"),
        "the error should name the provider: {error}"
    );
}

#[test]
fn an_account_reports_itself_without_its_credential() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let config = config_with(workspace.path(), true, "super-secret");

    let account = Account::from_config(&config)
        .expect("resolution does not fail")
        .expect("an account");

    assert!(
        !format!("{account:?}").contains("super-secret"),
        "the credential must never be rendered"
    );
}

#[test]
fn an_account_exposes_every_hosting_tool() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let config = config_with(workspace.path(), true, "token");
    let account = Account::from_config(&config)
        .expect("resolution does not fail")
        .expect("an account");

    let names: Vec<String> = account
        .tools()
        .iter()
        .map(|tool| tool.name().to_string())
        .collect();

    assert_eq!(
        names,
        [
            "hosting_launch_site",
            "hosting_deployment_status",
            "hosting_list_sites",
            "hosting_set_env",
            "hosting_add_domain",
            "hosting_analytics",
            "hosting_rollback",
        ]
    );
}

#[test]
fn only_the_tools_that_change_the_world_carry_an_external_effect() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let config = config_with(workspace.path(), true, "token");
    let account = Account::from_config(&config)
        .expect("resolution does not fail")
        .expect("an account");

    for tool in account.tools() {
        // `hosting_rollback` belongs here: promoting an earlier build changes
        // which one the public is looking at, which is as outward as a launch.
        let expected = matches!(
            tool.name(),
            "hosting_launch_site" | "hosting_set_env" | "hosting_add_domain" | "hosting_rollback"
        );
        assert_eq!(
            tool.external_effect(),
            expected,
            "{} has the wrong external effect",
            tool.name()
        );
    }
}

#[test]
fn every_tool_schema_is_an_object_naming_its_required_arguments() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let config = config_with(workspace.path(), true, "token");
    let account = Account::from_config(&config)
        .expect("resolution does not fail")
        .expect("an account");

    for tool in account.tools() {
        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object", "{}", tool.name());
        assert!(
            schema["properties"].is_object(),
            "{} has no properties",
            tool.name()
        );
        assert!(
            !tool.description().is_empty(),
            "{} has no description",
            tool.name()
        );
    }
}

#[test]
fn a_directory_inside_the_workspace_resolves() {
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(workspace.path().join("site")).expect("mkdir");

    let resolved = resolve_in_workspace(workspace.path(), "site").expect("resolves");

    assert_eq!(
        resolved,
        workspace
            .path()
            .canonicalize()
            .expect("canonical root")
            .join("site")
    );
}

#[test]
fn an_empty_path_is_the_workspace_root() {
    let workspace = tempfile::tempdir().expect("tempdir");

    let resolved = resolve_in_workspace(workspace.path(), "  ").expect("resolves");

    assert_eq!(
        resolved,
        workspace.path().canonicalize().expect("canonical root")
    );
}

#[test]
fn a_path_outside_the_workspace_is_refused() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let root = workspace.path();

    // A deployment uploads every byte under the directory to a third party, so
    // this is the check that decides what may leave the machine.
    assert!(resolve_in_workspace(root, "/etc").is_err());
    assert!(resolve_in_workspace(root, "../..").is_err());
    assert!(resolve_in_workspace(root, "does-not-exist").is_err());
}

#[test]
fn a_file_is_not_a_deployable_directory() {
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::write(workspace.path().join("page.tsx"), b"x").expect("write");

    let error =
        resolve_in_workspace(workspace.path(), "page.tsx").expect_err("a file is not a directory");

    assert!(error.to_string().contains("not a directory"), "{error}");
}

#[tokio::test]
async fn launching_reports_a_missing_directory_instead_of_deploying_nothing() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let config = config_with(workspace.path(), true, "token");
    let account = Account::from_config(&config)
        .expect("resolution does not fail")
        .expect("an account");

    let tool = tools::LaunchSiteTool::new(account);
    let result = tool
        .execute(json!({"site": "shop", "path": "missing"}))
        .await
        .expect("the tool reports rather than panics");

    assert!(result.is_error);
}

#[tokio::test]
async fn launching_without_a_site_name_is_refused_before_any_upload() {
    let workspace = tempfile::tempdir().expect("tempdir");
    std::fs::write(workspace.path().join("package.json"), b"{}").expect("write");
    let config = config_with(workspace.path(), true, "token");
    let account = Account::from_config(&config)
        .expect("resolution does not fail")
        .expect("an account");

    let tool = tools::LaunchSiteTool::new(account);
    let result = tool
        .execute(json!({"path": "."}))
        .await
        .expect("the tool reports rather than panics");

    assert!(result.is_error);
}

#[tokio::test]
async fn a_read_tool_reports_a_missing_argument_rather_than_calling_out() {
    let workspace = tempfile::tempdir().expect("tempdir");
    let config = config_with(workspace.path(), true, "token");
    let account = Account::from_config(&config)
        .expect("resolution does not fail")
        .expect("an account");

    let result = tools::DeploymentStatusTool::new(account.host())
        .execute(json!({}))
        .await
        .expect("the tool reports rather than panics");

    assert!(result.is_error);
}

// ── hosting_rollback ────────────────────────────────────────────────────────

/// A [`Host`] that answers `list_deployments` from a script and records what
/// `promote` was asked to do.
///
/// Every other method is unreachable on purpose: a rollback that called out to
/// anything else would be doing hosting work this seam has no business doing,
/// and a stub that panics says so louder than one returning a default.
#[derive(Debug)]
struct ScriptedHost {
    deployments: Vec<tinyhosts::Deployment>,
    promoted: std::sync::Mutex<Option<(String, String)>>,
}

impl ScriptedHost {
    fn new(deployments: Vec<tinyhosts::Deployment>) -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            deployments,
            promoted: std::sync::Mutex::new(None),
        })
    }

    fn promoted_id(&self) -> Option<String> {
        self.promoted
            .lock()
            .unwrap()
            .as_ref()
            .map(|(_, id)| id.clone())
    }
}

/// One deployment, with only the fields the target choice reads.
fn deployment(
    id: &str,
    status: tinyhosts::DeploymentStatus,
    target: tinyhosts::DeploymentTarget,
) -> tinyhosts::Deployment {
    tinyhosts::Deployment {
        id: id.to_string(),
        site: "shop".to_string(),
        url: None,
        status,
        target,
        created_at_ms: None,
        error_message: None,
    }
}

#[async_trait::async_trait]
impl tinyhosts::Host for ScriptedHost {
    fn kind(&self) -> tinyhosts::ProviderKind {
        tinyhosts::ProviderKind::Vercel
    }
    async fn list_deployments(
        &self,
        _site: &str,
        _limit: u32,
    ) -> tinyhosts::Result<Vec<tinyhosts::Deployment>> {
        Ok(self.deployments.clone())
    }
    async fn promote(&self, site: &str, deployment: &str) -> tinyhosts::Result<()> {
        *self.promoted.lock().unwrap() = Some((site.to_string(), deployment.to_string()));
        Ok(())
    }
    async fn create_site(&self, _spec: &tinyhosts::SiteSpec) -> tinyhosts::Result<tinyhosts::Site> {
        unreachable!("rollback does not create sites")
    }
    async fn find_site(&self, _name: &str) -> tinyhosts::Result<Option<tinyhosts::Site>> {
        unreachable!("rollback does not look up sites")
    }
    async fn list_sites(&self, _limit: u32) -> tinyhosts::Result<Vec<tinyhosts::Site>> {
        unreachable!("rollback does not list sites")
    }
    async fn set_env(&self, _site: &str, _vars: &[tinyhosts::EnvVar]) -> tinyhosts::Result<()> {
        unreachable!("rollback does not set environment variables")
    }
    async fn list_env(&self, _site: &str) -> tinyhosts::Result<Vec<tinyhosts::EnvVarRecord>> {
        unreachable!("rollback does not read environment variables")
    }
    async fn provision_database(
        &self,
        _spec: &tinyhosts::DatabaseSpec,
    ) -> tinyhosts::Result<tinyhosts::Database> {
        unreachable!("rollback does not provision databases")
    }
    async fn attach_database(
        &self,
        _database: &tinyhosts::Database,
        _site: &str,
    ) -> tinyhosts::Result<Vec<String>> {
        unreachable!("rollback does not attach databases")
    }
    async fn deploy(
        &self,
        _request: &tinyhosts::DeployRequest,
    ) -> tinyhosts::Result<tinyhosts::Deployment> {
        unreachable!("rollback promotes an existing build, it does not create one")
    }
    async fn deployment(&self, _id: &str) -> tinyhosts::Result<tinyhosts::Deployment> {
        unreachable!("rollback does not poll a single deployment")
    }
    async fn add_domain(&self, _site: &str, _domain: &str) -> tinyhosts::Result<tinyhosts::Domain> {
        unreachable!("rollback does not add domains")
    }
    async fn list_domains(&self, _site: &str) -> tinyhosts::Result<Vec<tinyhosts::Domain>> {
        unreachable!("rollback does not list domains")
    }
    async fn analytics(
        &self,
        _query: &tinyhosts::AnalyticsQuery,
    ) -> tinyhosts::Result<tinyhosts::AnalyticsSummary> {
        unreachable!("rollback does not read analytics")
    }
}

/// The default target: the ready production deployment **before** the one
/// currently serving. The provider returns newest-first, so `new` is live and
/// `previous` is what "roll back" means.
#[tokio::test]
async fn rolling_back_promotes_the_deployment_before_the_live_one() {
    use tinyhosts::{DeploymentStatus::Ready, DeploymentTarget::Production};
    let host = ScriptedHost::new(vec![
        deployment("new", Ready, Production),
        deployment("previous", Ready, Production),
        deployment("older", Ready, Production),
    ]);

    let result = tools::RollbackTool::new(host.clone())
        .execute(json!({ "site": "shop" }))
        .await
        .expect("the tool reports rather than panics");

    assert!(!result.is_error, "{result:?}");
    assert_eq!(
        host.promoted.lock().unwrap().clone(),
        Some(("shop".to_string(), "previous".to_string())),
        "the one before the live deployment, not the live one and not the oldest"
    );
}

/// A failed or still-building deployment is never promoted. Rolling on to one
/// would take the site down in the name of recovering it, which is the single
/// outcome this tool exists to prevent.
#[tokio::test]
async fn rolling_back_skips_deployments_that_are_not_ready() {
    use tinyhosts::DeploymentStatus::{Building, Failed, Ready};
    use tinyhosts::DeploymentTarget::Production;
    let host = ScriptedHost::new(vec![
        deployment("live", Ready, Production),
        deployment("broken", Failed, Production),
        deployment("half-built", Building, Production),
        deployment("last-good", Ready, Production),
    ]);

    tools::RollbackTool::new(host.clone())
        .execute(json!({ "site": "shop" }))
        .await
        .expect("the tool reports rather than panics");

    assert_eq!(
        host.promoted_id().as_deref(),
        Some("last-good"),
        "a failed or building deployment is not a rollback target"
    );
}

/// A preview deployment is not a rollback target either: it is not attached to
/// the site's domains, so promoting one is a deploy rather than a rollback.
#[tokio::test]
async fn rolling_back_ignores_preview_deployments() {
    use tinyhosts::DeploymentStatus::Ready;
    use tinyhosts::DeploymentTarget::{Preview, Production};
    let host = ScriptedHost::new(vec![
        deployment("live", Ready, Production),
        deployment("a-preview", Ready, Preview),
        deployment("last-good", Ready, Production),
    ]);

    tools::RollbackTool::new(host.clone())
        .execute(json!({ "site": "shop" }))
        .await
        .expect("the tool reports rather than panics");

    assert_eq!(
        host.promoted_id().as_deref(),
        Some("last-good"),
        "a preview build serves no domain, so it is not what `back` means"
    );
}

/// Nothing to roll back to is an error, not a silent no-op. A site with one
/// good deployment has no earlier one, and reporting success would tell an
/// operator mid-incident that they had recovered when they had not.
#[tokio::test]
async fn a_site_with_nothing_earlier_reports_it_rather_than_promoting_the_live_one() {
    use tinyhosts::{DeploymentStatus::Ready, DeploymentTarget::Production};
    let host = ScriptedHost::new(vec![deployment("only", Ready, Production)]);

    let result = tools::RollbackTool::new(host.clone())
        .execute(json!({ "site": "shop" }))
        .await
        .expect("the tool reports rather than panics");

    assert!(result.is_error, "{result:?}");
    assert!(
        host.promoted_id().is_none(),
        "nothing was promoted, least of all the deployment already serving"
    );
}

/// An explicit id is taken as given — the caller may be rolling on to something
/// the default choice would not have picked, which is why the argument exists.
#[tokio::test]
async fn an_explicit_deployment_id_overrides_the_default_choice() {
    use tinyhosts::{DeploymentStatus::Ready, DeploymentTarget::Production};
    let host = ScriptedHost::new(vec![
        deployment("live", Ready, Production),
        deployment("previous", Ready, Production),
    ]);

    tools::RollbackTool::new(host.clone())
        .execute(json!({ "site": "shop", "deployment_id": "much-older" }))
        .await
        .expect("the tool reports rather than panics");

    assert_eq!(host.promoted_id().as_deref(), Some("much-older"));
}

/// The site name is required, and its absence is reported before any call out.
#[tokio::test]
async fn rolling_back_without_a_site_is_refused_before_calling_out() {
    let host = ScriptedHost::new(Vec::new());

    let result = tools::RollbackTool::new(host.clone())
        .execute(json!({}))
        .await
        .expect("the tool reports rather than panics");

    assert!(result.is_error);
    assert!(host.promoted_id().is_none());
}
