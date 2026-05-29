/**
 * RepoPicker — unit tests
 *
 * Covers: loading state, repo options rendered from Composio result,
 * empty list (shows error), not-connected error, generic fetch error,
 * onChange callback, disabled prop passthrough.
 */
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const stableT = (key: string) => key;
vi.mock('../../../../lib/i18n/I18nContext', () => ({ useT: () => ({ t: stableT }) }));

const hoisted = vi.hoisted(() => ({
  execute: vi.fn(),
  listConnections: vi.fn(),
}));

vi.mock('../../../../lib/composio/composioApi', () => ({
  execute: hoisted.execute,
  listConnections: hoisted.listConnections,
}));

async function importRepoPicker() {
  const mod = await import('../RepoPicker');
  return mod.default;
}

// Shared helpers
const ghConnection = {
  connections: [{ id: 'conn-1', toolkit: 'github', status: 'ACTIVE' }],
};

function repoRes(repos: { full_name: string; name: string; owner: { login: string }; private?: boolean }[]) {
  return { successful: true, data: repos, error: null, costUsd: 0 };
}

describe('RepoPicker', () => {
  beforeEach(() => {
    hoisted.execute.mockReset();
    hoisted.listConnections.mockReset();
  });

  it('renders repo options after successful fetch', async () => {
    hoisted.listConnections.mockResolvedValue(ghConnection);
    hoisted.execute.mockResolvedValue(
      repoRes([
        { full_name: 'user/repo1', name: 'repo1', owner: { login: 'user' } },
        { full_name: 'user/repo2', name: 'repo2', owner: { login: 'user' } },
      ])
    );
    const RepoPicker = await importRepoPicker();
    render(<RepoPicker value="" onChange={vi.fn()} />);

    await waitFor(() =>
      expect(hoisted.execute).toHaveBeenCalledWith(
        'GITHUB_LIST_REPOSITORIES_FOR_THE_AUTHENTICATED_USER',
        {}
      )
    );

    await waitFor(() =>
      expect(screen.getByRole('option', { name: 'user/repo1' })).toBeInTheDocument()
    );
    expect(screen.getByRole('option', { name: 'user/repo2' })).toBeInTheDocument();
  });

  it('marks private repos with the privateTag label', async () => {
    hoisted.listConnections.mockResolvedValue(ghConnection);
    hoisted.execute.mockResolvedValue(
      repoRes([
        { full_name: 'user/private-repo', name: 'private-repo', owner: { login: 'user' }, private: true },
      ])
    );
    const RepoPicker = await importRepoPicker();
    render(<RepoPicker value="" onChange={vi.fn()} />);

    await waitFor(() =>
      expect(screen.getByRole('option', { name: /user\/private-repo/ })).toBeInTheDocument()
    );
    // The option text includes the privateTag i18n key
    const option = screen.getByRole('option', { name: /private-repo/ });
    expect(option.textContent).toContain('settings.skillsRunner.repoPicker.privateTag');
  });

  it('shows empty error when repo list is empty', async () => {
    hoisted.listConnections.mockResolvedValue(ghConnection);
    hoisted.execute.mockResolvedValue({ successful: true, data: [], error: null, costUsd: 0 });
    const RepoPicker = await importRepoPicker();
    render(<RepoPicker value="" onChange={vi.fn()} />);

    await waitFor(() => expect(hoisted.execute).toHaveBeenCalled());
    await waitFor(() =>
      expect(
        screen.getByText('settings.skillsRunner.repoPicker.empty')
      ).toBeInTheDocument()
    );
  });

  it('shows notConnected error when GitHub is not connected', async () => {
    hoisted.listConnections.mockResolvedValue({ connections: [] });
    const RepoPicker = await importRepoPicker();
    render(<RepoPicker value="" onChange={vi.fn()} />);

    await waitFor(() => expect(hoisted.listConnections).toHaveBeenCalled());
    await waitFor(() =>
      expect(
        screen.getByText('settings.skillsRunner.repoPicker.notConnected')
      ).toBeInTheDocument()
    );
    // Execute should NOT have been called since no connection exists
    expect(hoisted.execute).not.toHaveBeenCalled();
  });

  it('shows generic error when execute throws', async () => {
    hoisted.listConnections.mockResolvedValue(ghConnection);
    hoisted.execute.mockRejectedValue(new Error('fetch failed'));
    const RepoPicker = await importRepoPicker();
    render(<RepoPicker value="" onChange={vi.fn()} />);

    await waitFor(() => expect(screen.getByText('fetch failed')).toBeInTheDocument());
  });

  it('shows error when execute returns unsuccessful', async () => {
    hoisted.listConnections.mockResolvedValue(ghConnection);
    hoisted.execute.mockResolvedValue({
      successful: false,
      data: null,
      error: 'api error',
      costUsd: 0,
    });
    const RepoPicker = await importRepoPicker();
    render(<RepoPicker value="" onChange={vi.fn()} />);

    await waitFor(() => expect(screen.getByText('api error')).toBeInTheDocument());
  });

  it('calls onChange when a repo is selected', async () => {
    hoisted.listConnections.mockResolvedValue(ghConnection);
    hoisted.execute.mockResolvedValue(
      repoRes([
        { full_name: 'user/repo1', name: 'repo1', owner: { login: 'user' } },
        { full_name: 'user/repo2', name: 'repo2', owner: { login: 'user' } },
      ])
    );
    const onChange = vi.fn();
    const RepoPicker = await importRepoPicker();
    render(<RepoPicker value="" onChange={onChange} />);

    await waitFor(() =>
      expect(screen.getByRole('option', { name: 'user/repo2' })).toBeInTheDocument()
    );

    const select = screen.getByRole('combobox');
    await userEvent.selectOptions(select, 'user/repo2');
    expect(onChange).toHaveBeenCalledWith('user/repo2');
  });

  it('is disabled when disabled prop is true', async () => {
    hoisted.listConnections.mockResolvedValue(ghConnection);
    hoisted.execute.mockResolvedValue(
      repoRes([{ full_name: 'user/repo1', name: 'repo1', owner: { login: 'user' } }])
    );
    const RepoPicker = await importRepoPicker();
    render(<RepoPicker value="" onChange={vi.fn()} disabled />);

    // The disabled prop keeps it disabled even after loading
    const select = screen.getByRole('combobox') as HTMLSelectElement;
    // During loading, also disabled — just verify it's disabled at some point
    // wait for load to complete
    await waitFor(() => expect(hoisted.execute).toHaveBeenCalled());
    // Even after loaded, disabled=true keeps it disabled
    await waitFor(() =>
      expect(screen.getByRole('combobox') as HTMLSelectElement).toBeDisabled()
    );
  });

  it('parses repos wrapped under .repositories key', async () => {
    hoisted.listConnections.mockResolvedValue(ghConnection);
    hoisted.execute.mockResolvedValue({
      successful: true,
      data: {
        repositories: [
          { full_name: 'org/wrapped-repo', name: 'wrapped-repo', owner: { login: 'org' } },
        ],
      },
      error: null,
      costUsd: 0,
    });
    const RepoPicker = await importRepoPicker();
    render(<RepoPicker value="" onChange={vi.fn()} />);

    await waitFor(() =>
      expect(screen.getByRole('option', { name: 'org/wrapped-repo' })).toBeInTheDocument()
    );
  });
});
