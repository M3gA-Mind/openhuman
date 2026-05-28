/**
 * CreateSkillForm — standalone form coverage.
 *
 * Phase 5 of the /skills IA restructure: validates the form behaves
 * the same way as it did inside CreateSkillModal, so both the modal
 * and the /skills/new page can rely on it.
 *
 * Covers:
 *  - submit calls skillsApi.createSkill with the trimmed/normalised
 *    payload (CSVs split, optional fields omitted when empty).
 *  - onStateChange is called with validity + submitting flags so
 *    wrappers can sync their submit button's disabled state.
 *  - error path surfaces the Rust message in role="alert".
 *  - slug preview reflects the typed name.
 */
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const stableT = (key: string) => key;
vi.mock('../../../lib/i18n/I18nContext', () => ({ useT: () => ({ t: stableT }) }));

const hoisted = vi.hoisted(() => ({
  createSkill: vi.fn(),
}));

vi.mock('../../../services/api/skillsApi', () => ({
  skillsApi: { createSkill: hoisted.createSkill },
}));

import CreateSkillForm, { previewSlug, splitCsv } from '../CreateSkillForm';

const FORM_ID = 'create-skill-test-form';

describe('previewSlug', () => {
  it('lowercases ASCII alnum, collapses spaces/underscores to single hyphens', () => {
    expect(previewSlug('My New Skill')).toBe('my-new-skill');
    expect(previewSlug('foo___bar')).toBe('foo-bar');
    expect(previewSlug('Hello, World!')).toBe('hello-world');
  });

  it('trims leading/trailing hyphens', () => {
    expect(previewSlug('  - leading and trailing - ')).toBe('leading-and-trailing');
  });

  it('strips diacritics via NFKD and drops symbols', () => {
    // NFKD decomposes é → e + combining acute; the combining mark is
    // outside ASCII alnum so it's dropped, leaving `cafe-beans`.
    expect(previewSlug('café & beans')).toBe('cafe-beans');
  });
});

describe('splitCsv', () => {
  it('trims entries and drops empties', () => {
    expect(splitCsv('a , b,, c ,')).toEqual(['a', 'b', 'c']);
  });
});

describe('CreateSkillForm', () => {
  beforeEach(() => {
    hoisted.createSkill.mockReset();
  });

  it('renders required fields and the slug preview updates as the name changes', () => {
    render(
      <CreateSkillForm formId={FORM_ID} onCreated={vi.fn()} />
    );

    const nameInput = screen.getByLabelText(/skills.create.name/i) as HTMLInputElement;
    fireEvent.change(nameInput, { target: { value: 'My Cool Skill' } });
    expect(screen.getByText('my-cool-skill')).toBeInTheDocument();
  });

  it('reports validity to the wrapper via onStateChange when name and description are filled', () => {
    const onStateChange = vi.fn();
    render(
      <CreateSkillForm formId={FORM_ID} onCreated={vi.fn()} onStateChange={onStateChange} />
    );

    // Initially invalid (empty form).
    expect(onStateChange).toHaveBeenLastCalledWith({ valid: false, submitting: false });

    fireEvent.change(screen.getByLabelText(/skills.create.name/i), {
      target: { value: 'My Skill' },
    });
    // Name alone is not enough.
    expect(onStateChange).toHaveBeenLastCalledWith({ valid: false, submitting: false });

    fireEvent.change(screen.getByLabelText(/skills.create.description/i), {
      target: { value: 'Does the thing.' },
    });
    expect(onStateChange).toHaveBeenLastCalledWith({ valid: true, submitting: false });
  });

  it('submits with the trimmed payload, dropping empty optional fields', async () => {
    const created = { id: 'my-skill', name: 'My Skill', scope: 'user', legacy: false };
    hoisted.createSkill.mockResolvedValue(created);
    const onCreated = vi.fn();

    render(<CreateSkillForm formId={FORM_ID} onCreated={onCreated} />);

    fireEvent.change(screen.getByLabelText(/skills.create.name/i), {
      target: { value: '  My Skill  ' },
    });
    fireEvent.change(screen.getByLabelText(/skills.create.description/i), {
      target: { value: '  Does the thing.  ' },
    });
    fireEvent.change(screen.getByLabelText(/skills.create.tags/i), {
      target: { value: 'tag-a, tag-b ,, ' },
    });

    // The form has no internal submit button — fire a submit event on
    // the <form id> directly (this is what `<button form=...>` does
    // from a wrapper).
    fireEvent.submit(document.getElementById(FORM_ID)!);

    await waitFor(() => {
      expect(hoisted.createSkill).toHaveBeenCalledWith({
        name: 'My Skill',
        description: 'Does the thing.',
        scope: 'user',
        tags: ['tag-a', 'tag-b'],
      });
    });
    expect(onCreated).toHaveBeenCalledWith(created);
  });

  it('surfaces the Rust error message in an alert when createSkill rejects', async () => {
    hoisted.createSkill.mockRejectedValue(new Error('slug already exists'));
    render(<CreateSkillForm formId={FORM_ID} onCreated={vi.fn()} />);

    fireEvent.change(screen.getByLabelText(/skills.create.name/i), {
      target: { value: 'Dupe' },
    });
    fireEvent.change(screen.getByLabelText(/skills.create.description/i), {
      target: { value: 'whatever' },
    });
    fireEvent.submit(document.getElementById(FORM_ID)!);

    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('slug already exists');
  });

  it('does not call createSkill if the form is invalid (no name)', async () => {
    render(<CreateSkillForm formId={FORM_ID} onCreated={vi.fn()} />);
    fireEvent.submit(document.getElementById(FORM_ID)!);
    // Give the microtask queue a tick — should still be 0.
    await Promise.resolve();
    expect(hoisted.createSkill).not.toHaveBeenCalled();
  });
});
