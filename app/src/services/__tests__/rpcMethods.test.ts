import * as fs from 'node:fs';
import * as path from 'node:path';
import { describe, expect, test } from 'vitest';

import { CORE_RPC_METHODS, LEGACY_METHOD_ALIASES, normalizeRpcMethod } from '../rpcMethods';

describe('rpcMethods catalog', () => {
  describe('normalizeRpcMethod', () => {
    test('resolves all legacy aliases to their canonical core method', () => {
      for (const [legacyMethod, coreMethod] of Object.entries(LEGACY_METHOD_ALIASES)) {
        expect(normalizeRpcMethod(legacyMethod)).toBe(coreMethod);
      }
    });

    test('transforms auth methods by replacing dots with underscores', () => {
      expect(normalizeRpcMethod('openhuman.auth.login')).toBe('openhuman.auth_login');
      expect(normalizeRpcMethod('openhuman.auth.get.state')).toBe('openhuman.auth_get_state');
      expect(normalizeRpcMethod('openhuman.auth.a.b.c')).toBe('openhuman.auth_a_b_c');
    });

    test('returns unmapped or unrecognized methods unchanged', () => {
      expect(normalizeRpcMethod('openhuman.threads_list')).toBe('openhuman.threads_list');
      expect(normalizeRpcMethod('openhuman.unknown_method')).toBe('openhuman.unknown_method');
      expect(normalizeRpcMethod('')).toBe('');
      expect(normalizeRpcMethod('random_string')).toBe('random_string');
    });

    test('trims whitespace and converts to lower case', () => {
      expect(normalizeRpcMethod('  OpenHuman.Auth.Login  ')).toBe('openhuman.auth_login');
      expect(normalizeRpcMethod('  OPENHUMAN.GET_CONFIG ')).toBe(CORE_RPC_METHODS.configGet);
      expect(normalizeRpcMethod('OpenHuman.Unrecognized_Status  ')).toBe(
        'openhuman.unrecognized_status'
      );
      expect(normalizeRpcMethod('   some_RANDOM_method  ')).toBe('some_random_method');
    });
  });

  test('legacy aliases point at canonical method values', () => {
    expect(LEGACY_METHOD_ALIASES['openhuman.update_model_settings']).toBe(
      CORE_RPC_METHODS.inferenceUpdateModelSettings
    );
    expect(LEGACY_METHOD_ALIASES['openhuman.workspace_onboarding_flag_set']).toBe(
      CORE_RPC_METHODS.configWorkspaceOnboardingFlagSet
    );
  });

  describe('MCP client legacy alias resolution (Sentry CORE-RUST-DW/DV/DT/DS/DR)', () => {
    test('mcp_clients.list resolves to mcp_clients_installed_list', () => {
      expect(normalizeRpcMethod('mcp_clients.list')).toBe(CORE_RPC_METHODS.mcpClientsInstalledList);
    });

    test('openhuman.mcp_clients_list resolves to mcp_clients_installed_list', () => {
      expect(normalizeRpcMethod('openhuman.mcp_clients_list')).toBe(
        CORE_RPC_METHODS.mcpClientsInstalledList
      );
    });

    test('openhuman.mcp_list resolves to mcp_clients_installed_list', () => {
      expect(normalizeRpcMethod('openhuman.mcp_list')).toBe(
        CORE_RPC_METHODS.mcpClientsInstalledList
      );
    });

    test('openhuman.mcp_servers_list resolves to mcp_clients_installed_list', () => {
      expect(normalizeRpcMethod('openhuman.mcp_servers_list')).toBe(
        CORE_RPC_METHODS.mcpClientsInstalledList
      );
    });

    test('openhuman.tool_registry_call resolves to mcp_clients_tool_call', () => {
      expect(normalizeRpcMethod('openhuman.tool_registry_call')).toBe(
        CORE_RPC_METHODS.mcpClientsToolCall
      );
    });

    test('dotted tool_registry.diagnostics resolves to the canonical method (#3294)', () => {
      expect(normalizeRpcMethod('tool_registry.diagnostics')).toBe(
        CORE_RPC_METHODS.toolRegistryDiagnostics
      );
      expect(CORE_RPC_METHODS.toolRegistryDiagnostics).toBe('openhuman.tool_registry_diagnostics');
    });

    test('canonical mcp_clients_installed_list passes through unchanged', () => {
      expect(normalizeRpcMethod('openhuman.mcp_clients_installed_list')).toBe(
        'openhuman.mcp_clients_installed_list'
      );
    });

    test('canonical mcp_clients_tool_call passes through unchanged', () => {
      expect(normalizeRpcMethod('openhuman.mcp_clients_tool_call')).toBe(
        'openhuman.mcp_clients_tool_call'
      );
    });
  });

  describe('health legacy alias resolution (Sentry CORE-RUST-FG / CORE-RUST-G0)', () => {
    test('health_snapshot resolves to openhuman.health_snapshot', () => {
      expect(normalizeRpcMethod('health_snapshot')).toBe(CORE_RPC_METHODS.healthSnapshot);
    });

    test('openhuman.system_info resolves to openhuman.health_system_info (Sentry CORE-RUST-G0)', () => {
      // Older clients called openhuman.system_info before the method was
      // namespaced under health as openhuman.health_system_info.
      expect(normalizeRpcMethod('openhuman.system_info')).toBe(CORE_RPC_METHODS.healthSystemInfo);
    });

    test('canonical health_system_info passes through unchanged', () => {
      expect(normalizeRpcMethod('openhuman.health_system_info')).toBe(
        'openhuman.health_system_info'
      );
    });
  });

  describe('channels legacy alias resolution (Sentry OPENHUMAN-CORE-1Y / OPENHUMAN-CORE-1Z)', () => {
    test('dotted channel list aliases resolve to channels_list', () => {
      expect(normalizeRpcMethod('channels.list')).toBe(CORE_RPC_METHODS.channelsList);
      expect(normalizeRpcMethod('openhuman.channels.list')).toBe(CORE_RPC_METHODS.channelsList);
    });

    test('canonical channels_list passes through unchanged', () => {
      expect(normalizeRpcMethod('openhuman.channels_list')).toBe('openhuman.channels_list');
    });
  });

  test('catalog canonical methods exist in core schema registry (drift guard)', () => {
    // Discovery, not a hand-written path list.
    //
    // This used to read ten hardcoded `schemas.rs` paths. That broke on
    // 2026-08-30: the include! split (#5856/#5857) turned several of them into
    // shells that `#[path = "..._part_NN.rs"] mod ...;` their contents, so the
    // guard was reading 29 lines of module declarations where the controllers
    // used to be. It failed on `openhuman.config_get_agent_paths` — correctly,
    // but for the wrong reason: the method exists, the corpus had shrunk. Any
    // method whose function name still happened to appear somewhere in the
    // surviving nine files kept passing, so the shrink was mostly silent.
    //
    // Walking for `ControllerSchema` literals cannot go stale the same way: a
    // declaration that moves to another file is still found, and a declaration
    // that is genuinely deleted still fails.
    const repoRoot = path.resolve(__dirname, '../../../..');
    const schemaRoots = [
      path.join(repoRoot, 'src', 'openhuman'),
      // The channels_* namespace/function literals live in the vendored
      // tinychannels workspace as `ChannelControllerSchema`, not in the thin
      // `src/openhuman/channels/controllers/schemas.rs` adapter, which only
      // converts from it with a dynamic `namespace: schema.namespace` no static
      // scan can read (#4557). Controller metadata is contract, so it lives in
      // the `tinychannels-bus` crate rather than the implementation crate.
      path.join(repoRoot, 'vendor/tinychannels/crates/tinychannels-bus/src/controllers'),
    ];

    const rustFilesIn = (dir: string, out: string[] = []): string[] => {
      if (!fs.existsSync(dir)) return out;
      for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
        const full = path.join(dir, entry.name);
        if (entry.isDirectory()) rustFilesIn(full, out);
        else if (full.endsWith('.rs')) out.push(full);
      }
      return out;
    };

    const declared = new Set<string>();
    for (const root of schemaRoots) {
      // A missing root is a broken guard, not a passing one — the same reason
      // the previous readFileSync list was deliberately allowed to throw.
      expect(fs.existsSync(root), `schema root missing: ${root}`).toBe(true);

      for (const file of rustFilesIn(root)) {
        const text = fs.readFileSync(file, 'utf8');
        const constNamespace = text.match(/const\s+NAMESPACE:\s*&str\s*=\s*"([a-z_]+)"/)?.[1];
        for (const match of text.matchAll(/(?:Channel)?ControllerSchema\s*\{([\s\S]*?)\n\s*\}/g)) {
          const block = match[1];
          const namespaceToken = block.match(/namespace:\s*(?:NAMESPACE|"([a-z_]+)")/);
          const fnName = block.match(/function:\s*"([A-Za-z0-9_]+)"/)?.[1];
          const namespace = namespaceToken?.[1] ?? (namespaceToken ? constNamespace : undefined);
          if (!namespace || !fnName || fnName === 'unknown') continue;
          declared.add(`openhuman.${namespace}_${fnName}`);
        }
      }
    }

    // Sanity floor: if discovery silently returned almost nothing, every
    // `toContain` below would still pass on a lucky substring. Assert the
    // corpus is the size we expect before trusting a single result from it.
    expect(declared.size).toBeGreaterThan(400);

    for (const method of Object.values(CORE_RPC_METHODS)) {
      // core.* methods (e.g. core.ping) are special dispatch methods, not in
      // the schema catalog.
      if (!method.startsWith('openhuman.')) continue;

      // Exact pairing. The previous version asserted `namespace: "x"` and
      // `function: "y"` as two INDEPENDENT substrings of one concatenated blob,
      // so `openhuman.config_get` passed if any file declared namespace
      // "config" and any other file anywhere declared function "get" — under a
      // different namespace, in a different domain. Function names like `get`,
      // `list`, `status` and `update` are shared across dozens of namespaces,
      // so a deleted controller was very likely to keep passing.
      expect(declared, `catalog method not declared by any ControllerSchema: ${method}`).toContain(
        method
      );
    }
  });
});
