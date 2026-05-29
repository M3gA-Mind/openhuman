/**
 * SmartIssuePicker — unit tests
 *
 * Covers: initial render, repo loading, not-connected error, repo
 * options rendered, repo select triggering fork detection + branch
 * list, fork banner shown when fork detected, branch select dropdown,
 * onPatchInputs callbacks.
 */
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const stableT = (key: string) => key;
vi.mock('../../../lib/i18n/I18nContext', () => ({ useT: () => ({ t: stableT }) }));

const hoisted = vi.hoisted(() => ({
  execute: vi.fn(),
  listConnections: vi.fn(),
}));

vi.mock('../../../lib/composio/composioApi', () => ({
  execute: hoisted.execute,
  listConnections: hoisted.listConnections,
}));

async function importSmartIssuePicker() {
  const mod = await import('../SmartIssuePicker');
  return mod.default;
}

// Helpers
const ghConnection = {
  connections: [{ id: 'conn-1', toolkit: 'github', status: 'ACTIVE' }],
};

function repoListRes(repos: { full_name: string; name: string; owner: { login: string } }[]) {
  return { successful: true, data: repos, error: null, costUsd: 0 };
}

function repoMetaRes(fork: boolean, defaultBranch = 'main', parent?: { full_name: string; owner: { login: string }; name: string }) {
  return {
    successful: true,
    data: { fork, default_branch: defaultBranch, ...(parent ? { parent } : {}) },
    error: null,
    costUsd: 0,
  };
}

function branchListRes(names: string[]) {
  return { successful: true, data: names.map((n) => ({ name: n })), error: null, costUsd: 0 };
}

describe('SmartIssuePicker', () => {
  beforeEach(() => {
    hoisted.execute.mockReset();
    hoisted.listConnections.mockReset();
  });

  it('renders the testid marker', async () => {
    hoisted.listConnections.mockResolvedValue(ghConnection);
    hoisted.execute.mockResolvedValue(repoListRes([]));
    const SmartIssuePicker = await importSmartIssuePicker();
    render(<SmartIssuePicker values={{}} onPatchInputs={vi.fn()} />);

    expect(screen.getByTestId('smart-issue-picker')).toBeInTheDocument();
  });

  it('shows repo options after successful load', async () => {
    hoisted.listConnections.mockResolvedValue(ghConnection);
    hoisted.execute.mockResolvedValue(
      repoListRes([
        { full_name: 'user/repo1', name: 'repo1', owner: { login: 'user' } },
        { full_name: 'user/repo2', name: 'repo2', owner: { login: 'user' } },
      ])
    );
    const SmartIssuePicker = await importSmartIssuePicker();
    render(<SmartIssuePicker values={{}} onPatchInputs={vi.fn()} />);

    await waitFor(() =>
      expect(screen.getByRole('option', { name: /user\/repo1/ })).toBeInTheDocument()
    );
    expect(screen.getByRole('option', { name: /user\/repo2/ })).toBeInTheDocument();
  });

  it('shows notConnected error when GitHub is not connected', async () => {
    hoisted.listConnections.mockResolvedValue({ connections: [] });
    const SmartIssuePicker = await importSmartIssuePicker();
    render(<SmartIssuePicker values={{}} onPatchInputs={vi.fn()} />);

    await waitFor(() =>
      expect(
        screen.getByText('settings.devWorkflow.errorNotConnected')
      ).toBeInTheDocument()
    );
    expect(hoisted.execute).not.toHaveBeenCalled();
  });

  it('shows error when repo list fetch fails', async () => {
    hoisted.listConnections.mockResolvedValue(ghConnection);
    hoisted.execute.mockRejectedValue(new Error('server down'));
    const SmartIssuePicker = await importSmartIssuePicker();
    render(<SmartIssuePicker values={{}} onPatchInputs={vi.fn()} />);

    await waitFor(() => expect(screen.getByText('server down')).toBeInTheDocument());
  });

  it('shows errorNoRepositories when list is empty', async () => {
    hoisted.listConnections.mockResolvedValue(ghConnection);
    hoisted.execute.mockResolvedValue(repoListRes([]));
    const SmartIssuePicker = await importSmartIssuePicker();
    render(<SmartIssuePicker values={{}} onPatchInputs={vi.fn()} />);

    await waitFor(() =>
      expect(
        screen.getByText('settings.devWorkflow.errorNoRepositories')
      ).toBeInTheDocument()
    );
  });

  it('calls onPatchInputs with initial repo fields when a repo is selected', async () => {
    hoisted.listConnections.mockResolvedValue(ghConnection);
    // First call: repo list; subsequent calls: repo meta + branch list
    hoisted.execute
      .mockResolvedValueOnce(
        repoListRes([{ full_name: 'user/myrepo', name: 'myrepo', owner: { login: 'user' } }])
      )
      .mockResolvedValueOnce(repoMetaRes(false, 'main'))
      .mockResolvedValueOnce(branchListRes(['main', 'dev']));

    const onPatchInputs = vi.fn();
    const SmartIssuePicker = await importSmartIssuePicker();
    render(<SmartIssuePicker values={{}} onPatchInputs={onPatchInputs} />);

    await waitFor(() =>
      expect(screen.getByRole('option', { name: /user\/myrepo/ })).toBeInTheDocument()
    );

    const repoSelect = screen.getByRole('combobox', { name: /settings\.devWorkflow\.githubRepository/ });
    fireEvent.change(repoSelect, { target: { value: 'user/myrepo' } });

    // The initial patch when repo first selected
    await waitFor(() =>
      expect(onPatchInputs).toHaveBeenCalledWith(
        expect.objectContaining({ repo: 'user/myrepo' })
      )
    );
  });

  it('does NOT show fork banner for a non-fork repo', async () => {
    hoisted.listConnections.mockResolvedValue(ghConnection);
    hoisted.execute
      .mockResolvedValueOnce(
        repoListRes([{ full_name: 'user/myrepo', name: 'myrepo', owner: { login: 'user' } }])
      )
      .mockResolvedValueOnce(repoMetaRes(false, 'main'))
      .mockResolvedValueOnce(branchListRes(['main']));

    const SmartIssuePicker = await importSmartIssuePicker();
    render(<SmartIssuePicker values={{}} onPatchInputs={vi.fn()} />);

    await waitFor(() =>
      expect(screen.getByRole('option', { name: /user\/myrepo/ })).toBeInTheDocument()
    );
    const repoSelect = screen.getAllByRole('combobox')[0];
    fireEvent.change(repoSelect, { target: { value: 'user/myrepo' } });

    // Wait for fork detection to complete
    await waitFor(() => expect(hoisted.execute).toHaveBeenCalledTimes(3));

    expect(screen.queryByTestId('smart-issue-picker-fork-banner')).not.toBeInTheDocument();
  });

  it('shows fork banner when a fork repo is selected', async () => {
    hoisted.listConnections.mockResolvedValue(ghConnection);
    hoisted.execute
      .mockResolvedValueOnce(
        repoListRes([{ full_name: 'user/fork-repo', name: 'fork-repo', owner: { login: 'user' } }])
      )
      .mockResolvedValueOnce(
        repoMetaRes(true, 'main', {
          full_name: 'upstream/original',
          owner: { login: 'upstream' },
          name: 'original',
        })
      )
      .mockResolvedValueOnce(branchListRes(['main', 'feature']));

    const onPatchInputs = vi.fn();
    const SmartIssuePicker = await importSmartIssuePicker();
    render(<SmartIssuePicker values={{}} onPatchInputs={onPatchInputs} />);

    await waitFor(() =>
      expect(screen.getByRole('option', { name: /user\/fork-repo/ })).toBeInTheDocument()
    );
    const repoSelect = screen.getAllByRole('combobox')[0];
    fireEvent.change(repoSelect, { target: { value: 'user/fork-repo' } });

    await waitFor(() =>
      expect(screen.getByTestId('smart-issue-picker-fork-banner')).toBeInTheDocument()
    );

    // Upstream info is shown in the banner
    expect(screen.getByText('upstream/original')).toBeInTheDocument();

    // onPatchInputs called with upstream info
    await waitFor(() =>
      expect(onPatchInputs).toHaveBeenCalledWith(
        expect.objectContaining({ upstream: 'upstream/original' })
      )
    );
  });

  it('renders branch dropdown after repo selection with branches', async () => {
    hoisted.listConnections.mockResolvedValue(ghConnection);
    hoisted.execute
      .mockResolvedValueOnce(
        repoListRes([{ full_name: 'user/myrepo', name: 'myrepo', owner: { login: 'user' } }])
      )
      .mockResolvedValueOnce(repoMetaRes(false, 'main'))
      .mockResolvedValueOnce(branchListRes(['main', 'develop', 'feature-x']));

    const SmartIssuePicker = await importSmartIssuePicker();
    render(<SmartIssuePicker values={{ repo: '' }} onPatchInputs={vi.fn()} />);

    await waitFor(() =>
      expect(screen.getByRole('option', { name: /user\/myrepo/ })).toBeInTheDocument()
    );
    fireEvent.change(screen.getAllByRole('combobox')[0], { target: { value: 'user/myrepo' } });

    // Branch dropdown appears
    await waitFor(() =>
      expect(screen.getByRole('option', { name: 'main' })).toBeInTheDocument()
    );
    expect(screen.getByRole('option', { name: 'develop' })).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'feature-x' })).toBeInTheDocument();
  });

  it('allows branch selection and calls onPatchInputs with target_branch', async () => {
    hoisted.listConnections.mockResolvedValue(ghConnection);
    hoisted.execute
      .mockResolvedValueOnce(
        repoListRes([{ full_name: 'user/myrepo', name: 'myrepo', owner: { login: 'user' } }])
      )
      .mockResolvedValueOnce(repoMetaRes(false, 'main'))
      .mockResolvedValueOnce(branchListRes(['main', 'develop']));

    const onPatchInputs = vi.fn();
    const SmartIssuePicker = await importSmartIssuePicker();
    render(<SmartIssuePicker values={{ repo: '' }} onPatchInputs={onPatchInputs} />);

    await waitFor(() =>
      expect(screen.getByRole('option', { name: /user\/myrepo/ })).toBeInTheDocument()
    );
    fireEvent.change(screen.getAllByRole('combobox')[0], { target: { value: 'user/myrepo' } });

    await waitFor(() =>
      expect(screen.getByRole('option', { name: 'develop' })).toBeInTheDocument()
    );

    const branchSelect = screen.getByLabelText('settings.devWorkflow.targetBranch');
    fireEvent.change(branchSelect, { target: { value: 'develop' } });

    expect(onPatchInputs).toHaveBeenCalledWith({ target_branch: 'develop' });
  });

  it('falls back to default branches when branch fetch fails', async () => {
    hoisted.listConnections.mockResolvedValue(ghConnection);
    hoisted.execute
      .mockResolvedValueOnce(
        repoListRes([{ full_name: 'user/myrepo', name: 'myrepo', owner: { login: 'user' } }])
      )
      .mockResolvedValueOnce(repoMetaRes(false, 'main'))
      .mockResolvedValueOnce({ successful: false, data: null, error: 'branch error', costUsd: 0 });

    const onPatchInputs = vi.fn();
    const SmartIssuePicker = await importSmartIssuePicker();
    render(<SmartIssuePicker values={{}} onPatchInputs={onPatchInputs} />);

    await waitFor(() =>
      expect(screen.getByRole('option', { name: /user\/myrepo/ })).toBeInTheDocument()
    );
    fireEvent.change(screen.getAllByRole('combobox')[0], { target: { value: 'user/myrepo' } });

    // Falls back to standard branches
    await waitFor(() =>
      expect(screen.getByRole('option', { name: 'main' })).toBeInTheDocument()
    );
    expect(screen.getByRole('option', { name: 'master' })).toBeInTheDocument();
  });
});
