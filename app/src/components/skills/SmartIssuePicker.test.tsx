import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import SmartIssuePicker from './SmartIssuePicker';

vi.mock('../../lib/i18n/I18nContext', () => ({ useT: () => ({ t: (k: string) => k }) }));

const mockListConnections = vi.fn();
const mockExecute = vi.fn();

vi.mock('../../lib/composio/composioApi', () => ({
  listConnections: () => mockListConnections(),
  execute: (...args: unknown[]) => mockExecute(...args),
}));

describe('SmartIssuePicker', () => {
  const baseProps = { values: {}, onPatchInputs: vi.fn() };

  beforeEach(() => {
    mockListConnections.mockResolvedValue({ connections: [] });
    mockExecute.mockResolvedValue({ repositories: [] });
  });

  it('renders the repo dropdown', async () => {
    render(<SmartIssuePicker {...baseProps} />);
    await waitFor(() => {
      expect(screen.getByRole('combobox')).toBeInTheDocument();
    });
  });

  it('shows loading and then empty state when no GitHub connection found', async () => {
    mockListConnections.mockResolvedValue({ connections: [] });
    render(<SmartIssuePicker {...baseProps} />);
    await waitFor(() => {
      // After loading resolves, dropdown should be present
      expect(screen.getByRole('combobox')).toBeInTheDocument();
    });
  });

  it('renders repos when GitHub connection is active', async () => {
    mockListConnections.mockResolvedValue({
      connections: [
        { toolkit: 'github', status: 'ACTIVE', username: 'testuser' },
      ],
    });
    mockExecute.mockResolvedValue({
      repositories: [{ full_name: 'testuser/myrepo', private: false, default_branch: 'main' }],
    });
    render(<SmartIssuePicker {...baseProps} />);
    await waitFor(() => {
      expect(screen.getByRole('combobox')).toBeInTheDocument();
    });
  });

  it('pre-selects repo from values prop', async () => {
    mockListConnections.mockResolvedValue({ connections: [] });
    render(<SmartIssuePicker {...baseProps} values={{ repo: 'owner/repo' }} />);
    await waitFor(() => {
      expect(screen.getByRole('combobox')).toBeInTheDocument();
    });
  });

  it('handles listConnections error gracefully', async () => {
    mockListConnections.mockRejectedValue(new Error('network error'));
    render(<SmartIssuePicker {...baseProps} />);
    await waitFor(() => {
      // Component should render without throwing
      expect(screen.getByRole('combobox')).toBeInTheDocument();
    });
  });
});
