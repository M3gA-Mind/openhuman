import { fireEvent, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { renderWithProviders } from '../../../../test/test-utils';

// [dev-workflow] Unit tests for DevWorkflowPanel.tsx — covers repo loading,
// not-connected error, fork detection, branch population, and cron job wiring.

const hoisted = vi.hoisted(() => ({
  composioExecute: vi.fn(),
  listConnections: vi.fn(),
  cronAdd: vi.fn(),
  cronList: vi.fn(),
  cronRemove: vi.fn(),
  cronUpdate: vi.fn(),
  cronRun: vi.fn(),
  cronRuns: vi.fn(),
}));

vi.mock('../../../../lib/composio/composioApi', () => ({
  execute: hoisted.composioExecute,
  listConnections: hoisted.listConnections,
}));

vi.mock('../../../../utils/tauriCommands/cron', () => ({
  openhumanCronAdd: hoisted.cronAdd,
  openhumanCronList: hoisted.cronList,
  openhumanCronRemove: hoisted.cronRemove,
  openhumanCronUpdate: hoisted.cronUpdate,
  openhumanCronRun: hoisted.cronRun,
  openhumanCronRuns: hoisted.cronRuns,
}));

// Stable t function — creating a new function object on every render
// would cause useCallback([t]) to re-create on every render, triggering
// the loadRepos useEffect in an infinite loop.
const stableT = (key: string) => key;
vi.mock('../../../../lib/i18n/I18nContext', () => ({ useT: () => ({ t: stableT }) }));

vi.mock('../../hooks/useSettingsNavigation', () => ({
  useSettingsNavigation: () => ({
    navigateBack: vi.fn(),
    navigateToSettings: vi.fn(),
    breadcrumbs: [],
  }),
}));

vi.mock('../../components/SettingsHeader', () => ({
  default: ({ title }: { title: string }) => <div data-testid="settings-header">{title}</div>,
}));

// Import once — DevWorkflowPanel state is managed via API mocks and
// cron RPC, not module-level vars, so a single import is sufficient.
async function importPanel() {
  const mod = await import('../DevWorkflowPanel');
  return mod.default;
}

// ── Mock data ─────────────────────────────────────────────────────────────────

const githubConnection = { connections: [{ id: 'conn-1', toolkit: 'github', status: 'ACTIVE' }] };

const reposResponse = {
  successful: true,
  data: [
    { full_name: 'user/repo1', name: 'repo1', owner: { login: 'user' }, private: false },
    { full_name: 'user/repo2', name: 'repo2', owner: { login: 'user' }, fork: true, private: true },
  ],
  error: null,
  costUsd: 0,
};

const repoMetaNonFork = {
  successful: true,
  data: { fork: false, default_branch: 'main' },
  error: null,
  costUsd: 0,
};

const repoMetaFork = {
  successful: true,
  data: {
    fork: true,
    parent: { full_name: 'upstream/repo', owner: { login: 'upstream' }, name: 'repo' },
    default_branch: 'main',
  },
  error: null,
  costUsd: 0,
};

const branchesResponse = {
  successful: true,
  data: { details: [{ name: 'main' }, { name: 'dev' }] },
  error: null,
  costUsd: 0,
};

// ── Tests ─────────────────────────────────────────────────────────────────────

describe('DevWorkflowPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    hoisted.listConnections.mockResolvedValue(githubConnection);
    hoisted.composioExecute.mockResolvedValue(reposResponse);
    hoisted.cronList.mockResolvedValue({ data: [] });
    hoisted.cronAdd.mockResolvedValue({ data: { id: 'cron-1', name: 'dev-workflow-user-repo1' } });
    hoisted.cronRemove.mockResolvedValue({ data: { job_id: 'cron-1', removed: true } });
    hoisted.cronRuns.mockResolvedValue({ data: [] });
  });

  test('renders header immediately and populates repo dropdown on successful fetch', async () => {
    const Panel = await importPanel();
    renderWithProviders(<Panel />);

    // Header is rendered synchronously
    expect(screen.getByTestId('settings-header')).toBeInTheDocument();

    // Wait for repos to load
    await waitFor(() => {
      expect(screen.getByRole('option', { name: /user\/repo1/ })).toBeInTheDocument();
    });
    expect(screen.getByRole('option', { name: /user\/repo2/ })).toBeInTheDocument();

    expect(hoisted.composioExecute).toHaveBeenCalledWith(
      'GITHUB_LIST_REPOSITORIES_FOR_THE_AUTHENTICATED_USER',
      {}
    );
  });

  test('shows not-connected error when no GitHub connection found', async () => {
    hoisted.listConnections.mockResolvedValue({ connections: [] });
    const Panel = await importPanel();
    renderWithProviders(<Panel />);

    await waitFor(() => {
      expect(screen.getByText('settings.devWorkflow.errorNotConnected')).toBeInTheDocument();
    });
    // composioExecute should not be called if not connected
    expect(hoisted.composioExecute).not.toHaveBeenCalled();
  });

  test('shows not-connected error when connections list is missing', async () => {
    hoisted.listConnections.mockResolvedValue({});
    const Panel = await importPanel();
    renderWithProviders(<Panel />);

    await waitFor(() => {
      expect(screen.getByText('settings.devWorkflow.errorNotConnected')).toBeInTheDocument();
    });
  });

  test('detects fork and shows upstream info after repo selection', async () => {
    // Call sequence: LIST_REPOS → GET_A_REPO (fork) → LIST_BRANCHES
    hoisted.composioExecute
      .mockResolvedValueOnce(reposResponse)
      .mockResolvedValueOnce(repoMetaFork)
      .mockResolvedValueOnce(branchesResponse);

    const Panel = await importPanel();
    renderWithProviders(<Panel />);

    // Wait for repos to appear
    await waitFor(() => {
      expect(screen.getByRole('option', { name: /user\/repo1/ })).toBeInTheDocument();
    });

    // Select a repo
    const select = screen.getAllByRole('combobox')[0];
    fireEvent.change(select, { target: { value: 'user/repo1' } });

    // Fork info should appear
    await waitFor(() => {
      expect(screen.getByText('settings.devWorkflow.forkDetected')).toBeInTheDocument();
    });
    expect(screen.getByText('upstream/repo')).toBeInTheDocument();
  });

  test('shows branches in dropdown after repo selection', async () => {
    // Call sequence: LIST_REPOS → GET_A_REPO (non-fork) → LIST_BRANCHES
    hoisted.composioExecute
      .mockResolvedValueOnce(reposResponse)
      .mockResolvedValueOnce(repoMetaNonFork)
      .mockResolvedValueOnce(branchesResponse);

    const Panel = await importPanel();
    renderWithProviders(<Panel />);

    await waitFor(() => {
      expect(screen.getByRole('option', { name: /user\/repo1/ })).toBeInTheDocument();
    });

    const repoSelect = screen.getAllByRole('combobox')[0];
    fireEvent.change(repoSelect, { target: { value: 'user/repo1' } });

    await waitFor(() => {
      expect(screen.getByRole('option', { name: 'main' })).toBeInTheDocument();
    });
    expect(screen.getByRole('option', { name: 'dev' })).toBeInTheDocument();

    expect(hoisted.composioExecute).toHaveBeenCalledWith('GITHUB_LIST_BRANCHES', {
      owner: 'user',
      repo: 'repo1',
      per_page: 100,
    });
  });

  test('save button creates a cron job via openhumanCronAdd', async () => {
    // Call sequence: LIST_REPOS → GET_A_REPO (non-fork) → LIST_BRANCHES
    hoisted.composioExecute
      .mockResolvedValueOnce(reposResponse)
      .mockResolvedValueOnce(repoMetaNonFork)
      .mockResolvedValueOnce(branchesResponse);

    const Panel = await importPanel();
    renderWithProviders(<Panel />);

    // Wait for repos
    await waitFor(() => {
      expect(screen.getByRole('option', { name: /user\/repo1/ })).toBeInTheDocument();
    });

    // Select repo
    const repoSelect = screen.getAllByRole('combobox')[0];
    fireEvent.change(repoSelect, { target: { value: 'user/repo1' } });

    // Wait for branches
    await waitFor(() => {
      expect(screen.getByRole('option', { name: 'main' })).toBeInTheDocument();
    });

    // Click save
    const saveBtn = screen.getByRole('button', {
      name: /settings\.devWorkflow\.(save|update)Configuration/,
    });
    fireEvent.click(saveBtn);

    // Verify cron_add was called
    await waitFor(() => {
      expect(hoisted.cronAdd).toHaveBeenCalledTimes(1);
    });
    const addCall = hoisted.cronAdd.mock.calls[0][0];
    expect(addCall.name).toBe('dev-workflow-user-repo1');
    expect(addCall.schedule).toEqual({ kind: 'cron', expr: '*/30 * * * *' });
    expect(addCall.job_type).toBe('agent');
    expect(addCall.prompt).toContain('dev-workflow');
    expect(addCall.prompt).toContain('user/repo1');
  });

  test('remove button deletes cron job via openhumanCronRemove', async () => {
    // Pre-populate cron list so existingJob is set on mount
    const existingCronJob = {
      id: 'cron-1',
      name: 'dev-workflow-user-repo1',
      expression: '*/30 * * * *',
      schedule: { kind: 'cron', expr: '*/30 * * * *' },
      command: '',
      prompt: 'Run the dev-workflow skill.',
      job_type: 'agent',
      session_target: 'isolated',
      enabled: true,
      delivery: { mode: 'proactive', best_effort: true },
      delete_after_run: false,
      created_at: '2026-01-01T00:00:00Z',
      next_run: '2026-01-01T01:00:00Z',
    };
    hoisted.cronList.mockResolvedValue({ data: [existingCronJob] });
    // Call sequence: LIST_REPOS → GET_A_REPO (non-fork) → LIST_BRANCHES
    hoisted.composioExecute
      .mockResolvedValueOnce(reposResponse)
      .mockResolvedValueOnce(repoMetaNonFork)
      .mockResolvedValueOnce(branchesResponse);

    const Panel = await importPanel();
    renderWithProviders(<Panel />);

    // Wait for repos to load
    await waitFor(() => {
      expect(screen.getByRole('option', { name: /user\/repo1/ })).toBeInTheDocument();
    });

    // Select a repo so the Actions section (with remove button) renders
    const repoSelect = screen.getAllByRole('combobox')[0];
    fireEvent.change(repoSelect, { target: { value: 'user/repo1' } });

    // Wait for active config summary + remove button
    await waitFor(() => {
      expect(screen.getByText('settings.devWorkflow.activeConfiguration')).toBeInTheDocument();
    });

    const removeBtn = screen.getByRole('button', { name: 'settings.devWorkflow.remove' });
    fireEvent.click(removeBtn);

    // Verify cron_remove was called
    await waitFor(() => {
      expect(hoisted.cronRemove).toHaveBeenCalledWith('cron-1');
    });
  });

  test('shows branches fetched from upstream when fork is detected', async () => {
    // Call sequence: LIST_REPOS → GET_A_REPO (fork) → LIST_BRANCHES on upstream
    hoisted.composioExecute
      .mockResolvedValueOnce(reposResponse)
      .mockResolvedValueOnce(repoMetaFork)
      .mockResolvedValueOnce(branchesResponse);

    const Panel = await importPanel();
    renderWithProviders(<Panel />);

    await waitFor(() => {
      expect(screen.getByRole('option', { name: /user\/repo1/ })).toBeInTheDocument();
    });

    const repoSelect = screen.getAllByRole('combobox')[0];
    fireEvent.change(repoSelect, { target: { value: 'user/repo1' } });

    await waitFor(() => {
      expect(screen.getByRole('option', { name: 'main' })).toBeInTheDocument();
    });

    // Branches were fetched from upstream owner/repo
    expect(hoisted.composioExecute).toHaveBeenCalledWith('GITHUB_LIST_BRANCHES', {
      owner: 'upstream',
      repo: 'repo',
      per_page: 100,
    });
  });

  test('panel still renders if listConnections rejects', async () => {
    hoisted.listConnections.mockRejectedValue(new Error('network error'));
    const Panel = await importPanel();
    renderWithProviders(<Panel />);

    // Header always renders
    expect(screen.getByTestId('settings-header')).toBeInTheDocument();

    // Error state shown
    await waitFor(() => {
      expect(screen.getByText('network error')).toBeInTheDocument();
    });
  });
});
