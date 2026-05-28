/**
 * /skills/new — full-page Create-a-Skill authoring view.
 *
 * Phase 2 stub: real form arrives in Phase 6 (extracts CreateSkillForm
 * from CreateSkillModal). For now it's a placeholder so the route
 * resolves and the dashboard's [+ Create a Skill] CTA navigates
 * somewhere meaningful.
 */
import { useNavigate } from 'react-router-dom';

import { useT } from '../lib/i18n/I18nContext';

export default function SkillNew() {
  const { t } = useT();
  const navigate = useNavigate();

  return (
    <div className="min-h-full flex flex-col">
      <div className="flex-1 flex items-start justify-center p-4 pt-6">
        <div className="w-full max-w-3xl space-y-4">
          <div className="flex items-center justify-between gap-2">
            <h1 className="text-base font-semibold text-stone-900 dark:text-neutral-100">
              {t('skills.new.title')}
            </h1>
            <button
              type="button"
              data-testid="skills-new-back"
              onClick={() => navigate('/skills')}
              className="rounded-lg border border-stone-200 dark:border-neutral-700 bg-white dark:bg-neutral-900 px-3 py-2 text-xs font-medium text-stone-700 dark:text-neutral-200 shadow-soft transition-colors hover:bg-stone-50 dark:hover:bg-neutral-800"
            >
              {t('common.back')}
            </button>
          </div>
          <div
            data-testid="skills-new-placeholder"
            className="rounded-2xl border border-stone-200 dark:border-neutral-800 bg-white dark:bg-neutral-900 p-6 shadow-soft text-sm text-stone-600 dark:text-neutral-400"
          >
            {t('skills.new.placeholderBody')}
          </div>
        </div>
      </div>
    </div>
  );
}
