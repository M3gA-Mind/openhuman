/**
 * CreateSkillForm
 * ----------------
 *
 * Body of the "create a new SKILL.md" flow, shared between
 * `CreateSkillModal` (modal chrome) and the `/skills/new` page wrapper.
 *
 * Owns:
 *   - All form fields (name, description, scope, license, author,
 *     tags, allowed-tools).
 *   - Slug preview + validation (name and description required).
 *   - Submit handler that calls `skillsApi.createSkill` and surfaces
 *     the result via `onCreated(skill)` / error string via inline
 *     `<div role="alert">`.
 *
 * Does NOT own:
 *   - The submit/cancel buttons (the wrapper provides them so the
 *     modal can use a footer bar and the page can render a top-right
 *     primary action).
 *   - Modal-specific concerns (focus capture, Escape-to-close,
 *     backdrop click). Those stay in `CreateSkillModal`.
 *
 * The wrapper drives submission by either calling the imperative
 * handle exposed via a ref (`<CreateSkillForm ref={ref} ... />` →
 * `ref.current.submit()`) OR by reading `formValid` + `submitting`
 * from the props the form raises and wiring its own submit button to
 * the underlying `<form>` via the standard `form="..."` attribute.
 * Both modal and page use the latter, so the form mounts a real
 * `<form id={formId}>` and they bind `<button form={formId}>`.
 */
import debug from 'debug';
import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
} from 'react';

import { useT } from '../../lib/i18n/I18nContext';
import {
  type CreateSkillInput,
  type SkillScope,
  type SkillSummary,
  skillsApi,
} from '../../services/api/skillsApi';

const log = debug('skills:create-form');

export interface CreateSkillFormHandle {
  /** True iff name+description are present and no submit is in flight. */
  isValid: () => boolean;
  /** True while skillsApi.createSkill is in flight. */
  isSubmitting: () => boolean;
  /** Imperatively trigger submit. Resolves once the round-trip finishes. */
  submit: () => Promise<void>;
}

export interface CreateSkillFormProps {
  /**
   * The id assigned to the underlying `<form>` element. Wrappers that
   * render their submit button outside the form (modal footer / page
   * header) set `<button form={formId}>` to fire submit via this id.
   */
  formId: string;
  /** Called with the freshly-created skill on success. */
  onCreated: (skill: SkillSummary) => void;
  /**
   * Called whenever validity / submission state changes so the
   * wrapper can sync its submit button's disabled state without
   * needing to introspect via a ref every render.
   */
  onStateChange?: (state: { valid: boolean; submitting: boolean }) => void;
  /** If true, autofocus the first field on mount (modal default). */
  autoFocus?: boolean;
}

/**
 * Client-side slug preview — mirrors the Rust `slugify_skill_name`
 * heuristic (lowercase, ASCII alphanumerics + `-`, collapse repeats,
 * trim hyphens at the edges). The preview is advisory only; the Rust
 * side is authoritative when the skill is persisted.
 */
export function previewSlug(name: string): string {
  const lower = name.normalize('NFKD').toLowerCase();
  let out = '';
  let prevHyphen = false;
  for (const ch of lower) {
    if ((ch >= 'a' && ch <= 'z') || (ch >= '0' && ch <= '9')) {
      out += ch;
      prevHyphen = false;
      continue;
    }
    if ((ch === '-' || ch === '_' || /\s/.test(ch)) && !prevHyphen) {
      out += '-';
      prevHyphen = true;
    }
  }
  return out.replace(/^-+|-+$/g, '');
}

const CreateSkillForm = forwardRef<CreateSkillFormHandle, CreateSkillFormProps>(
  function CreateSkillForm({ formId, onCreated, onStateChange, autoFocus = false }, ref) {
    const { t } = useT();
    const [name, setName] = useState('');
    const [description, setDescription] = useState('');
    // Scope is fixed to 'user' — the form previously exposed a radio
    // toggle for user/project plus license/author/tags/allowed-tools
    // fields. None of those were useful in practice and they cluttered
    // the create flow; user-scoped is the only sensible default for
    // dashboard-created skills. Project-scoped skills are still
    // creatable by editing the workspace skill files directly. The
    // backend payload still requires `scope` so we hold it as a const.
    const scope: SkillScope = 'user';
    const [submitting, setSubmitting] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const firstFieldRef = useRef<HTMLInputElement | null>(null);

    const slug = useMemo(() => previewSlug(name), [name]);

    const nameValid = slug.length > 0;
    const descriptionValid = description.trim().length > 0;
    const formValid = nameValid && descriptionValid && !submitting;

    // Surface state to the wrapper for its submit button's disabled prop.
    useEffect(() => {
      onStateChange?.({ valid: formValid, submitting });
    }, [formValid, submitting, onStateChange]);

    useEffect(() => {
      if (!autoFocus) return;
      const raf = window.requestAnimationFrame(() => {
        firstFieldRef.current?.focus();
      });
      return () => {
        window.cancelAnimationFrame(raf);
      };
    }, [autoFocus]);

    const submit = useCallback(async () => {
      if (!formValid) return;
      const payload: CreateSkillInput = {
        name: name.trim(),
        description: description.trim(),
        scope,
      };

      log('submit name=%s scope=%s', payload.name, payload.scope);
      setSubmitting(true);
      setError(null);
      try {
        const created = await skillsApi.createSkill(payload);
        log('submit-ok id=%s', created.id);
        onCreated(created);
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        log('submit-err %s', message);
        setError(message);
        setSubmitting(false);
      }
    }, [description, formValid, name, onCreated]);

    useImperativeHandle(
      ref,
      () => ({
        isValid: () => formValid,
        isSubmitting: () => submitting,
        submit,
      }),
      [formValid, submitting, submit]
    );

    const handleFormSubmit = (e: React.FormEvent) => {
      e.preventDefault();
      void submit();
    };

    return (
      <form id={formId} onSubmit={handleFormSubmit} className="space-y-4">
        {/* Name */}
        <div>
          <label
            htmlFor="create-skill-name"
            className="block text-xs font-medium text-stone-600 dark:text-neutral-300"
          >
            {t('skills.create.name')}
            <span className="text-coral-500"> *</span>
          </label>
          <input
            id="create-skill-name"
            ref={firstFieldRef}
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            required
            maxLength={128}
            className="mt-1 w-full rounded-lg border border-stone-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 px-3 py-2 text-sm text-stone-900 dark:text-neutral-100 shadow-sm transition-colors focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/30"
            placeholder={t('skills.create.namePlaceholder')}
          />
          <p className="mt-1 text-[11px] text-stone-500 dark:text-neutral-400">
            {t('skills.create.slugLabel')}{' '}
            <code className="rounded bg-stone-100 dark:bg-neutral-800 px-1 py-[1px] font-mono text-stone-700 dark:text-neutral-200">
              {slug || '—'}
            </code>
          </p>
        </div>

        {/* Description */}
        <div>
          <label
            htmlFor="create-skill-description"
            className="block text-xs font-medium text-stone-600 dark:text-neutral-300"
          >
            {t('skills.create.description')}
            <span className="text-coral-500"> *</span>
          </label>
          <textarea
            id="create-skill-description"
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            required
            rows={3}
            maxLength={500}
            className="mt-1 w-full rounded-lg border border-stone-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 px-3 py-2 text-sm text-stone-900 dark:text-neutral-100 shadow-sm transition-colors focus:border-primary-500 focus:outline-none focus:ring-2 focus:ring-primary-500/30"
            placeholder={t('skills.create.descriptionPlaceholder')}
          />
        </div>

        {/* Error */}
        {error ? (
          <div
            role="alert"
            className="rounded-xl border border-coral-200 bg-coral-50 p-3 text-xs text-coral-900"
          >
            <p className="font-semibold">{t('skills.create.createError')}</p>
            <p className="mt-1 whitespace-pre-wrap font-mono">{error}</p>
          </div>
        ) : null}
      </form>
    );
  }
);

export default CreateSkillForm;
