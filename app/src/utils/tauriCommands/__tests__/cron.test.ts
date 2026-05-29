// Tests for cron Tauri commands.
// Covers the isTauri guard and the core RPC forwarding for each function.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { callCoreRpc } from '../../../services/coreRpcClient';
import {
  openhumanCronAdd,
  openhumanCronList,
  openhumanCronRemove,
  openhumanCronRun,
  openhumanCronRuns,
  openhumanCronUpdate,
} from '../cron';

vi.mock('../../../services/coreRpcClient', () => ({ callCoreRpc: vi.fn() }));
vi.mock('../common', () => ({ isTauri: vi.fn(() => true), CommandResponse: undefined }));

describe('openhumanCronAdd', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it('throws when not running inside Tauri', async () => {
    const { isTauri } = await import('../common');
    vi.mocked(isTauri).mockReturnValueOnce(false);

    await expect(
      openhumanCronAdd({ schedule: { kind: 'cron', expr: '*/30 * * * *' } })
    ).rejects.toThrow('Not running in Tauri');
  });

  it('calls cron_add with the provided params', async () => {
    const expected = {
      result: {
        id: 'cron-1',
        name: 'my-job',
        expression: '*/30 * * * *',
        schedule: { kind: 'cron', expr: '*/30 * * * *' },
        command: '',
        job_type: 'agent',
        session_target: 'isolated',
        enabled: true,
        delivery: { mode: 'proactive', best_effort: true },
        delete_after_run: false,
        created_at: '2026-01-01T00:00:00Z',
        next_run: '2026-01-01T01:00:00Z',
      },
      logs: [],
    };
    vi.mocked(callCoreRpc).mockResolvedValueOnce(expected);

    const params = {
      name: 'my-job',
      schedule: { kind: 'cron' as const, expr: '*/30 * * * *' },
      job_type: 'agent' as const,
    };
    const result = await openhumanCronAdd(params);

    expect(callCoreRpc).toHaveBeenCalledWith({ method: 'openhuman.cron_add', params });
    expect(result).toEqual(expected);
  });

  it('forwards all optional params to the RPC call', async () => {
    vi.mocked(callCoreRpc).mockResolvedValueOnce({ result: {}, logs: [] });

    const params = {
      name: 'full-job',
      schedule: { kind: 'every' as const, every_ms: 60000 },
      job_type: 'shell' as const,
      command: 'echo hello',
      session_target: 'main' as const,
      delete_after_run: true,
    };
    await openhumanCronAdd(params);

    expect(callCoreRpc).toHaveBeenCalledWith({ method: 'openhuman.cron_add', params });
  });
});

describe('openhumanCronList', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('throws when not running inside Tauri', async () => {
    const { isTauri } = await import('../common');
    vi.mocked(isTauri).mockReturnValueOnce(false);

    await expect(openhumanCronList()).rejects.toThrow('Not running in Tauri');
  });

  it('calls cron_list with no params', async () => {
    const expected = { result: [], logs: [] };
    vi.mocked(callCoreRpc).mockResolvedValueOnce(expected);

    const result = await openhumanCronList();

    expect(callCoreRpc).toHaveBeenCalledWith({ method: 'openhuman.cron_list' });
    expect(result).toEqual(expected);
  });
});

describe('openhumanCronUpdate', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('throws when not running inside Tauri', async () => {
    const { isTauri } = await import('../common');
    vi.mocked(isTauri).mockReturnValueOnce(false);

    await expect(openhumanCronUpdate('job-1', { enabled: false })).rejects.toThrow(
      'Not running in Tauri'
    );
  });

  it('calls cron_update with job_id and patch', async () => {
    vi.mocked(callCoreRpc).mockResolvedValueOnce({ result: {}, logs: [] });

    await openhumanCronUpdate('job-1', { enabled: false });

    expect(callCoreRpc).toHaveBeenCalledWith({
      method: 'openhuman.cron_update',
      params: { job_id: 'job-1', patch: { enabled: false } },
    });
  });
});

describe('openhumanCronRemove', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('throws when not running inside Tauri', async () => {
    const { isTauri } = await import('../common');
    vi.mocked(isTauri).mockReturnValueOnce(false);

    await expect(openhumanCronRemove('job-1')).rejects.toThrow('Not running in Tauri');
  });

  it('calls cron_remove with job_id', async () => {
    const expected = { result: { job_id: 'job-1', removed: true }, logs: [] };
    vi.mocked(callCoreRpc).mockResolvedValueOnce(expected);

    const result = await openhumanCronRemove('job-1');

    expect(callCoreRpc).toHaveBeenCalledWith({
      method: 'openhuman.cron_remove',
      params: { job_id: 'job-1' },
    });
    expect(result).toEqual(expected);
  });
});

describe('openhumanCronRun', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('throws when not running inside Tauri', async () => {
    const { isTauri } = await import('../common');
    vi.mocked(isTauri).mockReturnValueOnce(false);

    await expect(openhumanCronRun('job-1')).rejects.toThrow('Not running in Tauri');
  });

  it('calls cron_run with job_id', async () => {
    const expected = {
      result: { job_id: 'job-1', status: 'ok', duration_ms: 1500, output: 'done' },
      logs: [],
    };
    vi.mocked(callCoreRpc).mockResolvedValueOnce(expected);

    const result = await openhumanCronRun('job-1');

    expect(callCoreRpc).toHaveBeenCalledWith({
      method: 'openhuman.cron_run',
      params: { job_id: 'job-1' },
    });
    expect(result).toEqual(expected);
  });
});

describe('openhumanCronRuns', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('throws when not running inside Tauri', async () => {
    const { isTauri } = await import('../common');
    vi.mocked(isTauri).mockReturnValueOnce(false);

    await expect(openhumanCronRuns('job-1')).rejects.toThrow('Not running in Tauri');
  });

  it('calls cron_runs with job_id and default limit of 20', async () => {
    const expected = { result: { runs: [] }, logs: [] };
    vi.mocked(callCoreRpc).mockResolvedValueOnce(expected);

    await openhumanCronRuns('job-1');

    expect(callCoreRpc).toHaveBeenCalledWith({
      method: 'openhuman.cron_runs',
      params: { job_id: 'job-1', limit: 20 },
    });
  });

  it('passes custom limit to cron_runs', async () => {
    vi.mocked(callCoreRpc).mockResolvedValueOnce({ result: { runs: [] }, logs: [] });

    await openhumanCronRuns('job-1', 5);

    expect(callCoreRpc).toHaveBeenCalledWith({
      method: 'openhuman.cron_runs',
      params: { job_id: 'job-1', limit: 5 },
    });
  });
});
