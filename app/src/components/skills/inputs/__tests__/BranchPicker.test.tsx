/**
 * BranchPicker — unit tests
 *
 * Covers: disabled-when-no-repo, loading state, branch options rendered,
 * error fallback (still shows default branches), onChange callback,
 * disabled prop passthrough.
 */
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const stableT = (key: string) => key;
vi.mock('../../../../lib/i18n/I18nContext', () => ({ useT: () => ({ t: stableT }) }));

const hoisted = vi.hoisted(() => ({
  execute: vi.fn(),
}));

vi.mock('../../../../lib/composio/composioApi', () => ({
  execute: hoisted.execute,
}));

async function importBranchPicker() {
  const mod = await import('../BranchPicker');
  return mod.default;
}

// Shared response factories
function branchRes(names: string[]) {
  return {
    successful: true,
    data: names.map((name) => ({ name })),
    error: null,
    costUsd: 0,
  };
}

describe('BranchPicker', () => {
  beforeEach(() => {
    hoisted.execute.mockReset();
  });

  it('renders disabled with needRepo hint when no repo provided', async () => {
    const BranchPicker = await importBranchPicker();
    render(<BranchPicker value="" onChange={vi.fn()} repo="" />);

    const select = screen.getByRole('combobox') as HTMLSelectElement;
    expect(select).toBeDisabled();
    // The placeholder option shows the needRepo key
    expect(select).toHaveTextContent('settings.skillsRunner.branchPicker.needRepo');
    // No composio call should happen when repo is empty
    expect(hoisted.execute).not.toHaveBeenCalled();
  });

  it('does not fetch when repo has no slash (invalid format)', async () => {
    const BranchPicker = await importBranchPicker();
    render(<BranchPicker value="" onChange={vi.fn()} repo="justarepo" />);

    // Allow any async effects to settle
    await waitFor(() => expect(hoisted.execute).not.toHaveBeenCalled());
    // The select is enabled (repo truthy) but no branches are listed
    const options = screen.getAllByRole('option');
    // Only the placeholder option
    expect(options).toHaveLength(1);
  });

  it('renders branch options after successful fetch', async () => {
    hoisted.execute.mockResolvedValue(branchRes(['main', 'dev', 'feature-x']));
    const BranchPicker = await importBranchPicker();
    render(<BranchPicker value="" onChange={vi.fn()} repo="owner/myrepo" />);

    await waitFor(() =>
      expect(hoisted.execute).toHaveBeenCalledWith('GITHUB_LIST_BRANCHES', {
        owner: 'owner',
        repo: 'myrepo',
        per_page: 100,
      })
    );

    await waitFor(() => expect(screen.getByRole('option', { name: 'main' })).toBeInTheDocument());
    expect(screen.getByRole('option', { name: 'dev' })).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'feature-x' })).toBeInTheDocument();
  });

  it('falls back to main/master when fetch returns empty list', async () => {
    hoisted.execute.mockResolvedValue({ successful: true, data: [], error: null, costUsd: 0 });
    const BranchPicker = await importBranchPicker();
    render(<BranchPicker value="" onChange={vi.fn()} repo="owner/myrepo" />);

    await waitFor(() => expect(hoisted.execute).toHaveBeenCalled());
    await waitFor(() => expect(screen.getByRole('option', { name: 'main' })).toBeInTheDocument());
    expect(screen.getByRole('option', { name: 'master' })).toBeInTheDocument();
  });

  it('shows error text but still renders default branches on fetch failure', async () => {
    hoisted.execute.mockResolvedValue({
      successful: false,
      data: null,
      error: 'rate limited',
      costUsd: 0,
    });
    const BranchPicker = await importBranchPicker();
    render(<BranchPicker value="" onChange={vi.fn()} repo="owner/myrepo" />);

    await waitFor(() => expect(hoisted.execute).toHaveBeenCalled());

    // Error message paragraph
    await waitFor(() => expect(screen.getByText('rate limited')).toBeInTheDocument());
    // Fallback branches still rendered
    expect(screen.getByRole('option', { name: 'main' })).toBeInTheDocument();
    expect(screen.getByRole('option', { name: 'master' })).toBeInTheDocument();
  });

  it('shows error when execute throws', async () => {
    hoisted.execute.mockRejectedValue(new Error('network error'));
    const BranchPicker = await importBranchPicker();
    render(<BranchPicker value="" onChange={vi.fn()} repo="owner/myrepo" />);

    await waitFor(() => expect(hoisted.execute).toHaveBeenCalled());
    await waitFor(() => expect(screen.getByText('network error')).toBeInTheDocument());
    // Fallback branches rendered even on exception
    expect(screen.getByRole('option', { name: 'main' })).toBeInTheDocument();
  });

  it('calls onChange when branch is selected', async () => {
    hoisted.execute.mockResolvedValue(branchRes(['main', 'dev']));
    const onChange = vi.fn();
    const BranchPicker = await importBranchPicker();
    render(<BranchPicker value="" onChange={onChange} repo="owner/myrepo" />);

    await waitFor(() => expect(screen.getByRole('option', { name: 'dev' })).toBeInTheDocument());

    const select = screen.getByRole('combobox');
    await userEvent.selectOptions(select, 'dev');
    expect(onChange).toHaveBeenCalledWith('dev');
  });

  it('renders disabled when disabled prop is true even with a valid repo', async () => {
    hoisted.execute.mockResolvedValue(branchRes(['main']));
    const BranchPicker = await importBranchPicker();
    render(<BranchPicker value="" onChange={vi.fn()} repo="owner/myrepo" disabled />);

    // Wait for loading to finish
    await waitFor(() => expect(hoisted.execute).toHaveBeenCalled());
    await waitFor(() => expect(screen.getByRole('option', { name: 'main' })).toBeInTheDocument());

    const select = screen.getByRole('combobox') as HTMLSelectElement;
    expect(select).toBeDisabled();
  });

  it('refetches branches when repo prop changes', async () => {
    hoisted.execute.mockResolvedValue(branchRes(['main']));
    const BranchPicker = await importBranchPicker();
    const { rerender } = render(<BranchPicker value="" onChange={vi.fn()} repo="owner/repo1" />);

    await waitFor(() =>
      expect(hoisted.execute).toHaveBeenCalledWith('GITHUB_LIST_BRANCHES', {
        owner: 'owner',
        repo: 'repo1',
        per_page: 100,
      })
    );

    hoisted.execute.mockResolvedValue(branchRes(['release']));
    rerender(<BranchPicker value="" onChange={vi.fn()} repo="owner/repo2" />);

    await waitFor(() =>
      expect(hoisted.execute).toHaveBeenCalledWith('GITHUB_LIST_BRANCHES', {
        owner: 'owner',
        repo: 'repo2',
        per_page: 100,
      })
    );
    await waitFor(() => expect(screen.getByRole('option', { name: 'release' })).toBeInTheDocument());
  });

  it('renders placeholder prop in the default option when repo is set', async () => {
    hoisted.execute.mockResolvedValue(branchRes(['main']));
    const BranchPicker = await importBranchPicker();
    render(
      <BranchPicker value="" onChange={vi.fn()} repo="owner/myrepo" placeholder="Pick a branch" />
    );

    await waitFor(() => expect(hoisted.execute).toHaveBeenCalled());
    await waitFor(() => expect(screen.getByRole('option', { name: 'main' })).toBeInTheDocument());
    // Once loaded, the empty option shows the placeholder
    expect(screen.getByRole('option', { name: 'Pick a branch' })).toBeInTheDocument();
  });

  it('parses branches from .details key in the response object', async () => {
    // Covers the "else if (raw && typeof raw === 'object')" branch (lines 74-84)
    hoisted.execute.mockResolvedValue({
      successful: true,
      data: { details: [{ name: 'from-details' }, { name: 'other' }] },
      error: null,
      costUsd: 0,
    });
    const BranchPicker = await importBranchPicker();
    render(<BranchPicker value="" onChange={vi.fn()} repo="owner/myrepo" />);

    await waitFor(() => expect(hoisted.execute).toHaveBeenCalled());
    await waitFor(() =>
      expect(screen.getByRole('option', { name: 'from-details' })).toBeInTheDocument()
    );
    expect(screen.getByRole('option', { name: 'other' })).toBeInTheDocument();
  });

  it('parses branches from .data.details key (nested object wrapping)', async () => {
    hoisted.execute.mockResolvedValue({
      successful: true,
      data: { data: { details: [{ name: 'nested-detail' }] } },
      error: null,
      costUsd: 0,
    });
    const BranchPicker = await importBranchPicker();
    render(<BranchPicker value="" onChange={vi.fn()} repo="owner/myrepo" />);

    await waitFor(() => expect(hoisted.execute).toHaveBeenCalled());
    await waitFor(() =>
      expect(screen.getByRole('option', { name: 'nested-detail' })).toBeInTheDocument()
    );
  });
});
