/**
 * TransferHandleModal — confirm + execute a Tiny Place handle transfer (GH-4929).
 *
 * A handle transfer is DESTRUCTIVE and irreversible for the sender: on success
 * the recipient becomes the handle's sole owner. So this modal states that
 * plainly, requires an explicit recipient and an explicit confirm click, and
 * fails **closed** — on any error it keeps the dialog open with the message and
 * never reports success. The core handler resolves the recipient @handle and
 * read-back-confirms the new owner before this promise resolves, so a resolved
 * transfer means the reassignment actually landed.
 */
import debugFactory from 'debug';
import { useCallback, useState } from 'react';

import Button from '../../components/ui/Button';
import { ModalShell } from '../../components/ui/ModalShell';
import { useT } from '../../lib/i18n/I18nContext';
import { apiClient } from '../AgentWorldShell';

const debug = debugFactory('agentworld:identity');

export interface TransferHandleModalProps {
  /** The handle being transferred away (without a leading @). */
  handle: string;
  onClose: () => void;
  /** Called after a confirmed, read-back-verified transfer. */
  onTransferred: () => void;
}

export default function TransferHandleModal({
  handle,
  onClose,
  onTransferred,
}: TransferHandleModalProps) {
  const { t } = useT();
  const [recipient, setRecipient] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = useCallback(async () => {
    const target = recipient.trim().replace(/^@+/, '');
    if (!target) {
      setError(t('agentWorld.transferHandle.recipientRequired'));
      return;
    }
    setSubmitting(true);
    setError(null);
    // Never log the handle or recipient — both identify a user.
    debug('[agentworld:identity] handle transfer requested');
    try {
      await apiClient.registry.transfer(handle, target);
      debug('[agentworld:identity] handle transfer confirmed');
      onTransferred();
      onClose();
    } catch (err) {
      // Fail closed: keep the dialog open, show why, report no success.
      // Log only the status (no raw error — it can carry backend/SDK detail);
      // the raw message still surfaces in the UI via setError.
      debug('[agentworld:identity] handle transfer failed');
      setError(String(err));
      setSubmitting(false);
    }
  }, [recipient, handle, t, onTransferred, onClose]);

  return (
    <ModalShell
      title={t('agentWorld.transferHandle.title')}
      titleId="agentworld-transfer-handle-title"
      maxWidthClassName="max-w-sm"
      onClose={submitting ? () => undefined : onClose}>
      <div className="space-y-4" data-testid="transfer-handle-modal">
        <p className="text-sm text-content">@{handle.replace(/^@+/, '')}</p>
        <p className="text-xs text-red-600 dark:text-red-400">
          {t('agentWorld.transferHandle.warning')}
        </p>

        <input
          type="text"
          value={recipient}
          onChange={e => {
            setRecipient(e.target.value);
            setError(null);
          }}
          disabled={submitting}
          placeholder={t('agentWorld.transferHandle.recipientPlaceholder')}
          aria-label={t('agentWorld.transferHandle.recipientPlaceholder')}
          className="w-full rounded-md border border-line-strong bg-surface px-3 py-2 text-sm text-content placeholder-content-faint outline-none focus:border-primary-500"
        />

        {error && (
          <p className="text-xs text-red-600 dark:text-red-400" data-testid="transfer-handle-error">
            {error}
          </p>
        )}

        <div className="flex justify-end gap-2">
          <Button variant="secondary" size="sm" onClick={onClose} disabled={submitting}>
            {t('common.cancel')}
          </Button>
          <Button
            variant="primary"
            size="sm"
            tone="danger"
            onClick={() => void submit()}
            disabled={submitting || !recipient.trim()}
            data-testid="transfer-handle-confirm">
            {submitting
              ? t('agentWorld.transferHandle.submitting')
              : t('agentWorld.transferHandle.confirm')}
          </Button>
        </div>
      </div>
    </ModalShell>
  );
}
