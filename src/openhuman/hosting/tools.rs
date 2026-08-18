//! The `hosting_*` agent tools.
//!
//! Seven tools over one hosting account. They are thin on purpose: argument
//! parsing, one call into [`tinyhosts`], and a result described for a model.
//! Anything that looks like hosting logic belongs in the crate, where it is
//! provider-independent and tested against a mock of the provider's API.
//!
//! Four carry `external_effect` and route through the approval gate:
//! `hosting_launch_site` uploads a directory to a third party and can spend
//! money on a database, `hosting_set_env` and `hosting_add_domain` change what
//! an existing site serves, and `hosting_rollback` changes which build the
//! public is looking at. The other three read.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use tinyhosts::{
    AnalyticsDimension, AnalyticsQuery, Bundle, DatabaseKind, DatabaseSpec, DeploymentStatus,
    DeploymentTarget, Domain, EnvVar, Host, Launch, LaunchPlan, SiteSpec,
};

use super::{resolve_in_workspace, Account};
use crate::openhuman::tools::traits::{PermissionLevel, Tool, ToolCallOptions, ToolResult};

/// Every hosting tool, for one account.
pub fn hosting_tools(account: &Account) -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(LaunchSiteTool::new(account.clone())),
        Box::new(DeploymentStatusTool::new(account.host())),
        Box::new(ListSitesTool::new(account.host())),
        Box::new(SetEnvTool::new(account.host())),
        Box::new(AddDomainTool::new(account.host())),
        Box::new(AnalyticsTool::new(account.host())),
        Box::new(RollbackTool::new(account.host())),
    ]
}

/// Reads a required string argument.
fn required_str(args: &Value, key: &str) -> anyhow::Result<String> {
    args.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow::anyhow!("`{key}` is required"))
}

/// Renders one `env` object value. A number or a bool is still a variable, so
/// it is rendered rather than dropped. `null` and a container are refused: a
/// variable silently set to `"null"` is worse than a named error.
fn env_value(key: &str, value: &Value) -> anyhow::Result<String> {
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Number(_) | Value::Bool(_) => Ok(value.to_string()),
        _ => anyhow::bail!("`env.{key}` must be a string, number, or boolean"),
    }
}

/// Renders a launch as the two sentences a model needs: where it is, and what
/// it still has to wait for.
fn describe(launch: &Launch) -> String {
    let mut lines = vec![format!(
        "Site **{}** ({}), deployment `{}` is {:?}.",
        launch.site.name,
        if launch.created_site {
            "created"
        } else {
            "already existed"
        },
        launch.deployment.id,
        launch.deployment.status,
    )];

    match launch.url() {
        Some(url) => lines.push(format!(
            "It will serve from {url} once the build finishes — poll \
             `hosting_deployment_status` with the deployment id."
        )),
        None => lines.push(
            "The provider has not assigned a URL yet; poll \
             `hosting_deployment_status` with the deployment id."
                .to_string(),
        ),
    }

    if let Some(database) = &launch.database {
        lines.push(format!(
            "Database **{}** ({}) is {}; it injected {} into the site's \
             environment. The values are the provider's — nothing here can read them.",
            database.name,
            database.kind.as_str(),
            database.status,
            if launch.database_env_keys.is_empty() {
                "no variables".to_string()
            } else {
                launch.database_env_keys.join(", ")
            },
        ));
    }

    if !launch.domains.is_empty() {
        let unverified: Vec<&str> = launch
            .domains
            .iter()
            .filter(|domain| !domain.verified)
            .map(|domain| domain.name.as_str())
            .collect();
        if unverified.is_empty() {
            lines.push("Every domain is verified.".to_string());
        } else {
            lines.push(format!(
                "These domains still need their DNS records pointed at the \
                 provider before they serve traffic: {}.",
                unverified.join(", ")
            ));
        }
    }

    lines.join("\n\n")
}

// ── hosting_launch_site ─────────────────────────────────────────────────────

/// Deploys a workspace directory as a live site, with an optional database.
pub struct LaunchSiteTool {
    account: Account,
}

impl LaunchSiteTool {
    pub fn new(account: Account) -> Self {
        Self { account }
    }

    /// Builds the plan an invocation describes.
    fn plan(&self, args: &Value) -> anyhow::Result<LaunchPlan> {
        let site = required_str(args, "site")?;
        let path = args
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or(".")
            .to_string();

        let directory = resolve_in_workspace(self.account.workspace_dir(), &path)?;
        let bundle = Bundle::from_dir(&directory)?;

        let mut plan = LaunchPlan::new(SiteSpec::new(site), bundle);

        if let Some(name) = args.get("database").and_then(Value::as_str) {
            let name = name.trim();
            if !name.is_empty() {
                let kind = match args
                    .get("database_kind")
                    .and_then(Value::as_str)
                    .unwrap_or("postgres")
                {
                    "postgres" => DatabaseKind::Postgres,
                    "redis" => DatabaseKind::Redis,
                    "blob" => DatabaseKind::Blob,
                    other => DatabaseKind::Other(other.to_string()),
                };
                plan = plan.with_database(DatabaseSpec::new(name).with_kind(kind));
            }
        }

        if let Some(env) = args.get("env").and_then(Value::as_object) {
            let vars = env
                .iter()
                .map(|(key, value)| Ok(EnvVar::new(key, env_value(key, value)?)))
                .collect::<anyhow::Result<Vec<_>>>()?;
            plan = plan.with_env(vars);
        }

        if let Some(domains) = args.get("domains").and_then(Value::as_array) {
            plan = plan.with_domains(
                domains
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|domain| !domain.is_empty())
                    .map(ToOwned::to_owned)
                    .collect(),
            );
        }

        if args
            .get("production")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            plan = plan.into_production();
        }

        Ok(plan)
    }
}

#[async_trait]
impl Tool for LaunchSiteTool {
    fn name(&self) -> &str {
        "hosting_launch_site"
    }

    fn description(&self) -> &str {
        "Deploy a directory in the workspace to a real hosting provider as a \
         live website, optionally provisioning a managed database and wiring it \
         in. Creates the site if it does not exist yet, so calling it again \
         redeploys. Use for a Next.js application or a static site. The build \
         starts immediately and finishes later: poll hosting_deployment_status \
         with the returned deployment id until it is ready. Node dependencies, \
         build output and .git are never uploaded — the provider builds from \
         source."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["site"],
            "properties": {
                "site": {
                    "type": "string",
                    "description": "The site's name on the provider, e.g. 'acme-shop'. \
                                    Reused on a redeploy."
                },
                "path": {
                    "type": "string",
                    "description": "Directory to deploy, relative to the workspace. \
                                    Defaults to the workspace root."
                },
                "database": {
                    "type": "string",
                    "description": "Name for a managed database to provision and connect. \
                                    Omit if the site needs none. The connection variables \
                                    are injected by the provider before the build."
                },
                "database_kind": {
                    "type": "string",
                    "enum": ["postgres", "redis", "blob"],
                    "description": "What the database speaks. Defaults to postgres."
                },
                "env": {
                    "type": "object",
                    "description": "Environment variables to set before the build. \
                                    Do not put a database connection string here; the \
                                    provider injects its own.",
                    "additionalProperties": { "type": "string" }
                },
                "domains": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Custom domains to attach. They need DNS records \
                                    pointed at the provider before they serve traffic."
                },
                "production": {
                    "type": "boolean",
                    "description": "Deploy to production rather than to a preview URL. \
                                    Defaults to false."
                }
            }
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::Write
    }

    fn external_effect(&self) -> bool {
        true
    }

    fn supports_markdown(&self) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        self.execute_with_options(args, ToolCallOptions::default())
            .await
    }

    async fn execute_with_options(
        &self,
        args: Value,
        _options: ToolCallOptions,
    ) -> anyhow::Result<ToolResult> {
        let plan = match self.plan(&args) {
            Ok(plan) => plan,
            Err(error) => return Ok(ToolResult::error(error.to_string())),
        };

        tracing::info!(
            site = %plan.site.name,
            files = plan.bundle.len(),
            bytes = plan.bundle.total_bytes(),
            database = plan.database.is_some(),
            target = plan.target.as_str(),
            "[hosting] launching"
        );

        match tinyhosts::launch(self.account.host().as_ref(), &plan).await {
            Ok(launch) => Ok(ToolResult::success_with_markdown(
                serde_json::to_value(&launch)?,
                describe(&launch),
            )),
            Err(error) => Ok(ToolResult::error(error.to_string())),
        }
    }
}

// ── hosting_deployment_status ───────────────────────────────────────────────

/// Reads one deployment's current state.
pub struct DeploymentStatusTool {
    host: Arc<dyn Host>,
}

impl DeploymentStatusTool {
    pub fn new(host: Arc<dyn Host>) -> Self {
        Self { host }
    }
}

#[async_trait]
impl Tool for DeploymentStatusTool {
    fn name(&self) -> &str {
        "hosting_deployment_status"
    }

    fn description(&self) -> &str {
        "Check whether a deployment has finished building and is serving. Poll \
         this after hosting_launch_site until the status is ready, failed, or \
         canceled. A failed deployment reports the provider's build error."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["deployment_id"],
            "properties": {
                "deployment_id": {
                    "type": "string",
                    "description": "The id hosting_launch_site returned."
                }
            }
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::ReadOnly
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let id = match required_str(&args, "deployment_id") {
            Ok(id) => id,
            Err(error) => return Ok(ToolResult::error(error.to_string())),
        };

        match self.host.deployment(&id).await {
            Ok(deployment) => Ok(ToolResult::success(serde_json::to_string_pretty(
                &deployment,
            )?)),
            Err(error) => Ok(ToolResult::error(error.to_string())),
        }
    }
}

// ── hosting_list_sites ──────────────────────────────────────────────────────

/// Lists the sites on the account.
pub struct ListSitesTool {
    host: Arc<dyn Host>,
}

impl ListSitesTool {
    pub fn new(host: Arc<dyn Host>) -> Self {
        Self { host }
    }
}

#[async_trait]
impl Tool for ListSitesTool {
    fn name(&self) -> &str {
        "hosting_list_sites"
    }

    fn description(&self) -> &str {
        "List the sites already on the hosting account, newest first. Use it to \
         find out whether a site exists before deploying, or to recover a name."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "limit": {
                    "type": "integer",
                    "description": "How many to return. Defaults to 20."
                }
            }
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::ReadOnly
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(20)
            .clamp(1, 100) as u32;

        match self.host.list_sites(limit).await {
            Ok(sites) => Ok(ToolResult::success(serde_json::to_string_pretty(&sites)?)),
            Err(error) => Ok(ToolResult::error(error.to_string())),
        }
    }
}

// ── hosting_set_env ─────────────────────────────────────────────────────────

/// Sets environment variables on an existing site.
pub struct SetEnvTool {
    host: Arc<dyn Host>,
}

impl SetEnvTool {
    pub fn new(host: Arc<dyn Host>) -> Self {
        Self { host }
    }
}

#[async_trait]
impl Tool for SetEnvTool {
    fn name(&self) -> &str {
        "hosting_set_env"
    }

    fn description(&self) -> &str {
        "Set environment variables on a site, replacing any of the same name. \
         The site must be redeployed afterwards for a build-time variable to \
         take effect. Values are write-only: they can never be read back through \
         these tools."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["site", "env"],
            "properties": {
                "site": { "type": "string", "description": "The site's name." },
                "env": {
                    "type": "object",
                    "description": "Variables to set.",
                    "additionalProperties": { "type": "string" }
                },
                "secret": {
                    "type": "boolean",
                    "description": "Store them write-only at the provider. Defaults to false."
                },
                "production_only": {
                    "type": "boolean",
                    "description": "Apply to production only rather than every environment."
                }
            }
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::Write
    }

    fn external_effect(&self) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let site = match required_str(&args, "site") {
            Ok(site) => site,
            Err(error) => return Ok(ToolResult::error(error.to_string())),
        };
        let Some(env) = args.get("env").and_then(Value::as_object) else {
            return Ok(ToolResult::error("`env` must be an object of variables"));
        };

        let secret = args.get("secret").and_then(Value::as_bool).unwrap_or(false);
        let targets = if args
            .get("production_only")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            vec![DeploymentTarget::Production]
        } else {
            Vec::new()
        };

        let vars: Vec<EnvVar> = match env
            .iter()
            .map(|(key, value)| {
                let var = EnvVar::new(key, env_value(key, value)?).with_targets(targets.clone());
                Ok(if secret { var.secret() } else { var })
            })
            .collect::<anyhow::Result<Vec<_>>>()
        {
            Ok(vars) => vars,
            Err(error) => return Ok(ToolResult::error(error.to_string())),
        };

        let names: Vec<&str> = vars.iter().map(|var| var.key.as_str()).collect();
        match self.host.set_env(&site, &vars).await {
            Ok(()) => Ok(ToolResult::success(format!(
                "Set {} on {site}. Redeploy the site for a build-time variable to take effect.",
                names.join(", ")
            ))),
            Err(error) => Ok(ToolResult::error(error.to_string())),
        }
    }
}

// ── hosting_add_domain ──────────────────────────────────────────────────────

/// Attaches a custom domain to a site.
pub struct AddDomainTool {
    host: Arc<dyn Host>,
}

impl AddDomainTool {
    pub fn new(host: Arc<dyn Host>) -> Self {
        Self { host }
    }
}

#[async_trait]
impl Tool for AddDomainTool {
    fn name(&self) -> &str {
        "hosting_add_domain"
    }

    fn description(&self) -> &str {
        "Attach a custom domain to a site. The domain does not serve traffic \
         until its DNS records point at the provider, which the user has to do \
         at their registrar — the response says whether the provider has \
         verified it yet."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["site", "domain"],
            "properties": {
                "site": { "type": "string", "description": "The site's name." },
                "domain": { "type": "string", "description": "e.g. 'shop.example.com'." }
            }
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::Write
    }

    fn external_effect(&self) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let site = match required_str(&args, "site") {
            Ok(site) => site,
            Err(error) => return Ok(ToolResult::error(error.to_string())),
        };
        let domain = match required_str(&args, "domain") {
            Ok(domain) => domain,
            Err(error) => return Ok(ToolResult::error(error.to_string())),
        };

        match self.host.add_domain(&site, &domain).await {
            Ok(Domain {
                name,
                verified: true,
                ..
            }) => Ok(ToolResult::success(format!(
                "{name} is attached to {site} and verified."
            ))),
            Ok(Domain { name, .. }) => Ok(ToolResult::success(format!(
                "{name} is attached to {site} but not verified yet: its DNS \
                 records still have to point at the provider."
            ))),
            Err(error) => Ok(ToolResult::error(error.to_string())),
        }
    }
}

// ── hosting_analytics ───────────────────────────────────────────────────────

/// Reports the traffic a site served.
pub struct AnalyticsTool {
    host: Arc<dyn Host>,
}

impl AnalyticsTool {
    pub fn new(host: Arc<dyn Host>) -> Self {
        Self { host }
    }
}

#[async_trait]
impl Tool for AnalyticsTool {
    fn name(&self) -> &str {
        "hosting_analytics"
    }

    fn description(&self) -> &str {
        "Report how much traffic a hosted site served over the last N days — \
         visitors and page views, optionally broken down by country, path, \
         device, browser, or referrer."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["site"],
            "properties": {
                "site": { "type": "string", "description": "The site's name." },
                "days": {
                    "type": "integer",
                    "description": "How many days back to report. Defaults to 7."
                },
                "breakdown": {
                    "type": "string",
                    "enum": [
                        "country", "request_path", "device_type",
                        "browser_name", "os_name", "referrer_hostname", "route"
                    ],
                    "description": "Break the totals down by this dimension."
                }
            }
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::ReadOnly
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let site = match required_str(&args, "site") {
            Ok(site) => site,
            Err(error) => return Ok(ToolResult::error(error.to_string())),
        };
        let days = args
            .get("days")
            .and_then(Value::as_u64)
            .unwrap_or(7)
            .clamp(1, 365);

        let until_ms = chrono::Utc::now().timestamp_millis().max(0) as u64;
        let since_ms = until_ms.saturating_sub(days * 24 * 60 * 60 * 1000);

        let mut query = AnalyticsQuery::new(site, since_ms, until_ms);
        if let Some(dimension) = args.get("breakdown").and_then(Value::as_str) {
            let breakdown = match dimension {
                "country" => AnalyticsDimension::Country,
                "request_path" => AnalyticsDimension::RequestPath,
                "device_type" => AnalyticsDimension::DeviceType,
                "browser_name" => AnalyticsDimension::BrowserName,
                "os_name" => AnalyticsDimension::OsName,
                "referrer_hostname" => AnalyticsDimension::ReferrerHostname,
                "route" => AnalyticsDimension::Route,
                other => {
                    return Ok(ToolResult::error(format!(
                        "`breakdown` must be one of country, request_path, device_type, \
                         browser_name, os_name, referrer_hostname, route — not `{other}`"
                    )));
                }
            };
            query = query.with_breakdown(breakdown);
        }

        match self.host.analytics(&query).await {
            Ok(summary) => Ok(ToolResult::success(serde_json::to_string_pretty(&summary)?)),
            Err(error) => Ok(ToolResult::error(error.to_string())),
        }
    }
}

// ── hosting_rollback ────────────────────────────────────────────────────────

/// How far back to look for a deployment worth rolling on to.
///
/// The provider returns newest-first, so this is "the last N deployments of
/// this site". Large enough that a run of failed builds does not hide the last
/// good one, small enough to stay one cheap call.
const ROLLBACK_SCAN: u32 = 20;

/// Promotes an earlier deployment back to production.
///
/// The recovery half of `hosting_launch_site`. A launch can provision a
/// database, rewrite environment variables and attach domains in one call, so
/// an agent could compose a broken site in a single step and — before this —
/// had no single step to undo it: only `hosting_set_env` and
/// `hosting_add_domain`, one variable and one domain at a time.
///
/// The crate already had the primitive. [`Host::promote`] is provider-
/// independent and tested against a mock of the provider's API, so nothing
/// Vercel-specific appears here; this is argument parsing and a target choice.
pub struct RollbackTool {
    host: Arc<dyn Host>,
}

impl RollbackTool {
    pub fn new(host: Arc<dyn Host>) -> Self {
        Self { host }
    }

    /// The deployment to roll on to when the caller named none.
    ///
    /// "Roll back" with no target is ambiguous, and the ambiguity is resolved
    /// here rather than left to the model: **the most recent production
    /// deployment that is `Ready` and is not the one currently serving**. The
    /// provider returns newest-first, so the first `Ready` production entry is
    /// what is live now and the second is what "back" means.
    ///
    /// Only `Ready` counts. Promoting a `Failed` or still-`Building` deployment
    /// would take the site down in the name of recovering it, which is the one
    /// outcome this tool exists to prevent.
    async fn previous_ready(&self, site: &str) -> anyhow::Result<String> {
        let deployments = self.host.list_deployments(site, ROLLBACK_SCAN).await?;
        let mut ready = deployments.iter().filter(|deployment| {
            deployment.target == DeploymentTarget::Production
                && deployment.status == DeploymentStatus::Ready
        });
        // The one serving now, skipped: rolling back onto it is a no-op an
        // operator would read as a successful recovery.
        let current = ready.next();
        match (current, ready.next()) {
            (_, Some(previous)) => Ok(previous.id.clone()),
            (Some(_), None) => anyhow::bail!(
                "`{site}` has only one ready production deployment, so there is nothing to roll \
                 back to. Name a deployment_id explicitly if you meant a different one."
            ),
            (None, _) => anyhow::bail!(
                "`{site}` has no ready production deployment in its last {ROLLBACK_SCAN}, so \
                 there is nothing to roll back to."
            ),
        }
    }
}

#[async_trait]
impl Tool for RollbackTool {
    fn name(&self) -> &str {
        "hosting_rollback"
    }

    fn description(&self) -> &str {
        "Roll a site back to an earlier deployment by promoting it to \
         production. Use this to recover from a bad deploy. With no \
         deployment_id, rolls back to the most recent ready production \
         deployment before the one currently serving; pass deployment_id (from \
         hosting_deployment_status or a previous launch) to choose a specific \
         one. Changes what the public sees."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["site"],
            "properties": {
                "site": {
                    "type": "string",
                    "description": "The site name, as hosting_list_sites reports it."
                },
                "deployment_id": {
                    "type": "string",
                    "description": "The deployment to promote. Omit to roll back to the \
                                    one before the deployment currently serving."
                }
            }
        })
    }

    fn permission_level(&self) -> PermissionLevel {
        PermissionLevel::Write
    }

    /// Changes which build the public is looking at, so it gates like a launch.
    fn external_effect(&self) -> bool {
        true
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let site = match required_str(&args, "site") {
            Ok(site) => site,
            Err(error) => return Ok(ToolResult::error(error.to_string())),
        };

        // An explicit id is taken as given — the caller may be rolling on to
        // something this tool's own heuristic would not have picked, which is
        // exactly why the argument exists.
        let deployment = match args.get("deployment_id").and_then(Value::as_str) {
            Some(id) if !id.trim().is_empty() => id.trim().to_string(),
            _ => match self.previous_ready(&site).await {
                Ok(id) => id,
                Err(error) => return Ok(ToolResult::error(error.to_string())),
            },
        };

        match self.host.promote(&site, &deployment).await {
            Ok(()) => Ok(ToolResult::success(format!(
                "`{site}` is now serving deployment `{deployment}`."
            ))),
            Err(error) => Ok(ToolResult::error(error.to_string())),
        }
    }
}
