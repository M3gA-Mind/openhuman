// DevWorkflowPanel — deprecated thin shell.
//
// The bespoke dev-workflow setup UI (repo + fork detection + branch
// dropdown + cron schedule + run-history with output viewer) has been
// merged into the generic Skills Runner at /skills (see
// docs/skills-runner-unification.md).
//
// This panel is kept as a stub so:
//   - existing deep links / bookmarks to /settings/dev-workflow don't 404,
//   - the Developer Options menu entry still resolves to *something* (a
//     "moved to /skills" notice with a one-click navigation button),
//   - the route can be fully removed in a future release without
//     touching the panel itself.
//
// Everything that used to live here (Composio fetch, fork detection,
// branch list, cron job CRUD, run-history viewer) is now in:
//   - app/src/components/skills/SkillsRunnerBody.tsx (generic UI)
//   - app/src/components/skills/SmartIssuePicker.tsx (dev-workflow's
//     repo/fork/branch picker, conditionally mounted by SkillsRunnerBody
//     when the selected skill is `dev-workflow`).
import { useNavigate } from 'react-router-dom';

import { useT } from '../../../lib/i18n/I18nContext';
import SettingsHeader from '../components/SettingsHeader';
import { useSettingsNavigation } from '../hooks/useSettingsNavigation';

const DevWorkflowPanel = () => {
  const { t } = useT();
  const { navigateBack, breadcrumbs } = useSettingsNavigation();
  const navigate = useNavigate();

  return (
    <div data-testid="dev-workflow-panel" className="z-10 relative">
      <SettingsHeader
        title={t('settings.developerMenu.devWorkflow.title')}
        showBackButton={true}
        onBack={navigateBack}
        breadcrumbs={breadcrumbs}
      />
      <div className="px-4 pt-4 flex flex-col gap-4">
        <div
          data-testid="dev-workflow-moved-notice"
          className="px-4 py-3 rounded-lg border border-sage-200 dark:border-sage-500/30 bg-sage-50 dark:bg-sage-500/10">
          <div className="text-sm font-semibold text-sage-900 dark:text-sage-200">
            {t('settings.devWorkflow.movedHeading')}
          </div>
          <p className="mt-1 text-sm text-sage-800 dark:text-sage-300">
            {t('settings.devWorkflow.movedBody')}
          </p>
          <button
            type="button"
            onClick={() => navigate('/skills')}
            className="mt-3 px-4 py-2 rounded-md bg-primary-600 hover:bg-primary-500 text-white text-sm font-medium transition-colors">
            {t('settings.devWorkflow.movedOpenSkills')}
          </button>
        </div>
      </div>
    </div>
  );
};

export default DevWorkflowPanel;
