/**
 * SkillsRunnerBody — vitest coverage for the saved-schedules block.
 *
 * Phase 2 of the SkillsRunnerBody / DevWorkflowPanel unification (see
 * docs/skills-runner-unification.md): this file is seeded with the
 * smoke-test for the enable/disable toggle so future Phase 3 chunks
 * (run-history, active-config card, smart-issue picker gating) drop
 * additional cases alongside.
 *
 * Covered here:
 *  - Mount with one saved schedule for the picked skill (mocking
 *    skills_list, skills_describe, cron_list, recent_runs).
 *  - Toggle flips enabled → false via openhumanCronUpdate(id, { enabled }).
 *  - The list re-loads after toggle (openhumanCronList called again).
 *  - aria-checked reflects the new state once the list refreshes.
 */
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// Mock the i18n hook with a stable identity-returning t() so our
// assertions can query by key (matches existing patterns in the repo,
// e.g. DevWorkflowPanel.test.tsx).
const stableT = (key: string) => key;
vi.mock('../../../lib/i18n/I18nContext', () => ({ useT: () => ({ t: stableT }) }));

// Hoisted mocks so vi.mock factories can reach them.
const hoisted = vi.hoisted(() => ({
  cronList: vi.fn(),
  cronAdd: vi.fn(),
  cronRemove: vi.fn(),
  cronRun: vi.fn(),
  cronUpdate: vi.fn(),
  listSkills: vi.fn(),
  describeSkill: vi.fn(),
  runSkill: vi.fn(),
  recentRuns: vi.fn(),
  readRunLog: vi.fn(),
}));

vi.mock('../../../utils/tauriCommands/cron', () => ({
  openhumanCronAdd: hoisted.cronAdd,
  openhumanCronList: hoisted.cronList,
  openhumanCronRemove: hoisted.cronRemove,
  openhumanCronRun: hoisted.cronRun,
  openhumanCronUpdate: hoisted.cronUpdate,
}));

vi.mock('../../../services/api/skillsApi', () => ({
  skillsApi: {
    listSkills: hoisted.listSkills,
    describeSkill: hoisted.describeSkill,
    runSkill: hoisted.runSkill,
    recentRuns: hoisted.recentRuns,
    readRunLog: hoisted.readRunLog,
  },
}));

// Composio-backed pickers fetch on mount — stub them so they don't
// throw on the test environment.
vi.mock('../inputs/RepoPicker', () => ({
  default: (props: { id: string; value: string; onChange: (s: string) => void }) => (
    <input
      data-testid="repo-picker-stub"
      id={props.id}
      value={props.value}
      onChange={(e) => props.onChange(e.target.value)}
    />
  ),
}));
vi.mock('../inputs/BranchPicker', () => ({
  default: (props: { id: string; value: string; onChange: (s: string) => void }) => (
    <input
      data-testid="branch-picker-stub"
      id={props.id}
      value={props.value}
      onChange={(e) => props.onChange(e.target.value)}
    />
  ),
}));

// Mock data ──────────────────────────────────────────────────────────

const SKILL_ID = 'github-issue-crusher';

const skillsList = [{ id: SKILL_ID, name: 'GitHub Issue Crusher' }];

const skillDescription = {
  id: SKILL_ID,
  name: 'GitHub Issue Crusher',
  when_to_use: 'Pick + fix an issue.',
  inputs: [],
};

function makeJob(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    id: 'job-1',
    expression: '*/30 * * * *',
    schedule: { kind: 'cron', expr: '*/30 * * * *' },
    command: '',
    prompt: '',
    name: `skill-run-${SKILL_ID}`,
    job_type: 'agent',
    session_target: 'isolated',
    enabled: true,
    delivery: { mode: 'proactive', best_effort: true },
    delete_after_run: false,
    created_at: '2026-05-29T10:00:00Z',
    next_run: '2026-05-29T11:00:00Z',
    ...overrides,
  };
}

async function importBody() {
  const mod = await import('../SkillsRunnerBody');
  return mod.SkillsRunnerBody;
}

// Tests ──────────────────────────────────────────────────────────────

describe('SkillsRunnerBody — saved-schedule toggle', () => {
  beforeEach(() => {
    Object.values(hoisted).forEach((fn) => fn.mockReset());

    hoisted.listSkills.mockResolvedValue(skillsList);
    hoisted.describeSkill.mockResolvedValue(skillDescription);
    hoisted.recentRuns.mockResolvedValue([]);
    hoisted.cronList.mockResolvedValue({ result: [makeJob({ enabled: true })] });
    hoisted.cronUpdate.mockResolvedValue({ result: makeJob({ enabled: false }) });
  });

  it('renders the toggle in the enabled state for an enabled job', async () => {
    const Body = await importBody();
    render(<Body />);

    // Wait for skills_list to resolve and populate the dropdown.
    await waitFor(() => expect(hoisted.listSkills).toHaveBeenCalled());

    // Pick the skill so the schedule list mounts.
    const select = screen.getByLabelText('settings.skillsRunner.skill') as HTMLSelectElement;
    fireEvent.change(select, { target: { value: SKILL_ID } });

    await waitFor(() => expect(hoisted.cronList).toHaveBeenCalled());

    const toggle = await screen.findByRole('switch', {
      name: 'settings.skillsRunner.scheduleToggleAria',
    });
    expect(toggle).toHaveAttribute('aria-checked', 'true');
    expect(screen.getByText('settings.skillsRunner.scheduleEnabled')).toBeInTheDocument();
  });

  it('calls openhumanCronUpdate with { enabled: false } when toggled on→off', async () => {
    const Body = await importBody();
    render(<Body />);

    await waitFor(() => expect(hoisted.listSkills).toHaveBeenCalled());
    const select = screen.getByLabelText('settings.skillsRunner.skill') as HTMLSelectElement;
    fireEvent.change(select, { target: { value: SKILL_ID } });
    await waitFor(() => expect(hoisted.cronList).toHaveBeenCalled());

    // After the first list, the next call (post-toggle) should return
    // the disabled job so the UI refresh reflects the new state.
    hoisted.cronList.mockResolvedValueOnce({ result: [makeJob({ enabled: false })] });

    const toggle = await screen.findByRole('switch', {
      name: 'settings.skillsRunner.scheduleToggleAria',
    });
    fireEvent.click(toggle);

    await waitFor(() =>
      expect(hoisted.cronUpdate).toHaveBeenCalledWith('job-1', { enabled: false })
    );

    // Refresh-list invoked after toggle (so the label updates).
    await waitFor(() => expect(hoisted.cronList).toHaveBeenCalledTimes(2));

    await waitFor(() =>
      expect(
        screen.getByRole('switch', { name: 'settings.skillsRunner.scheduleToggleAria' })
      ).toHaveAttribute('aria-checked', 'false')
    );
    expect(screen.getByText('settings.skillsRunner.scheduleDisabled')).toBeInTheDocument();
  });

  it('round-trips off→on as well', async () => {
    hoisted.cronList.mockResolvedValueOnce({ result: [makeJob({ enabled: false })] });

    const Body = await importBody();
    render(<Body />);

    await waitFor(() => expect(hoisted.listSkills).toHaveBeenCalled());
    const select = screen.getByLabelText('settings.skillsRunner.skill') as HTMLSelectElement;
    fireEvent.change(select, { target: { value: SKILL_ID } });
    await waitFor(() => expect(hoisted.cronList).toHaveBeenCalled());

    const toggle = await screen.findByRole('switch', {
      name: 'settings.skillsRunner.scheduleToggleAria',
    });
    expect(toggle).toHaveAttribute('aria-checked', 'false');

    fireEvent.click(toggle);
    await waitFor(() =>
      expect(hoisted.cronUpdate).toHaveBeenCalledWith('job-1', { enabled: true })
    );
  });
});
