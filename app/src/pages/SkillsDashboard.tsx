/**
 * /skills — landing dashboard.
 *
 * Lists the user's currently-scheduled skills as DevWorkflowPanel-style
 * "active config" cards (one per cron job whose name starts with the
 * SkillsRunnerBody prefix `skill-run-`). Each card shows the skill_id,
 * a human-readable schedule, last/next run, and an enable/disable
 * toggle that mirrors DevWorkflowPanel:439's update-then-reload pattern
 * verbatim. Click anywhere else on the card → /skills/run?skill=<id>
 * so the user lands in the runner with the right skill pre-picked.
 *
 * The dashboard *only* surfaces cron-scheduled skills. The catalog of
 * available skills, integrations, etc. lives on /skills/run; the
 * dashboard is deliberately a "what's running on a schedule" view so
 * users can see at a glance what their agent is autonomously doing.
 */
import createDebug from 'debug';
import { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';

import { cronToHuman } from '../lib/cron/cronToHuman';
import { useT } from '../lib/i18n/I18nContext';
import {
  type CoreCronJob,
  openhumanCronList,
  openhumanCronUpdate,
} from '../utils/tauriCommands/cron';

const log = createDebug('app:pages:SkillsDashboard');

/** Same prefix SkillsRunnerBody.tsx uses to namespace its cron jobs. */
const CRON_NAME_PREFIX = 'skill-run-';

/**
 * Extract the skill_id from a cron job name. Format is
 *   `skill-run-<skill_id>[-input1=v1_input2=v2…]`
 * The first `-` after the prefix delimits the skill_id from the
 * input-encoded suffix. If we can't recognise the shape (e.g. the
 * job pre-dates the convention), fall back to the full name minus
 * prefix so users still see *something* identifying.
 */
function extractSkillId(jobName: string): string {
  const tail = jobName.startsWith(CRON_NAME_PREFIX)
    ? jobName.slice(CRON_NAME_PREFIX.length)
    : jobName;
  // Split on the first `-input=` marker (input pairs always contain `=`).
  const eqIdx = tail.indexOf('=');
  if (eqIdx === -1) return tail;
  // Walk back from `=` to the last `-` before it — that's the input-pair separator.
  const dashBeforeEq = tail.lastIndexOf('-', eqIdx);
  if (dashBeforeEq === -1) return tail;
  return tail.slice(0, dashBeforeEq);
}

/**
 * Pull the cron expression out of the schedule discriminated-union.
 * Today only `kind: 'cron'` carries an `expr`; the other variants
 * (`at`, `every`) render their own shape.
 */
function formatSchedule(job: CoreCronJob): string {
  const s = job.schedule as { kind?: string; expr?: string; at?: string; every_ms?: number };
  if (!s) return job.expression ?? '';
  if (s.kind === 'cron' && s.expr) return cronToHuman(s.expr);
  if (s.kind === 'at' && s.at) return new Date(s.at).toLocaleString();
  if (s.kind === 'every' && s.every_ms) {
    const minutes = Math.round(s.every_ms / 60_000);
    return `Every ${minutes} minutes`;
  }
  return cronToHuman(job.expression ?? '');
}

/** Group jobs by skill_id and present a single card per skill (newest first). */
interface SkillGroup {
  skillId: string;
  jobs: CoreCronJob[];
  /** The representative job — the most recently active one. */
  primary: CoreCronJob;
}

function groupBySkill(jobs: CoreCronJob[]): SkillGroup[] {
  const byId = new Map<string, CoreCronJob[]>();
  for (const job of jobs) {
    const name = job.name ?? '';
    if (!name.startsWith(CRON_NAME_PREFIX)) continue;
    const skillId = extractSkillId(name);
    const bucket = byId.get(skillId);
    if (bucket) {
      bucket.push(job);
    } else {
      byId.set(skillId, [job]);
    }
  }
  const groups: SkillGroup[] = [];
  for (const [skillId, list] of byId.entries()) {
    // Pick "primary": enabled-with-most-recent-last_run beats enabled
    // beats disabled, fall back to created_at desc for stability.
    const sorted = [...list].sort((a, b) => {
      if (a.enabled !== b.enabled) return a.enabled ? -1 : 1;
      const aTs = a.last_run ? new Date(a.last_run).getTime() : 0;
      const bTs = b.last_run ? new Date(b.last_run).getTime() : 0;
      if (aTs !== bTs) return bTs - aTs;
      return new Date(b.created_at).getTime() - new Date(a.created_at).getTime();
    });
    groups.push({ skillId, jobs: sorted, primary: sorted[0] });
  }
  // Order skills by primary's enabled-then-last_run; matches the
  // DevWorkflowPanel sort intent (active surface first).
  groups.sort((a, b) => {
    if (a.primary.enabled !== b.primary.enabled) return a.primary.enabled ? -1 : 1;
    const aTs = a.primary.last_run ? new Date(a.primary.last_run).getTime() : 0;
    const bTs = b.primary.last_run ? new Date(b.primary.last_run).getTime() : 0;
    return bTs - aTs;
  });
  return groups;
}

export default function SkillsDashboard() {
  const { t } = useT();
  const navigate = useNavigate();

  const [jobs, setJobs] = useState<CoreCronJob[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  // Per-job "busy" key so we can disable the toggle while update is in
  // flight — mirrors CronJobsPanel's `coreBusyKey` pattern.
  const [busyJobId, setBusyJobId] = useState<string | null>(null);

  const loadJobs = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const resp = await openhumanCronList();
      const all = (resp.result ?? []) as CoreCronJob[];
      const filtered = all.filter((j) => (j.name ?? '').startsWith(CRON_NAME_PREFIX));
      log('loaded %d skill cron jobs (of %d total)', filtered.length, all.length);
      setJobs(filtered);
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      log('loadJobs error: %s', msg);
      setError(msg);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadJobs();
  }, [loadJobs]);

  // Mirror DevWorkflowPanel:439 verbatim — flip enabled, refresh the
  // list. We keep this generic on the job rather than the skill so
  // it works for any cron-backed skill.
  const handleToggle = useCallback(
    async (job: CoreCronJob) => {
      setBusyJobId(job.id);
      try {
        await openhumanCronUpdate(job.id, { enabled: !job.enabled });
        await loadJobs();
      } catch (err: unknown) {
        log('toggle error: %s', err instanceof Error ? err.message : String(err));
      } finally {
        setBusyJobId(null);
      }
    },
    [loadJobs]
  );

  const groups = useMemo(() => groupBySkill(jobs), [jobs]);

  const goCreate = () => navigate('/skills/new');
  const goRun = () => navigate('/skills/run');
  const goRunSkill = (skillId: string) =>
    navigate(`/skills/run?skill=${encodeURIComponent(skillId)}`);

  return (
    <div className="min-h-full flex flex-col">
      <div className="flex-1 flex items-start justify-center p-4 pt-6">
        <div className="w-full max-w-3xl space-y-4">
          {/* Header + CTAs */}
          <div className="flex items-center justify-between gap-2">
            <h1 className="text-base font-semibold text-stone-900 dark:text-neutral-100">
              {t('skills.dashboard.title')}
            </h1>
            <div className="flex items-center gap-2">
              <button
                type="button"
                data-testid="skills-dashboard-create"
                onClick={goCreate}
                className="rounded-lg border border-stone-200 dark:border-neutral-700 bg-white dark:bg-neutral-900 px-3 py-2 text-xs font-medium text-stone-700 dark:text-neutral-200 shadow-soft transition-colors hover:bg-stone-50 dark:hover:bg-neutral-800"
              >
                + {t('skills.dashboard.create')}
              </button>
              <button
                type="button"
                data-testid="skills-dashboard-run"
                onClick={goRun}
                className="rounded-lg bg-primary-500 px-3 py-2 text-xs font-semibold text-white shadow-soft transition-colors hover:bg-primary-600"
              >
                ▷ {t('skills.dashboard.run')}
              </button>
            </div>
          </div>

          {/* Section heading — kept above whatever state the list is in */}
          <h2 className="text-xs font-semibold uppercase tracking-wider text-stone-500 dark:text-neutral-400 px-1">
            {t('skills.dashboard.scheduledHeading')}
          </h2>

          {loading && (
            <div
              data-testid="skills-dashboard-loading"
              className="rounded-2xl border border-stone-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 p-6 shadow-soft text-sm text-stone-500 dark:text-neutral-400"
            >
              {t('common.loading')}
            </div>
          )}

          {!loading && error && (
            <div
              data-testid="skills-dashboard-error"
              className="rounded-2xl border border-coral-200 bg-coral-50 dark:bg-coral-500/10 dark:border-coral-500/30 p-4 text-sm"
            >
              <p className="text-coral-800 dark:text-coral-200">
                {t('skills.dashboard.loadError')}: {error}
              </p>
              <button
                type="button"
                onClick={() => void loadJobs()}
                className="mt-2 rounded border border-coral-300 dark:border-coral-500/40 bg-white dark:bg-neutral-900 px-3 py-1.5 text-xs font-medium text-coral-700 dark:text-coral-300 hover:bg-coral-100 dark:hover:bg-coral-500/15"
              >
                {t('common.retry')}
              </button>
            </div>
          )}

          {!loading && !error && groups.length === 0 && (
            <div
              data-testid="skills-dashboard-empty"
              className="rounded-2xl border border-stone-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 p-8 shadow-soft text-center"
            >
              <h3 className="text-sm font-semibold text-stone-900 dark:text-neutral-100">
                {t('skills.dashboard.emptyTitle')}
              </h3>
              <p className="mt-1 text-xs text-stone-500 dark:text-neutral-400">
                {t('skills.dashboard.emptyBody')}
              </p>
              <button
                type="button"
                data-testid="skills-dashboard-empty-cta"
                onClick={goRun}
                className="mt-4 rounded-lg bg-primary-500 px-4 py-2 text-xs font-semibold text-white shadow-soft transition-colors hover:bg-primary-600"
              >
                ▷ {t('skills.dashboard.run')}
              </button>
            </div>
          )}

          {!loading && !error && groups.length > 0 && (
            <div className="space-y-2">
              {groups.map((group) => {
                const job = group.primary;
                const isActive = job.enabled;
                const isBusy = busyJobId === job.id;
                return (
                  <div
                    key={group.skillId}
                    data-testid={`skill-card-${group.skillId}`}
                    className={`rounded-2xl border shadow-soft transition-colors ${
                      isActive
                        ? 'border-sage-200 dark:border-sage-500/30 bg-sage-50 dark:bg-sage-500/10'
                        : 'border-stone-200 dark:border-neutral-800 bg-white dark:bg-neutral-900'
                    }`}
                  >
                    {/* Whole-card click → runner with the skill pre-picked.
                        We split into two stacked clickable surfaces so the
                        toggle inside isn't accidentally consuming card-click
                        events. */}
                    <button
                      type="button"
                      data-testid={`skill-card-open-${group.skillId}`}
                      aria-label={t('skills.dashboard.cardOpenRunner')}
                      onClick={() => goRunSkill(group.skillId)}
                      className="w-full text-left px-4 py-3 flex items-center justify-between gap-3 focus:outline-none focus-visible:ring-2 focus-visible:ring-primary-500/40 rounded-2xl"
                    >
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-2">
                          <span
                            className={`font-mono text-sm font-semibold truncate ${
                              isActive
                                ? 'text-sage-900 dark:text-sage-100'
                                : 'text-stone-700 dark:text-neutral-200'
                            }`}
                          >
                            {group.skillId}
                          </span>
                          {group.jobs.length > 1 && (
                            <span className="px-1.5 py-0.5 rounded text-[10px] font-medium bg-stone-200 dark:bg-neutral-700 text-stone-700 dark:text-neutral-300">
                              ×{group.jobs.length}
                            </span>
                          )}
                        </div>
                        <div className="mt-0.5 text-xs text-stone-600 dark:text-neutral-400">
                          {formatSchedule(job)}
                        </div>
                        <div className="mt-1 text-[11px] text-stone-500 dark:text-neutral-500">
                          {job.last_run && (
                            <span>
                              {t('skills.dashboard.lastRun')}:{' '}
                              {new Date(job.last_run).toLocaleString()}
                              {job.last_status && (
                                <span
                                  className={`ml-1.5 px-1 py-0.5 rounded text-[10px] font-medium ${
                                    job.last_status === 'ok'
                                      ? 'bg-sage-100 dark:bg-sage-500/20 text-sage-700 dark:text-sage-300'
                                      : 'bg-coral-100 dark:bg-coral-500/20 text-coral-700 dark:text-coral-300'
                                  }`}
                                >
                                  {job.last_status}
                                </span>
                              )}
                            </span>
                          )}
                          {job.last_run && job.next_run && <span className="mx-1">·</span>}
                          {job.next_run && (
                            <span>
                              {t('skills.dashboard.nextRun')}:{' '}
                              {new Date(job.next_run).toLocaleString()}
                            </span>
                          )}
                        </div>
                      </div>
                      {/* Toggle — DevWorkflowPanel:502-516 style. Wrapped in
                          a span (not a button-inside-button) and onClick
                          stopPropagation so toggling doesn't navigate. */}
                      <span
                        className="flex items-center gap-1.5 shrink-0"
                        onClick={(e) => e.stopPropagation()}
                      >
                        <button
                          type="button"
                          role="switch"
                          aria-checked={job.enabled}
                          aria-label={
                            job.enabled
                              ? t('skills.dashboard.disable')
                              : t('skills.dashboard.enable')
                          }
                          data-testid={`skill-toggle-${group.skillId}`}
                          disabled={isBusy}
                          onClick={() => void handleToggle(job)}
                          className={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full transition-colors disabled:opacity-50 ${
                            job.enabled ? 'bg-sage-500' : 'bg-neutral-300 dark:bg-neutral-600'
                          }`}
                        >
                          <span
                            className={`pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow-sm transition-transform mt-0.5 ${
                              job.enabled ? 'translate-x-4' : 'translate-x-0.5'
                            }`}
                          />
                        </button>
                        <span className="text-[10px] text-stone-500 dark:text-neutral-400 min-w-[44px]">
                          {job.enabled ? t('common.enabled') : t('common.disabled')}
                        </span>
                      </span>
                    </button>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
