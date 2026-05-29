import { beforeEach, describe, expect, it, vi } from 'vitest';

import { skillsApi } from '../skillsApi';

vi.mock('../../coreRpcClient', () => ({ callCoreRpc: vi.fn() }));

describe('skillsApi.createSkill', () => {
  beforeEach(async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockReset();
  });

  it('forwards inputs to skills_create and rekeys allowedTools', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockResolvedValueOnce({
      skill: {
        id: 'my-skill',
        name: 'my-skill',
        description: 'does stuff',
        version: '',
        author: null,
        tags: ['alpha'],
        tools: ['mcp/fs'],
        prompts: [],
        location: '/home/u/.openhuman/skills/my-skill/SKILL.md',
        resources: [],
        scope: 'user',
        legacy: false,
        warnings: [],
      },
    });

    const result = await skillsApi.createSkill({
      name: 'My Skill',
      description: 'does stuff',
      scope: 'user',
      tags: ['alpha'],
      allowedTools: ['mcp/fs'],
    });

    expect(callCoreRpc).toHaveBeenCalledWith({
      method: 'openhuman.skills_create',
      params: {
        name: 'My Skill',
        description: 'does stuff',
        scope: 'user',
        tags: ['alpha'],
        'allowed-tools': ['mcp/fs'],
      },
    });
    expect(result.id).toBe('my-skill');
    expect(result.scope).toBe('user');
  });

  it('omits optional fields when not provided', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockResolvedValueOnce({
      skill: {
        id: 'minimal',
        name: 'minimal',
        description: 'd',
        version: '',
        author: null,
        tags: [],
        tools: [],
        prompts: [],
        location: null,
        resources: [],
        scope: 'user',
        legacy: false,
        warnings: [],
      },
    });

    await skillsApi.createSkill({ name: 'minimal', description: 'd' });

    const call = vi.mocked(callCoreRpc).mock.calls[0][0];
    expect(call.params).toEqual({ name: 'minimal', description: 'd' });
  });

  it('unwraps an envelope response', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockResolvedValueOnce({
      data: {
        skill: {
          id: 'env',
          name: 'env',
          description: 'e',
          version: '',
          author: null,
          tags: [],
          tools: [],
          prompts: [],
          location: null,
          resources: [],
          scope: 'project',
          legacy: false,
          warnings: [],
        },
      },
    });
    const result = await skillsApi.createSkill({ name: 'env', description: 'e' });
    expect(result.id).toBe('env');
    expect(result.scope).toBe('project');
  });
});

describe('skillsApi.installSkillFromUrl', () => {
  beforeEach(async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockReset();
  });

  it('forwards url and rekeys timeoutSecs to timeout_secs', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockResolvedValueOnce({
      url: 'https://example.com/my-skill.tgz',
      stdout: 'added my-skill',
      stderr: '',
      new_skills: ['my-skill'],
    });

    const result = await skillsApi.installSkillFromUrl({
      url: 'https://example.com/my-skill.tgz',
      timeoutSecs: 120,
    });

    expect(callCoreRpc).toHaveBeenCalledWith({
      method: 'openhuman.skills_install_from_url',
      params: { url: 'https://example.com/my-skill.tgz', timeout_secs: 120 },
    });
    expect(result.newSkills).toEqual(['my-skill']);
    expect(result.stdout).toBe('added my-skill');
  });

  it('omits timeout_secs when not provided and normalizes missing new_skills', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockResolvedValueOnce({
      url: 'https://example.com/x',
      stdout: '',
      stderr: '',
      new_skills: undefined,
    });

    const result = await skillsApi.installSkillFromUrl({ url: 'https://example.com/x' });

    const call = vi.mocked(callCoreRpc).mock.calls[0][0];
    expect(call.params).toEqual({ url: 'https://example.com/x' });
    expect(result.newSkills).toEqual([]);
  });

  it('unwraps an envelope response', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockResolvedValueOnce({
      data: { url: 'https://example.com/y', stdout: 'ok', stderr: 'warn', new_skills: ['y-skill'] },
    });
    const result = await skillsApi.installSkillFromUrl({ url: 'https://example.com/y' });
    expect(result.newSkills).toEqual(['y-skill']);
    expect(result.stderr).toBe('warn');
  });
});

describe('skillsApi.describeSkill', () => {
  beforeEach(async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockReset();
  });

  it('calls skills_describe with skill_id and returns the response', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    const mockDescription = {
      id: 'my-skill',
      display_name: 'My Skill',
      when_to_use: 'When you need to do stuff',
      inputs: [
        { name: 'input1', description: 'First input', required: true, type: 'string' },
      ],
    };
    vi.mocked(callCoreRpc).mockResolvedValueOnce(mockDescription);

    const result = await skillsApi.describeSkill('my-skill');

    expect(callCoreRpc).toHaveBeenCalledWith({
      method: 'openhuman.skills_describe',
      params: { skill_id: 'my-skill' },
    });
    expect(result.id).toBe('my-skill');
    expect(result.inputs).toHaveLength(1);
    expect(result.inputs[0].name).toBe('input1');
  });

  it('unwraps an envelope response', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    const mockDescription = {
      id: 'env-skill',
      display_name: 'Env Skill',
      when_to_use: 'Always',
      inputs: [],
    };
    vi.mocked(callCoreRpc).mockResolvedValueOnce({ data: mockDescription });

    const result = await skillsApi.describeSkill('env-skill');
    expect(result.id).toBe('env-skill');
    expect(result.inputs).toHaveLength(0);
  });

  it('returns a skill with multiple inputs', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockResolvedValueOnce({
      id: 'multi-input-skill',
      display_name: 'Multi Input Skill',
      when_to_use: 'When needed',
      inputs: [
        { name: 'name', description: 'A name', required: true, type: 'string' },
        { name: 'count', description: 'A count', required: false, type: 'integer' },
        { name: 'verbose', description: 'Verbosity', required: false, type: 'boolean' },
      ],
    });

    const result = await skillsApi.describeSkill('multi-input-skill');
    expect(result.inputs).toHaveLength(3);
    expect(result.inputs[1].type).toBe('integer');
  });
});

describe('skillsApi.runSkill', () => {
  beforeEach(async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockReset();
  });

  it('calls skills_run with skill_id and inputs, returns run start info', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    const mockRunStarted = {
      run_id: 'run-abc123',
      status: 'started',
      skill_id: 'my-skill',
      log: '/home/u/.openhuman/skills/.runs/run-abc123.log',
    };
    vi.mocked(callCoreRpc).mockResolvedValueOnce(mockRunStarted);

    const result = await skillsApi.runSkill('my-skill', { input1: 'hello' });

    expect(callCoreRpc).toHaveBeenCalledWith({
      method: 'openhuman.skills_run',
      params: { skill_id: 'my-skill', inputs: { input1: 'hello' } },
    });
    expect(result.run_id).toBe('run-abc123');
    expect(result.status).toBe('started');
    expect(result.log).toContain('run-abc123');
  });

  it('passes empty inputs object when no inputs provided', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockResolvedValueOnce({
      run_id: 'run-xyz',
      status: 'started',
      skill_id: 'simple-skill',
      log: '/tmp/run-xyz.log',
    });

    await skillsApi.runSkill('simple-skill', {});

    const call = vi.mocked(callCoreRpc).mock.calls[0][0];
    expect(call.params).toEqual({ skill_id: 'simple-skill', inputs: {} });
  });

  it('unwraps an envelope response', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockResolvedValueOnce({
      data: {
        run_id: 'env-run-1',
        status: 'started',
        skill_id: 'env-skill',
        log: '/tmp/env-run-1.log',
      },
    });

    const result = await skillsApi.runSkill('env-skill', { key: 'value' });
    expect(result.run_id).toBe('env-run-1');
    expect(result.skill_id).toBe('env-skill');
  });
});

describe('skillsApi.readRunLog', () => {
  beforeEach(async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockReset();
  });

  it('calls skills_read_run_log with run_id only when offset/maxBytes omitted', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    const mockSlice = {
      offset: 512,
      bytes_read: 512,
      content: 'log line 1\nlog line 2\n',
      eof: false,
      complete: false,
    };
    vi.mocked(callCoreRpc).mockResolvedValueOnce(mockSlice);

    const result = await skillsApi.readRunLog('run-abc123');

    expect(callCoreRpc).toHaveBeenCalledWith({
      method: 'openhuman.skills_read_run_log',
      params: { run_id: 'run-abc123' },
    });
    expect(result.offset).toBe(512);
    expect(result.complete).toBe(false);
    expect(result.content).toContain('log line 1');
  });

  it('includes offset and max_bytes when provided', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockResolvedValueOnce({
      offset: 1024,
      bytes_read: 256,
      content: 'more log\n',
      eof: false,
      complete: false,
    });

    await skillsApi.readRunLog('run-abc123', 512, 256);

    expect(callCoreRpc).toHaveBeenCalledWith({
      method: 'openhuman.skills_read_run_log',
      params: { run_id: 'run-abc123', offset: 512, max_bytes: 256 },
    });
  });

  it('returns complete=true and eof=true at end of run', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockResolvedValueOnce({
      offset: 2048,
      bytes_read: 64,
      content: '--- result ---\nDONE\n',
      eof: true,
      complete: true,
    });

    const result = await skillsApi.readRunLog('run-done');
    expect(result.complete).toBe(true);
    expect(result.eof).toBe(true);
  });

  it('unwraps an envelope response', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockResolvedValueOnce({
      data: {
        offset: 100,
        bytes_read: 100,
        content: 'hello\n',
        eof: false,
        complete: false,
      },
    });

    const result = await skillsApi.readRunLog('run-env', 0);
    expect(result.bytes_read).toBe(100);
  });
});

describe('skillsApi.recentRuns', () => {
  beforeEach(async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockReset();
  });

  it('calls skills_recent_runs with no params when skillId/limit omitted', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockResolvedValueOnce({
      runs: [
        {
          run_id: 'run-1',
          skill_id: 'skill-a',
          started: '2026-01-01T00:00:00Z',
          status: 'DONE',
          duration_ms: 5000,
          finished: '2026-01-01T00:00:05Z',
          log_path: '/tmp/run-1.log',
        },
      ],
    });

    const result = await skillsApi.recentRuns();

    expect(callCoreRpc).toHaveBeenCalledWith({
      method: 'openhuman.skills_recent_runs',
      params: {},
    });
    expect(result).toHaveLength(1);
    expect(result[0].run_id).toBe('run-1');
  });

  it('includes skill_id when skillId is provided', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockResolvedValueOnce({ runs: [] });

    await skillsApi.recentRuns('my-skill');

    const call = vi.mocked(callCoreRpc).mock.calls[0][0];
    expect(call.params).toEqual({ skill_id: 'my-skill' });
  });

  it('includes limit when provided', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockResolvedValueOnce({ runs: [] });

    await skillsApi.recentRuns(undefined, 5);

    const call = vi.mocked(callCoreRpc).mock.calls[0][0];
    expect(call.params).toEqual({ limit: 5 });
  });

  it('includes both skill_id and limit when both are provided', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockResolvedValueOnce({ runs: [] });

    await skillsApi.recentRuns('filtered-skill', 10);

    const call = vi.mocked(callCoreRpc).mock.calls[0][0];
    expect(call.params).toEqual({ skill_id: 'filtered-skill', limit: 10 });
  });

  it('unwraps an envelope response', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockResolvedValueOnce({
      data: {
        runs: [
          {
            run_id: 'env-run',
            skill_id: 'env-skill',
            started: '2026-01-01T00:00:00Z',
            status: 'RUNNING',
            duration_ms: null,
            finished: null,
            log_path: '/tmp/env-run.log',
          },
        ],
      },
    });

    const result = await skillsApi.recentRuns();
    expect(result).toHaveLength(1);
    expect(result[0].status).toBe('RUNNING');
    expect(result[0].duration_ms).toBeNull();
  });

  it('returns empty array when runs is empty', async () => {
    const { callCoreRpc } = await import('../../coreRpcClient');
    vi.mocked(callCoreRpc).mockResolvedValueOnce({ runs: [] });

    const result = await skillsApi.recentRuns('empty-skill');
    expect(result).toEqual([]);
  });
});
