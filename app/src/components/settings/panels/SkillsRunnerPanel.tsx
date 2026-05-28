// Settings panel: ad-hoc Skills Runner.
//
// Generalises across every bundled skill (`github-issue-crusher`,
// `pr-review-shepherd`, `dev-workflow`, plus anything the user installs
// later) — pick one from the dropdown, fill the dynamically-rendered
// inputs (loaded from `openhuman.skills_describe`), click Run Now to
// fire-and-forget a background autonomous run. The companion
// `DevWorkflowPanel` stays for cron-driven recurring runs against the
// dev-workflow skill specifically; this panel handles one-shot runs of
// any skill.

import createDebug from 'debug';
import { useCallback, useEffect, useMemo, useState } from 'react';

import { useT } from '../../../lib/i18n/I18nContext';
import {
  type SkillDescription,
  type SkillRunStarted,
  type SkillSummary,
  skillsApi,
} from '../../../services/api/skillsApi';
import SettingsHeader from '../components/SettingsHeader';
import { useSettingsNavigation } from '../hooks/useSettingsNavigation';

const log = createDebug('app:settings:SkillsRunnerPanel');

type InputValue = string | number | boolean;

interface RunState {
  status: 'idle' | 'submitting' | 'started' | 'error';
  message?: string;
  result?: SkillRunStarted;
}

// ── Helpers ────────────────────────────────────────────────────────────

/**
 * Default form value for an input based on its declared type. Strings/
 * integers default to empty (renders as placeholder); booleans to false.
 * `runSkill` later trims and drops empty optional fields before sending
 * them over the wire.
 */
function defaultForType(type: string): InputValue {
  if (type === 'boolean') return false;
  if (type === 'integer') return '';
  return '';
}

/**
 * Project the form-state map back into the JSON inputs shape `skills_run`
 * expects: trim strings, coerce integer-typed fields to numbers, drop
 * empty optional fields entirely (so the backend sees them as "not
 * provided" rather than `""`).
 */
function buildInputsPayload(
  description: SkillDescription,
  values: Record<string, InputValue>
): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const inp of description.inputs) {
    const raw = values[inp.name];
    if (raw === undefined || raw === null) {
      if (inp.required) {
        // Will fail validation in the submit handler before we even try to
        // send; included here so the project step is total.
        out[inp.name] = '';
      }
      continue;
    }
    if (inp.type === 'boolean') {
      out[inp.name] = Boolean(raw);
      continue;
    }
    if (typeof raw === 'string' && raw.trim() === '') {
      if (inp.required) out[inp.name] = '';
      continue;
    }
    if (inp.type === 'integer') {
      const n = typeof raw === 'number' ? raw : Number(String(raw).trim());
      if (Number.isFinite(n)) {
        out[inp.name] = n;
      } else if (inp.required) {
        out[inp.name] = raw; // let backend reject with a clear error
      }
      continue;
    }
    out[inp.name] = typeof raw === 'string' ? raw.trim() : raw;
  }
  return out;
}

// ── Component ──────────────────────────────────────────────────────────

const SkillsRunnerPanel = () => {
  const { t } = useT();
  const { navigateBack, breadcrumbs } = useSettingsNavigation();

  // Skill catalog (loaded once on mount)
  const [skills, setSkills] = useState<SkillSummary[]>([]);
  const [skillsLoading, setSkillsLoading] = useState(false);
  const [skillsError, setSkillsError] = useState<string | null>(null);

  // Active skill + its full description (inputs declared)
  const [selectedSkillId, setSelectedSkillId] = useState('');
  const [description, setDescription] = useState<SkillDescription | null>(null);
  const [descLoading, setDescLoading] = useState(false);
  const [descError, setDescError] = useState<string | null>(null);

  // Form state per input
  const [formValues, setFormValues] = useState<Record<string, InputValue>>({});

  // Run state
  const [run, setRun] = useState<RunState>({ status: 'idle' });

  // ── Initial load: skills_list ──────────────────────────────────────
  useEffect(() => {
    let cancelled = false;
    setSkillsLoading(true);
    setSkillsError(null);
    skillsApi
      .listSkills()
      .then((list) => {
        if (cancelled) return;
        // Hide the codegraph-smoke skill — internal smoke-test only.
        const filtered = list.filter((s) => s.id !== 'codegraph-smoke');
        setSkills(filtered);
        log('loaded %d skills', filtered.length);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        const msg = err instanceof Error ? err.message : String(err);
        log('listSkills error: %s', msg);
        setSkillsError(msg);
      })
      .finally(() => {
        if (!cancelled) setSkillsLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // ── On selection: skills_describe ──────────────────────────────────
  useEffect(() => {
    if (!selectedSkillId) {
      setDescription(null);
      setFormValues({});
      return;
    }
    let cancelled = false;
    setDescLoading(true);
    setDescError(null);
    setRun({ status: 'idle' });
    skillsApi
      .describeSkill(selectedSkillId)
      .then((desc) => {
        if (cancelled) return;
        setDescription(desc);
        // Seed form values from each input's default.
        const seed: Record<string, InputValue> = {};
        for (const i of desc.inputs) {
          seed[i.name] = defaultForType(i.type);
        }
        setFormValues(seed);
        log('described %s — %d inputs', selectedSkillId, desc.inputs.length);
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        const msg = err instanceof Error ? err.message : String(err);
        log('describeSkill error: %s', msg);
        setDescError(msg);
      })
      .finally(() => {
        if (!cancelled) setDescLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [selectedSkillId]);

  // ── Required-field validity ────────────────────────────────────────
  const missingRequired = useMemo(() => {
    if (!description) return [];
    const missing: string[] = [];
    for (const inp of description.inputs) {
      if (!inp.required) continue;
      const v = formValues[inp.name];
      if (v === undefined || v === null) {
        missing.push(inp.name);
        continue;
      }
      if (inp.type === 'boolean') continue; // false is a valid choice
      if (typeof v === 'string' && v.trim() === '') {
        missing.push(inp.name);
      }
    }
    return missing;
  }, [description, formValues]);

  // ── Run handler ────────────────────────────────────────────────────
  const handleRun = useCallback(async () => {
    if (!description) return;
    if (missingRequired.length > 0) {
      setRun({
        status: 'error',
        message: `${t('settings.skillsRunner.error.missingRequired')} ${missingRequired.join(', ')}`,
      });
      return;
    }
    setRun({ status: 'submitting' });
    try {
      const inputs = buildInputsPayload(description, formValues);
      log('runSkill %s inputs=%o', description.id, inputs);
      const result = await skillsApi.runSkill(description.id, inputs);
      setRun({ status: 'started', result });
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      log('runSkill error: %s', msg);
      setRun({ status: 'error', message: msg });
    }
  }, [description, formValues, missingRequired, t]);

  // ── Form-field renderer ────────────────────────────────────────────
  const renderField = (
    inp: SkillDescription['inputs'][number],
    value: InputValue,
    onChange: (next: InputValue) => void
  ) => {
    const id = `skills-runner-input-${inp.name}`;
    const requiredMark = inp.required ? <span className="text-red-500"> *</span> : null;
    const commonLabel = (
      <label
        htmlFor={id}
        className="block text-sm font-medium text-stone-700 dark:text-stone-300 mb-1"
      >
        {inp.name}
        {requiredMark}
      </label>
    );
    const desc = inp.description ? (
      <p className="text-xs text-stone-500 dark:text-stone-400 mt-1">{inp.description}</p>
    ) : null;

    if (inp.type === 'boolean') {
      return (
        <div key={inp.name}>
          <label
            htmlFor={id}
            className="flex items-center gap-2 text-sm font-medium text-stone-700 dark:text-stone-300"
          >
            <input
              id={id}
              type="checkbox"
              checked={Boolean(value)}
              onChange={(e) => onChange(e.target.checked)}
              className="rounded"
            />
            {inp.name}
            {requiredMark}
          </label>
          {desc}
        </div>
      );
    }

    if (inp.type === 'integer') {
      return (
        <div key={inp.name}>
          {commonLabel}
          <input
            id={id}
            type="number"
            inputMode="numeric"
            value={typeof value === 'number' ? value : (value as string)}
            onChange={(e) => onChange(e.target.value)}
            placeholder={inp.required ? t('settings.skillsRunner.placeholder.required') : ''}
            className="w-full rounded border border-stone-300 dark:border-stone-600 bg-white dark:bg-stone-800 px-3 py-2 text-sm text-stone-900 dark:text-stone-100"
          />
          {desc}
        </div>
      );
    }

    // string (default)
    return (
      <div key={inp.name}>
        {commonLabel}
        <input
          id={id}
          type="text"
          value={value as string}
          onChange={(e) => onChange(e.target.value)}
          placeholder={inp.required ? t('settings.skillsRunner.placeholder.required') : ''}
          className="w-full rounded border border-stone-300 dark:border-stone-600 bg-white dark:bg-stone-800 px-3 py-2 text-sm text-stone-900 dark:text-stone-100"
        />
        {desc}
      </div>
    );
  };

  // ── Render ─────────────────────────────────────────────────────────
  return (
    <div className="flex flex-col h-full">
      <SettingsHeader
        title={t('settings.developerMenu.skillsRunner.title')}
        showBackButton={true}
        onBack={navigateBack}
        breadcrumbs={breadcrumbs}
      />

      <div className="flex-1 overflow-y-auto p-6 space-y-6">
        <div className="text-sm text-stone-600 dark:text-stone-400">
          {t('settings.developerMenu.skillsRunner.panelDesc')}
        </div>

        {/* Skill picker */}
        <div>
          <label
            htmlFor="skills-runner-skill"
            className="block text-sm font-medium text-stone-700 dark:text-stone-300 mb-1"
          >
            {t('settings.skillsRunner.skill')}
          </label>
          <select
            id="skills-runner-skill"
            value={selectedSkillId}
            onChange={(e) => setSelectedSkillId(e.target.value)}
            disabled={skillsLoading || skillsError !== null}
            className="w-full rounded border border-stone-300 dark:border-stone-600 bg-white dark:bg-stone-800 px-3 py-2 text-sm text-stone-900 dark:text-stone-100"
          >
            <option value="">
              {skillsLoading
                ? t('settings.skillsRunner.loadingSkills')
                : t('settings.skillsRunner.selectSkill')}
            </option>
            {skills.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name || s.id}
              </option>
            ))}
          </select>
          {skillsError && (
            <p className="text-xs text-red-600 dark:text-red-400 mt-1">
              {t('settings.skillsRunner.error.listSkills')} {skillsError}
            </p>
          )}
        </div>

        {/* Description + form */}
        {selectedSkillId && (
          <>
            {descLoading && (
              <div className="text-sm text-stone-500 dark:text-stone-400">
                {t('settings.skillsRunner.loadingDescription')}
              </div>
            )}
            {descError && (
              <div className="text-sm text-red-600 dark:text-red-400">
                {t('settings.skillsRunner.error.describe')} {descError}
              </div>
            )}
            {description && (
              <>
                <div className="rounded border border-stone-200 dark:border-stone-700 bg-stone-50 dark:bg-stone-900 p-3">
                  <p className="text-sm text-stone-700 dark:text-stone-300 whitespace-pre-wrap">
                    {description.when_to_use}
                  </p>
                </div>

                {description.inputs.length === 0 ? (
                  <p className="text-sm italic text-stone-500 dark:text-stone-400">
                    {t('settings.skillsRunner.noInputs')}
                  </p>
                ) : (
                  <div className="space-y-4">
                    {description.inputs.map((inp) =>
                      renderField(inp, formValues[inp.name] ?? defaultForType(inp.type), (next) =>
                        setFormValues((prev) => ({ ...prev, [inp.name]: next }))
                      )
                    )}
                  </div>
                )}

                {/* Run Now */}
                <div className="pt-2 flex flex-col gap-2">
                  <button
                    type="button"
                    onClick={() => void handleRun()}
                    disabled={run.status === 'submitting' || missingRequired.length > 0}
                    className="self-start rounded bg-primary-600 hover:bg-primary-700 disabled:opacity-50 px-4 py-2 text-sm font-medium text-white"
                  >
                    {run.status === 'submitting'
                      ? t('settings.skillsRunner.starting')
                      : t('settings.skillsRunner.runNow')}
                  </button>

                  {run.status === 'started' && run.result && (
                    <div className="rounded border border-emerald-300 dark:border-emerald-700 bg-emerald-50 dark:bg-emerald-950 p-3 text-sm">
                      <p className="text-emerald-800 dark:text-emerald-200">
                        {t('settings.skillsRunner.started')} {run.result.run_id}
                      </p>
                      <p className="text-xs text-emerald-700 dark:text-emerald-300 mt-1 break-all">
                        {t('settings.skillsRunner.logPath')}{' '}
                        <code>{run.result.log}</code>
                      </p>
                    </div>
                  )}
                  {run.status === 'error' && (
                    <div className="rounded border border-red-300 dark:border-red-700 bg-red-50 dark:bg-red-950 p-3 text-sm">
                      <p className="text-red-800 dark:text-red-200">
                        {t('settings.skillsRunner.error.run')} {run.message ?? ''}
                      </p>
                    </div>
                  )}
                </div>
              </>
            )}
          </>
        )}
      </div>
    </div>
  );
};

export default SkillsRunnerPanel;
