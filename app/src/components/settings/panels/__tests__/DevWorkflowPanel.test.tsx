/**
 * DevWorkflowPanel (deprecated stub) — vitest coverage.
 *
 * After the Skills Runner unification (Phase 3 chunk 4, see
 * docs/skills-runner-unification.md) this panel is a tiny "moved to
 * /skills" notice. The old behaviour (Composio repo loading, fork
 * detection, branch dropdown, cron CRUD, run history) lives in
 * SkillsRunnerBody + SmartIssuePicker, and is covered by that
 * component's own tests.
 *
 * Covered here:
 *  - The "moved" notice renders.
 *  - Clicking "Open Skills" navigates to /skills.
 */
import { fireEvent, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import { renderWithProviders } from '../../../../test/test-utils';

const navigate = vi.fn();
vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useNavigate: () => navigate };
});

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

describe('DevWorkflowPanel (deprecated stub)', () => {
  it('renders the moved-to-skills notice', async () => {
    const { default: DevWorkflowPanel } = await import('../DevWorkflowPanel');
    renderWithProviders(<DevWorkflowPanel />);
    expect(screen.getByTestId('dev-workflow-moved-notice')).toBeInTheDocument();
    expect(screen.getByText('settings.devWorkflow.movedHeading')).toBeInTheDocument();
    expect(screen.getByText('settings.devWorkflow.movedBody')).toBeInTheDocument();
  });

  it('navigates to /skills on click', async () => {
    navigate.mockReset();
    const { default: DevWorkflowPanel } = await import('../DevWorkflowPanel');
    renderWithProviders(<DevWorkflowPanel />);
    fireEvent.click(screen.getByRole('button', { name: 'settings.devWorkflow.movedOpenSkills' }));
    expect(navigate).toHaveBeenCalledWith('/skills');
  });
});
