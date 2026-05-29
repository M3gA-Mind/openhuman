// Tests for the /skills/run page.
// The page is a thin wrapper around SkillsRunnerBody with a page-level
// header (back button + title) that navigates to /skills?tab=runners.
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

// Mock i18n — return the key so assertions are stable.
const stableT = (key: string) => key;
vi.mock('../../lib/i18n/I18nContext', () => ({ useT: () => ({ t: stableT }) }));

// Capture the navigate mock so we can assert it was called.
const mockNavigate = vi.fn();
vi.mock('react-router-dom', async importOriginal => {
  const actual = await importOriginal<typeof import('react-router-dom')>();
  return {
    ...actual,
    useNavigate: () => mockNavigate,
  };
});

// Stub SkillsRunnerBody — it's complex and tested separately.
// Path is relative to the test file in pages/__tests__/.
vi.mock('../../components/skills/SkillsRunnerBody', () => ({
  default: () => <div data-testid="skills-runner-body" />,
}));

describe('SkillsRun page', () => {
  it('renders the back button and title', async () => {
    const { default: SkillsRun } = await import('../SkillsRun');
    render(<SkillsRun />);

    // Back button uses the aria-label from t('common.back')
    expect(screen.getByRole('button', { name: 'common.back' })).toBeInTheDocument();
    // Page title is skills.tabs.runners key
    expect(screen.getByText('skills.tabs.runners')).toBeInTheDocument();
  });

  it('clicking the back button navigates to /skills?tab=runners', async () => {
    const { default: SkillsRun } = await import('../SkillsRun');
    render(<SkillsRun />);

    const backBtn = screen.getByRole('button', { name: 'common.back' });
    fireEvent.click(backBtn);

    expect(mockNavigate).toHaveBeenCalledWith('/skills?tab=runners');
  });

  it('renders the SkillsRunnerBody', async () => {
    const { default: SkillsRun } = await import('../SkillsRun');
    render(<SkillsRun />);

    expect(screen.getByTestId('skills-runner-body')).toBeInTheDocument();
  });

  it('renders the back arrow symbol', async () => {
    const { default: SkillsRun } = await import('../SkillsRun');
    render(<SkillsRun />);

    // The back button contains "← common.back" text
    const backBtn = screen.getByRole('button', { name: 'common.back' });
    expect(backBtn).toHaveTextContent('←');
  });
});
