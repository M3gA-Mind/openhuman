// Tool-pack routing for `ToolPolicyMiddleware`: the `load_skill` listing this
// session may see, and the route hint offered when it may not.
//
// Split out of `middleware_part_02.rs` at the repo's 750-line layout limit.
// These three are one concern — turning "what can this session reach" into
// text the model acts on — and they are the half of the middleware that knows
// about toolpacks at all.

impl ToolPolicyMiddleware {
    /// The delegation tools this session can actually call that reach one of
    /// `owners`, as tool names.
    ///
    /// Read from the session's own tool set — every synthesised `delegate_*`
    /// tool publishes its target on the erased host-extension slot
    /// (`traits::delegation_target`). Deliberately the only source: a static
    /// owner-to-tool table would duplicate each agent's `delegate_name` and
    /// could name a tool this session was never built with.
    fn callable_delegates_for(&self, owners: &[&str]) -> Vec<String> {
        let mut found: Vec<String> = Vec::new();
        for tool in self.tool_sets.iter().flat_map(|set| set.iter()) {
            let Some(target) =
                crate::openhuman::tools::traits::delegation_target(tool.as_ref())
            else {
                continue;
            };
            if !owners.contains(&target) {
                continue;
            }
            let name = tool.name().to_string();
            // Ask the gate itself, not a predicate that resembles it. This hint
            // names a tool to call DIRECTLY, so the correct test is the one
            // `channel_permission_block` applies to a direct call — covering the
            // session action (`is_denied`, true for `HideFromPrompt`, so a
            // prompt-hidden delegate is not a route) AND the permission ceiling.
            //
            // `Value::Null` is exact, not approximate: every tool this loop sees
            // is a synthesised delegation tool, and those do not override
            // `permission_level_with_args`, so the answer cannot depend on args.
            //
            // The `use_skill` check below deliberately uses `blocks_execution()`
            // instead — there, hiding is the disclosure mechanism, not a refusal.
            // Same tool, two call paths; each asks its own gate.
            if self
                .direct_call_refusal(&name, &serde_json::Value::Null)
                .is_some()
                || found.contains(&name)
            {
                continue;
            }
            found.push(name);
        }
        found
    }

    /// The route sentence for a pack, resolved against THIS session.
    fn route_for_pack(&self, pack: &crate::openhuman::tools::toolpacks::ToolPack) -> String {
        crate::openhuman::tools::toolpacks::route_sentence(
            &self.callable_delegates_for(pack.owners),
            pack.owners,
        )
    }

    /// Render a `load_skill` listing scoped to what this session may call.
    ///
    /// Lives here because the middleware is the only layer holding the session —
    /// `LoadSkillTool` is built once per registry and does not know its caller.
    /// `None` when there is nothing to scope (no `skill` argument, no pack
    /// handle), so the call falls through to the tool's own `execute`.
    fn render_skill_for_session(&self, call: &TaToolCall) -> Option<TaToolResult> {
        let skill = call
            .arguments
            .get("skill")
            .and_then(serde_json::Value::as_str)?;
        let tool = self.resolve_tool(&call.name)?;
        let handle = crate::openhuman::tools::traits::pack_registry_handle(tool.as_ref())?;
        let is_callable = |name: &str| !self.session.decision_for(name).blocks_execution();
        let route = crate::openhuman::tools::toolpacks::pack(skill)
            .map(|pack| self.route_for_pack(pack))
            .unwrap_or_default();
        let rendered = crate::openhuman::tools::toolpacks::render_pack_filtered(
            skill,
            handle,
            // The same predicate the gate applies to `use_skill`'s inner tool.
            // Two sources of truth for "can this session call it" is the bug.
            &is_callable,
            &route,
        );
        let (content, error) = match rendered {
            Ok(text) => (text, None),
            Err(message) => (message.clone(), Some(message)),
        };
        Some(TaToolResult {
            call_id: call.id.clone(),
            name: call.name.clone(),
            content,
            raw: None,
            error,
            elapsed_ms: 0,
        })
    }
}
