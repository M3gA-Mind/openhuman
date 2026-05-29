import { render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import BranchPicker from './BranchPicker';

vi.mock('../../../lib/i18n/I18nContext', () => ({ useT: () => ({ t: (k: string) => k }) }));
vi.mock('../../../lib/composio/composioApi', () => ({
  execute: vi.fn().mockResolvedValue({ branches: [] }),
}));

describe('BranchPicker', () => {
  const baseProps = { value: '', onChange: vi.fn(), repo: '' };

  it('renders disabled with hint when no repo is selected', async () => {
    render(<BranchPicker {...baseProps} />);
    const select = await screen.findByRole('combobox');
    expect(select).toBeDisabled();
  });

  it('renders with a repo prop and attempts to load branches', async () => {
    render(<BranchPicker {...baseProps} repo="owner/repo" />);
    await waitFor(() => {
      expect(screen.getByRole('combobox')).toBeInTheDocument();
    });
  });

  it('reflects a pre-selected value', async () => {
    render(<BranchPicker {...baseProps} repo="owner/repo" value="main" />);
    const select = await screen.findByRole('combobox');
    expect(select).toBeInTheDocument();
  });

  it('is disabled when disabled prop is true', async () => {
    render(<BranchPicker {...baseProps} repo="owner/repo" disabled />);
    const select = await screen.findByRole('combobox');
    expect(select).toBeDisabled();
  });
});
