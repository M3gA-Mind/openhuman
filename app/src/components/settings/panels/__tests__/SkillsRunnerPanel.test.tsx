// Tests for Settings → Developer Options → Skills Runner panel.
// Verifies that the thin wrapper panel renders the header/back button and
// delegates the actual runner UX to SkillsRunnerBody.
import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

// Mock the navigation hook — panel only needs navigateBack + breadcrumbs.
vi.mock('../../hooks/useSettingsNavigation', () => ({
  useSettingsNavigation: () => ({
    navigateBack: vi.fn(),
    breadcrumbs: [],
  }),
}));

// Stub SettingsHeader so we can assert on its props without pulling in its deps.
vi.mock('../../components/SettingsHeader', () => ({
  default: ({
    title,
    showBackButton,
  }: {
    title: string;
    showBackButton: boolean;
    onBack: () => void;
    breadcrumbs: unknown[];
  }) => (
    <div data-testid="settings-header">
      <span data-testid="header-title">{title}</span>
      {showBackButton && <button type="button">back</button>}
    </div>
  ),
}));

// Stub SkillsRunnerBody — it's complex and tested separately.
// Path is relative to the test file in panels/__tests__/.
vi.mock('../../../skills/SkillsRunnerBody', () => ({
  default: () => <div data-testid="skills-runner-body" />,
}));

describe('SkillsRunnerPanel', () => {
  it('renders the settings header with the Skills Runner title', async () => {
    const { default: SkillsRunnerPanel } = await import('../SkillsRunnerPanel');
    render(<SkillsRunnerPanel />);

    expect(screen.getByTestId('settings-header')).toBeInTheDocument();
    // The i18n system resolves settings.developerMenu.skillsRunner.title to "Skills Runner"
    expect(screen.getByTestId('header-title')).toHaveTextContent('Skills Runner');
  });

  it('renders with a back button', async () => {
    const { default: SkillsRunnerPanel } = await import('../SkillsRunnerPanel');
    render(<SkillsRunnerPanel />);

    expect(screen.getByRole('button', { name: 'back' })).toBeInTheDocument();
  });

  it('renders the SkillsRunnerBody', async () => {
    const { default: SkillsRunnerPanel } = await import('../SkillsRunnerPanel');
    render(<SkillsRunnerPanel />);

    expect(screen.getByTestId('skills-runner-body')).toBeInTheDocument();
  });
});
