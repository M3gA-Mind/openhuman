/**
 * The voice provider key / Piper install modal.
 *
 * `VoicePanel.test.tsx` renders this component for real (it is not mocked) but
 * reaches it through one test — "opens the install modal when the Piper chip is
 * clicked". That proves the modal mounts; it never varies the install status, so
 * the six states the install button renders, the status line beside it, and the
 * guard that stops a save being dismissed mid-flight are all unexercised.
 *
 * Those states are the whole point of the control: a user whose Piper install is
 * `broken` must be offered "Repair", not "Install Locally", and must be told what
 * broke. Getting that wrong is invisible to every existing test.
 *
 * Driven through props rather than through VoicePanel, because every input this
 * component branches on IS a prop — going through the parent would mean
 * simulating an install pipeline to assert a label.
 *
 * `t` is a prop here, so it is passed as the identity function and assertions are
 * on i18n keys. That keeps them stable against copy edits.
 */
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import type { VoiceInstallStatus } from '../../../../services/api/voiceInstallApi';
import VoicePanelKeyModal from '../VoicePanelKeyModal';

const testVoiceProvider = vi.fn();
vi.mock('../../../../services/api/voiceSettingsApi', () => ({
  testVoiceProvider: (...args: unknown[]) => testVoiceProvider(...args),
}));

type Props = React.ComponentProps<typeof VoicePanelKeyModal>;

function renderModal(overrides: Partial<Props> = {}) {
  const setPendingKeySlug = vi.fn();
  const setPendingKeyValue = vi.fn();
  const handleEnableExternalProvider = vi.fn().mockResolvedValue(undefined);
  const handleInstallPiper = vi.fn().mockResolvedValue(undefined);
  const persistProviders = vi.fn().mockResolvedValue(undefined);
  const onTtsProviderChange = vi.fn();
  const setTtsVoice = vi.fn();

  const props: Props = {
    t: (key: string) => key,
    pendingKeySlug: 'piper',
    setPendingKeySlug,
    pendingKeyValue: '',
    setPendingKeyValue,
    isSavingPendingKey: false,
    handleEnableExternalProvider,
    ttsVoice: 'en_US-amy-medium',
    setTtsVoice,
    piperVoicePresets: [{ id: 'en_US-amy-medium', label: 'Amy' }],
    piperVoicePresetIds: ['en_US-amy-medium'],
    piperInstall: null,
    isInstallingPiper: false,
    handleInstallPiper,
    piperReady: false,
    pendingLocalProviderReady: false,
    isSavingProviders: false,
    onTtsProviderChange,
    persistProviders,
    ...overrides,
  };

  render(<VoicePanelKeyModal {...props} />);
  return {
    setPendingKeySlug,
    setPendingKeyValue,
    handleEnableExternalProvider,
    handleInstallPiper,
    persistProviders,
    onTtsProviderChange,
  };
}

const status = (over: Partial<VoiceInstallStatus>): VoiceInstallStatus =>
  ({ state: 'idle', ...over }) as VoiceInstallStatus;

const installButton = () =>
  screen
    .getAllByRole('button')
    .find(b => /voice\.providers\.(install|reinstall|repair|retry)/.test(b.textContent ?? ''))!;

describe('VoicePanelKeyModal — the install button names the state it is in', () => {
  // Each row is a distinct affordance. Offering "Install Locally" over a broken
  // install sends the user round a loop that cannot fix anything.
  const CASES: Array<{
    name: string;
    install: VoiceInstallStatus | null;
    busy?: boolean;
    label: string;
  }> = [
    { name: 'nothing installed yet', install: null, label: 'voice.providers.installLocally' },
    {
      name: 'already installed',
      install: status({ state: 'installed' }),
      label: 'voice.providers.reinstallLocally',
    },
    {
      name: 'installed but broken',
      install: status({ state: 'broken' }),
      label: 'voice.providers.repair',
    },
    {
      name: 'a previous attempt errored',
      install: status({ state: 'error' }),
      label: 'voice.providers.retryLocally',
    },
  ];

  for (const { name, install, label } of CASES) {
    it(`offers ${label} when ${name}`, () => {
      renderModal({ piperInstall: install });
      expect(installButton()).toHaveTextContent(label);
    });
  }

  it('shows live percentage while installing', () => {
    renderModal({ piperInstall: status({ state: 'installing', progress: 42 }) });
    expect(installButton()).toHaveTextContent('voice.providers.installing 42%');
  });

  it('falls back to an ellipsis when installing without a percentage', () => {
    // `progress` is optional on the wire; rendering "Installing undefined%"
    // is the failure this branch exists to avoid.
    renderModal({ piperInstall: status({ state: 'installing' }) });
    expect(installButton()).toHaveTextContent('voice.providers.ellipsis');
    expect(installButton()).not.toHaveTextContent('undefined');
  });

  it('prefers the remote installing state over the local busy flag', () => {
    // The install RPC is fire-and-forget: `busy` only covers click→return, while
    // the polled status is the real signal. If `busy` won, a running install
    // would flip back to the generic busy label on every poll.
    renderModal({
      piperInstall: status({ state: 'installing', progress: 5 }),
      isInstallingPiper: true,
    });
    expect(installButton()).toHaveTextContent('voice.providers.installing 5%');
  });

  it('is disabled while an install is already running', () => {
    // Double-installing is the reason this is here; the click handler has no
    // re-entrancy guard of its own.
    renderModal({ piperInstall: status({ state: 'installing', progress: 10 }) });
    expect(installButton()).toBeDisabled();
  });
});

describe('VoicePanelKeyModal — the status line tells the user what happened', () => {
  it('surfaces the backend error detail rather than a generic failure', () => {
    renderModal({
      piperInstall: status({
        state: 'error',
        error_detail: 'checksum mismatch on piper-1.2.tar.gz',
      }),
    });
    expect(screen.getByText('checksum mismatch on piper-1.2.tar.gz')).toBeInTheDocument();
  });

  it('falls back to a generic failure when the backend gave no detail', () => {
    renderModal({ piperInstall: status({ state: 'error' }) });
    expect(screen.getByText('voice.providers.installFailed')).toBeInTheDocument();
  });

  it('appends the current stage while installing', () => {
    renderModal({
      piperInstall: status({ state: 'installing', progress: 7, stage: 'downloading' }),
    });
    expect(screen.getByText('voice.providers.installing 7% · downloading')).toBeInTheDocument();
  });

  it('reports installed once the provider is ready', () => {
    renderModal({ piperInstall: status({ state: 'installed' }), piperReady: true });
    expect(screen.getByText('voice.providers.installed')).toBeInTheDocument();
  });
});

describe('VoicePanelKeyModal — Piper enable guard', () => {
  it('refuses to enable Piper before the local provider is ready', () => {
    // Switching TTS routing to a provider that is not installed leaves the user
    // with silent speech and no error.
    //
    // Only the `disabled` prop is asserted, deliberately. The handler also opens
    // with `if (!pendingLocalProviderReady) return;`, but that line is
    // unreachable — it is guarded by the same condition as `disabled`, so a
    // click never arrives. Verified: deleting that `return` fails nothing.
    // Asserting "the handler did not fire" here would therefore be testing the
    // disabled attribute twice and reporting it as coverage of the guard.
    renderModal({ pendingLocalProviderReady: false });

    expect(screen.getByRole('button', { name: 'voice.modal.enable' })).toBeDisabled();
  });

  it('switches routing and persists the chosen voice once ready', () => {
    const { onTtsProviderChange, persistProviders, setPendingKeySlug } = renderModal({
      pendingLocalProviderReady: true,
    });

    fireEvent.click(screen.getByRole('button', { name: 'voice.modal.enable' }));

    expect(onTtsProviderChange).toHaveBeenCalledWith('piper');
    expect(persistProviders).toHaveBeenCalledWith({ tts_voice: 'en_US-amy-medium' });
    expect(setPendingKeySlug).toHaveBeenCalledWith(null);
  });
});

describe('VoicePanelKeyModal — the API key field', () => {
  it('is a password field with autofill suppressed', () => {
    // Same contract ComposioPanel pins for its key field: a provider secret must
    // not be shoulder-surfable, and must not be captured as a login by a
    // password manager.
    renderModal({ pendingKeySlug: 'deepgram' });

    const input = document.getElementById('voice-provider-key-input')!;
    expect(input).toHaveAttribute('type', 'password');
    expect(input).toHaveAttribute('autocomplete', 'off');
    expect(input).toHaveAttribute('data-lpignore', 'true');
  });

  it('disables Cancel while the key is being saved', () => {
    renderModal({
      pendingKeySlug: 'deepgram',
      pendingKeyValue: 'sk-live-abc',
      isSavingPendingKey: true,
    });

    expect(screen.getByRole('button', { name: 'common.cancel' })).toBeDisabled();
  });

  it('ignores the X close button while the key is being saved', () => {
    // ModalShell's own X is wired straight to `onClose` and is NOT disabled by
    // this component, so the disabled Cancel above is not the whole guard —
    // `close()` returning early when a save is in flight is. Dismissing here
    // would clear the pending key while the RPC is still in flight, and the
    // modal would not be there to report the result.
    const { setPendingKeySlug, setPendingKeyValue } = renderModal({
      pendingKeySlug: 'deepgram',
      pendingKeyValue: 'sk-live-abc',
      isSavingPendingKey: true,
    });

    const close = screen.getByRole('button', { name: /close/i });
    expect(close).toBeEnabled();

    fireEvent.click(close);
    expect(setPendingKeySlug).not.toHaveBeenCalled();
    expect(setPendingKeyValue).not.toHaveBeenCalled();
  });

  it('lets the X close button dismiss when no save is in flight', () => {
    const { setPendingKeySlug } = renderModal({
      pendingKeySlug: 'deepgram',
      pendingKeyValue: 'sk-live-abc',
      isSavingPendingKey: false,
    });

    fireEvent.click(screen.getByRole('button', { name: /close/i }));
    expect(setPendingKeySlug).toHaveBeenCalledWith(null);
  });

  it('dismisses normally when no save is in flight', () => {
    const { setPendingKeySlug } = renderModal({
      pendingKeySlug: 'deepgram',
      pendingKeyValue: 'sk-live-abc',
      isSavingPendingKey: false,
    });

    fireEvent.click(screen.getByRole('button', { name: 'common.cancel' }));
    expect(setPendingKeySlug).toHaveBeenCalledWith(null);
  });

  it('disables both actions until a non-blank key is entered', () => {
    // A whitespace-only key would otherwise be saved and then fail server-side.
    renderModal({ pendingKeySlug: 'deepgram', pendingKeyValue: '   ' });

    expect(screen.getByRole('button', { name: 'voice.modal.testKey' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'voice.modal.saveAndEnable' })).toBeDisabled();
  });
});

describe('VoicePanelKeyModal — a failed key test is reported in the modal', () => {
  it('shows the thrown message rather than swallowing it', async () => {
    // The success path of this alert is unreachable today — see the bug note in
    // ~/tinyhuman/bugs/W6-routes-e2e-bugs.md. The failure path is reachable and
    // is the one a user with a bad key actually hits.
    const boom = vi.fn().mockRejectedValue(new Error('401 invalid api key'));
    renderModal({
      pendingKeySlug: 'deepgram',
      pendingKeyValue: 'sk-bad',
      handleEnableExternalProvider: boom,
    });

    fireEvent.click(screen.getByRole('button', { name: 'voice.modal.testKey' }));

    expect(await screen.findByText('401 invalid api key')).toBeInTheDocument();
    expect(testVoiceProvider).not.toHaveBeenCalled();
  });
});
