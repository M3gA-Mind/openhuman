/**
 * /skills — landing dashboard.
 *
 * Phase 2 stub: keeps the route mountable + smoke-testable while the
 * real dashboard (Phase 3) lands. Renders a placeholder card with the
 * two CTAs the IA spec wires up — [+ Create a Skill] / [▷ Run a Skill]
 * — so the routing change is visible and tab-bar nav still resolves.
 */
import { useNavigate } from 'react-router-dom';

import { useT } from '../lib/i18n/I18nContext';

export default function SkillsDashboard() {
  const { t } = useT();
  const navigate = useNavigate();

  return (
    <div className="min-h-full flex flex-col">
      <div className="flex-1 flex items-start justify-center p-4 pt-6">
        <div className="w-full max-w-3xl space-y-4">
          <div className="flex items-center justify-between gap-2">
            <h1 className="text-base font-semibold text-stone-900 dark:text-neutral-100">
              {t('skills.dashboard.title')}
            </h1>
            <div className="flex items-center gap-2">
              <button
                type="button"
                data-testid="skills-dashboard-create"
                onClick={() => navigate('/skills/new')}
                className="rounded-lg border border-stone-200 dark:border-neutral-700 bg-white dark:bg-neutral-900 px-3 py-2 text-xs font-medium text-stone-700 dark:text-neutral-200 shadow-soft transition-colors hover:bg-stone-50 dark:hover:bg-neutral-800"
              >
                + {t('skills.dashboard.create')}
              </button>
              <button
                type="button"
                data-testid="skills-dashboard-run"
                onClick={() => navigate('/skills/run')}
                className="rounded-lg bg-primary-500 px-3 py-2 text-xs font-semibold text-white shadow-soft transition-colors hover:bg-primary-600"
              >
                ▷ {t('skills.dashboard.run')}
              </button>
            </div>
          </div>
          <div
            data-testid="skills-dashboard-placeholder"
            className="rounded-2xl border border-stone-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 p-6 shadow-soft text-sm text-stone-600 dark:text-neutral-400"
          >
            {t('skills.dashboard.emptyBody')}
          </div>
        </div>
      </div>
    </div>
  );
}
