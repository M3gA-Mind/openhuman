import { useCallback, useEffect, useState } from 'react';

import { useT } from '../../../lib/i18n/I18nContext';
import { fetchWalletStatus, type WalletAccount, type WalletStatus } from '../../../services/walletApi';
import SettingsHeader from '../components/SettingsHeader';
import { useSettingsNavigation } from '../hooks/useSettingsNavigation';

const CHAIN_META: Record<
  WalletAccount['chain'],
  { label: string; signsBaseSwap: boolean; networks: string[] }
> = {
  evm: {
    label: 'Ethereum & EVM L2s',
    signsBaseSwap: true,
    networks: ['Ethereum', 'Base', 'Arbitrum', 'Optimism', 'Polygon'],
  },
  btc: { label: 'Bitcoin', signsBaseSwap: false, networks: ['Bitcoin (P2WPKH)'] },
  solana: { label: 'Solana', signsBaseSwap: false, networks: ['Solana Mainnet'] },
  tron: { label: 'Tron', signsBaseSwap: false, networks: ['Tron Mainnet'] },
};

const WalletAddressesPanel = () => {
  const { t } = useT();
  const { navigateBack, breadcrumbs } = useSettingsNavigation();
  const [status, setStatus] = useState<WalletStatus | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [copiedAddress, setCopiedAddress] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const next = await fetchWalletStatus();
      setStatus(next);
    } catch (error) {
      console.error('[wallet-addresses-panel] failed to load status', error);
      setLoadError(error instanceof Error ? error.message : String(error));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const copyToClipboard = useCallback(async (address: string) => {
    try {
      await navigator.clipboard.writeText(address);
      setCopiedAddress(address);
      window.setTimeout(() => setCopiedAddress(current => (current === address ? null : current)), 1500);
    } catch (error) {
      console.warn('[wallet-addresses-panel] clipboard write failed', error);
    }
  }, []);

  return (
    <div className="flex flex-col">
      <SettingsHeader
        title={t('pages.settings.account.walletAddresses')}
        showBackButton
        onBack={navigateBack}
        breadcrumbs={breadcrumbs}
      />

      <div className="px-5 pb-6 space-y-4">
        <p className="text-xs text-stone-500 dark:text-neutral-400">
          {t('pages.settings.walletAddresses.intro')}
        </p>

        {loading && (
          <p className="text-xs text-stone-400 dark:text-neutral-500">
            {t('common.loading')}
          </p>
        )}

        {loadError && !loading && (
          <div className="text-xs text-coral-600 dark:text-coral-400 bg-coral-50 dark:bg-coral-950/30 border border-coral-200 dark:border-coral-900 rounded-lg px-3 py-2">
            {loadError}
          </div>
        )}

        {!loading && !loadError && status && !status.configured && (
          <div className="text-xs text-stone-600 dark:text-neutral-300 bg-stone-50 dark:bg-neutral-800/50 border border-stone-200 dark:border-neutral-700 rounded-lg px-3 py-3">
            {t('pages.settings.walletAddresses.notConfigured')}
          </div>
        )}

        {!loading && !loadError && status?.accounts?.length ? (
          <ul className="space-y-3">
            {status.accounts.map(account => {
              const meta = CHAIN_META[account.chain];
              const isBaseSigner = meta?.signsBaseSwap ?? false;
              return (
                <li
                  key={`${account.chain}-${account.address}`}
                  className={`rounded-xl border px-4 py-3 ${
                    isBaseSigner
                      ? 'border-ocean-300 bg-ocean-50/60 dark:border-ocean-800 dark:bg-ocean-950/30'
                      : 'border-stone-200 dark:border-neutral-800 bg-white dark:bg-neutral-900'
                  }`}>
                  <div className="flex items-center justify-between gap-2">
                    <div className="flex items-center gap-2 min-w-0">
                      <span className="text-sm font-semibold text-stone-900 dark:text-neutral-100 truncate">
                        {meta?.label ?? account.chain.toUpperCase()}
                      </span>
                      {isBaseSigner && (
                        <span className="text-[10px] uppercase tracking-wide font-medium text-ocean-700 dark:text-ocean-300 bg-ocean-100 dark:bg-ocean-900/50 rounded px-1.5 py-0.5 whitespace-nowrap">
                          {t('pages.settings.walletAddresses.baseSignerTag')}
                        </span>
                      )}
                    </div>
                    <button
                      type="button"
                      onClick={() => void copyToClipboard(account.address)}
                      className="text-[11px] text-ocean-600 dark:text-ocean-400 hover:underline whitespace-nowrap">
                      {copiedAddress === account.address
                        ? t('common.copied')
                        : t('common.copy')}
                    </button>
                  </div>
                  <div className="mt-1.5 text-[11px] text-stone-500 dark:text-neutral-400">
                    {meta?.networks.join(' · ')}
                  </div>
                  <code className="mt-2 block text-xs font-mono break-all text-stone-800 dark:text-neutral-200">
                    {account.address}
                  </code>
                  <div className="mt-1 text-[10px] font-mono text-stone-400 dark:text-neutral-500">
                    {account.derivationPath}
                  </div>
                </li>
              );
            })}
          </ul>
        ) : null}
      </div>
    </div>
  );
};

export default WalletAddressesPanel;
