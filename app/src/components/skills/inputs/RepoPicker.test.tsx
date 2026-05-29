import { render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import RepoPicker from './RepoPicker';

vi.mock('../../../lib/i18n/I18nContext', () => ({ useT: () => ({ t: (k: string) => k }) }));
vi.mock('../../../lib/composio/composioApi', () => ({
  execute: vi.fn().mockResolvedValue({ repositories: [] }),
  listConnections: vi.fn().mockResolvedValue([]),
}));

describe('RepoPicker', () => {
  const baseProps = { value: '', onChange: vi.fn() };

  it('renders a combobox', async () => {
    render(<RepoPicker {...baseProps} />);
    await waitFor(() => {
      expect(screen.getByRole('combobox')).toBeInTheDocument();
    });
  });

  it('respects disabled prop', async () => {
    render(<RepoPicker {...baseProps} disabled />);
    const select = await screen.findByRole('combobox');
    expect(select).toBeDisabled();
  });

  it('forwards id to the select element', async () => {
    render(<RepoPicker {...baseProps} id="repo-input" />);
    const select = await screen.findByRole('combobox');
    expect(select).toHaveAttribute('id', 'repo-input');
  });
});
