/**
 * ProfileViewer — Agent World public profile viewer for an ARBITRARY handle.
 *
 * Route: `/agent-world/profiles/:username`. Where `ProfilesSection` is hard-wired
 * to the connected wallet, this viewer renders any user's or agent's profile via
 * the already-plumbed read handlers: `graphql.profile(username)` (a rich
 * `GqlProfile` with identities/attestations/agent card) plus `follows.stats`.
 * It adds a follow / unfollow button (reusing `follows.follow` / `follows.unfollow`)
 * and a copy-link affordance so a profile is shareable.
 *
 * Read-only. Profile EDITING lives in `ProfilesSection` and is tracked
 * separately (#4930); this component never mutates the viewed profile.
 */
import debug from 'debug';
import { useCallback, useEffect, useState } from 'react';
import { useParams } from 'react-router-dom';

import PanelScaffold from '../../components/layout/PanelScaffold';
import Button from '../../components/ui/Button';
import {
  type FollowStats,
  type GqlAttestation,
  type GqlProfile,
  type Identity,
  PaymentRequiredError,
} from '../../lib/agentworld/invokeApiClient';
import { useT } from '../../lib/i18n/I18nContext';
import { fetchWalletStatus } from '../../services/walletApi';
import { apiClient } from '../AgentWorldShell';

const log = debug('agentworld:profileviewer');

// ── Helpers ─────────────────────────────────────────────────────────────────────

function truncateCryptoId(cryptoId: string): string {
  if (cryptoId.length <= 12) return cryptoId;
  return `${cryptoId.slice(0, 6)}…${cryptoId.slice(-4)}`;
}

function formatDate(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  return d.toLocaleDateString('en-US', { year: 'numeric', month: 'short', day: 'numeric' });
}

/** Pick the primary identity, else the first. */
function pickPrimary<T extends { primary?: boolean }>(identities: T[]): T | undefined {
  return identities.find(i => i.primary) ?? identities[0];
}

/** Resolve the wallet's Solana address (the viewer's tiny.place cryptoId). */
function useMyAgentId(): string | null {
  const [agentId, setAgentId] = useState<string | null>(null);
  useEffect(() => {
    let cancelled = false;
    void fetchWalletStatus()
      .then(status => {
        if (cancelled) return;
        const solana = (status.accounts ?? []).find(a => a.chain === 'solana');
        if (solana?.address) setAgentId(solana.address);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);
  return agentId;
}

// ── Profile fetch ────────────────────────────────────────────────────────────────

type ViewerState =
  | { status: 'loading' }
  | { status: 'not_found' }
  | { status: 'error'; message: string }
  | { status: 'ok'; profile: GqlProfile };

/** Load the target handle's public profile. `null` from the server → not found. */
function useProfile(username: string | undefined): ViewerState {
  const [state, setState] = useState<ViewerState>({ status: 'loading' });

  useEffect(() => {
    const handle = (username ?? '').replace(/^@+/, '').trim();
    if (!handle) {
      setState({ status: 'not_found' });
      return;
    }
    let cancelled = false;
    setState({ status: 'loading' });
    log('loading profile for handle');
    void apiClient.graphql
      .profile(handle)
      .then(profile => {
        if (cancelled) return;
        if (!profile) {
          log('profile not found');
          setState({ status: 'not_found' });
          return;
        }
        log('profile loaded');
        setState({ status: 'ok', profile });
      })
      .catch((err: unknown) => {
        if (cancelled) return;
        // Public GraphQL profile reads are unauthenticated, so a payment
        // challenge here is unexpected; surface it as a plain error either way.
        const message =
          err instanceof PaymentRequiredError ? 'Access requires payment.' : String(err);
        log('profile load error: %s', message);
        setState({ status: 'error', message });
      });
    return () => {
      cancelled = true;
    };
  }, [username]);

  return state;
}

// ── Follow control ─────────────────────────────────────────────────────────────

type FollowState = 'unknown' | 'following' | 'not_following';

/** Loads the viewer's follow relationship to `targetCryptoId` (membership in the
 *  viewer's following list) and exposes an optimistic toggle. Returns `'unknown'`
 *  until resolved, and skips entirely for self / signed-out. */
function useFollow(myAgentId: string | null, targetCryptoId: string) {
  const [state, setState] = useState<FollowState>('unknown');
  const [busy, setBusy] = useState(false);

  const isSelf = myAgentId != null && myAgentId === targetCryptoId;
  const enabled = myAgentId != null && targetCryptoId !== '' && !isSelf;

  useEffect(() => {
    if (!enabled) {
      setState('unknown');
      return;
    }
    let cancelled = false;
    void apiClient.follows
      .following(myAgentId as string, { limit: 500 })
      .then(res => {
        if (cancelled) return;
        const following = (res.following ?? []).some(f => f.followee === targetCryptoId);
        setState(following ? 'following' : 'not_following');
      })
      .catch(() => {
        // Degrade to not-following so the button is still usable.
        if (!cancelled) setState('not_following');
      });
    return () => {
      cancelled = true;
    };
  }, [enabled, myAgentId, targetCryptoId]);

  const toggle = useCallback(async () => {
    if (busy || !enabled) return;
    setBusy(true);
    const next = state === 'following' ? 'not_following' : 'following';
    try {
      if (state === 'following') {
        await apiClient.follows.unfollow(targetCryptoId);
      } else {
        await apiClient.follows.follow(targetCryptoId);
      }
      setState(next);
      // No PII: log only the resulting action, never the address/handle.
      log('follow toggled -> %s', next);
    } catch (err) {
      log('follow toggle error: %s', String(err));
    } finally {
      setBusy(false);
    }
  }, [busy, enabled, state, targetCryptoId]);

  return { state, busy, isSelf, enabled, toggle };
}

// ── Presentational bits ──────────────────────────────────────────────────────────

function StatusBlock({ tone, title, body }: { tone: string; title: string; body?: string }) {
  return (
    <div className="flex h-64 flex-col items-center justify-center gap-2 text-center">
      <p className={`text-base font-medium ${tone}`}>{title}</p>
      {body && <p className="max-w-md text-sm text-content-muted">{body}</p>}
    </div>
  );
}

// ── Follow stats sub-hook ────────────────────────────────────────────────────────

function useFollowStats(cryptoId: string): FollowStats | null {
  const [stats, setStats] = useState<FollowStats | null>(null);
  useEffect(() => {
    if (!cryptoId) return;
    let cancelled = false;
    void apiClient.follows
      .stats(cryptoId)
      .then(s => {
        if (!cancelled) setStats(s);
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, [cryptoId]);
  return stats;
}

// ── Main export ──────────────────────────────────────────────────────────────────

export default function ProfileViewer() {
  const { username } = useParams<{ username: string }>();
  const { t } = useT();
  const state = useProfile(username);

  let body: React.ReactNode;
  if (state.status === 'loading') {
    body = (
      <div className="flex h-64 items-center justify-center text-content-faint">
        <span className="animate-pulse text-sm">{t('agentWorld.profileViewer.loading')}</span>
      </div>
    );
  } else if (state.status === 'not_found') {
    body = (
      <StatusBlock
        tone="text-content-secondary"
        title={t('agentWorld.profileViewer.notFoundTitle')}
        body={t('agentWorld.profileViewer.notFoundBody')}
      />
    );
  } else if (state.status === 'error') {
    body = (
      <StatusBlock
        tone="text-red-600 dark:text-red-400"
        title={t('agentWorld.profileViewer.errorTitle')}
        body={state.message}
      />
    );
  } else {
    body = <ProfileCard profile={state.profile} />;
  }

  return (
    <PanelScaffold description={t('agentWorld.profileViewer.description')}>{body}</PanelScaffold>
  );
}

// ── Profile card ─────────────────────────────────────────────────────────────────

function ProfileCard({ profile }: { profile: GqlProfile }) {
  const { t } = useT();
  const myAgentId = useMyAgentId();
  const cryptoId = profile.cryptoId;
  const follow = useFollow(myAgentId, cryptoId);
  const followStats = useFollowStats(cryptoId);
  const [copied, setCopied] = useState(false);

  const primaryIdentity = pickPrimary(profile.identities ?? []);
  const primaryUsername = primaryIdentity?.username ?? null;
  const hasHandle = primaryUsername !== null;
  const usernameClean = (primaryUsername ?? profile.displayName ?? '').replace(/^@+/, '');
  const handle = hasHandle ? `@${usernameClean}` : usernameClean;
  const displayName = profile.displayName || usernameClean || '?';
  const initials = displayName.slice(0, 2).toUpperCase();
  const skills = profile.tags ?? [];
  const attestations: GqlAttestation[] = profile.attestations ?? [];
  const ownedIdentities: Identity[] = profile.identities ?? [];
  const actorType = profile.actorType ?? '';
  const isHuman = actorType.toLowerCase() === 'human';

  const copyLink = useCallback(() => {
    // HashRouter deep link to this viewer. Built from the live location so it is
    // correct regardless of host / base path.
    const { origin, pathname } = window.location;
    const link = `${origin}${pathname}#/agent-world/profiles/${encodeURIComponent(usernameClean)}`;
    void navigator.clipboard
      ?.writeText(link)
      .then(() => {
        setCopied(true);
        log('copied share link');
        window.setTimeout(() => setCopied(false), 2000);
      })
      .catch(() => {});
  }, [usernameClean]);

  return (
    <div className="rounded-lg border border-line bg-surface p-4">
      {/* Header row: identity + actions */}
      <div className="flex items-start gap-4">
        {profile.avatarUrl ? (
          <img
            src={profile.avatarUrl}
            alt={displayName}
            className="h-14 w-14 shrink-0 rounded-full object-cover"
          />
        ) : (
          <div className="bg-primary-600 flex h-14 w-14 shrink-0 items-center justify-center rounded-full text-lg font-semibold text-content-inverted">
            {initials}
          </div>
        )}
        <div className="min-w-0 flex-1">
          <h3 className="flex items-center gap-1.5 text-sm font-semibold text-content">
            <span className="truncate">{handle}</span>
            {profile.verified && (
              <span className="text-xs text-blue-500" title="Verified">
                &#10003;
              </span>
            )}
            {actorType && (
              <span
                className={`rounded-full px-1.5 py-0.5 text-[10px] font-medium ${
                  isHuman
                    ? 'bg-emerald-50 text-emerald-600 dark:bg-emerald-900/30 dark:text-emerald-300'
                    : 'bg-violet-50 text-violet-600 dark:bg-violet-900/30 dark:text-violet-300'
                }`}>
                {isHuman
                  ? t('agentWorld.profileViewer.humanBadge')
                  : t('agentWorld.profileViewer.agentBadge')}
              </span>
            )}
          </h3>
          {cryptoId && (
            <p className="mt-0.5 font-mono text-xs text-content-muted" title={cryptoId}>
              {truncateCryptoId(cryptoId)}
            </p>
          )}
          {profile.bio && (
            <p className="mt-1.5 text-xs leading-relaxed text-content-secondary">{profile.bio}</p>
          )}
        </div>

        {/* Actions */}
        <div className="flex shrink-0 flex-col items-end gap-1.5">
          {follow.isSelf ? (
            <span className="text-[11px] text-content-faint">
              {t('agentWorld.profileViewer.ownProfile')}
            </span>
          ) : follow.enabled ? (
            <Button
              variant={follow.state === 'following' ? 'secondary' : 'primary'}
              size="sm"
              disabled={follow.busy || follow.state === 'unknown'}
              onClick={() => void follow.toggle()}>
              {follow.state === 'following'
                ? t('agentWorld.profileViewer.following')
                : t('agentWorld.profileViewer.follow')}
            </Button>
          ) : null}
          <Button variant="tertiary" size="sm" onClick={copyLink} data-testid="profile-copy-link">
            {copied
              ? t('agentWorld.profileViewer.linkCopied')
              : t('agentWorld.profileViewer.copyLink')}
          </Button>
        </div>
      </div>

      {skills.length > 0 && (
        <div className="mt-4 border-t border-line pt-4">
          <h4 className="mb-2 text-xs font-medium text-content">
            {t('agentWorld.profileViewer.skills')}
          </h4>
          <div className="flex flex-wrap gap-1.5">
            {skills.map(skill => (
              <span
                key={skill}
                className="rounded-full bg-surface-subtle px-2 py-0.5 text-xs text-content-secondary">
                {skill}
              </span>
            ))}
          </div>
        </div>
      )}

      {attestations.length > 0 && (
        <div className="mt-4 border-t border-line pt-4">
          <h4 className="mb-2 text-xs font-medium text-content">
            {t('agentWorld.profileViewer.verifiedAccounts')}
          </h4>
          <div className="flex flex-wrap gap-2">
            {attestations.map(a => (
              <span
                key={a.attestationId}
                className="inline-flex items-center gap-1 rounded-full bg-green-50 px-2 py-0.5 text-xs text-green-700 dark:bg-green-900/30 dark:text-green-300">
                {a.platform}: {a.handle}
              </span>
            ))}
          </div>
        </div>
      )}

      {ownedIdentities.length > 0 && (
        <div className="mt-4 border-t border-line pt-4">
          <h4 className="mb-2 text-xs font-medium text-content">
            {t('agentWorld.profileViewer.handlesOwned')}
          </h4>
          <div className="flex flex-wrap gap-1.5">
            {ownedIdentities.map(id => (
              <span
                key={id.username}
                className="rounded-full border border-line px-2 py-0.5 text-xs text-content-secondary">
                @{id.username.replace(/^@+/, '')}
              </span>
            ))}
          </div>
        </div>
      )}

      {followStats && (
        <div className="mt-4 border-t border-line pt-4">
          <div className="flex gap-6">
            <div>
              <span className="text-sm font-semibold text-content">
                {followStats.followerCount}
              </span>
              <span className="ml-1 text-xs text-content-muted">
                {t('agentWorld.profileViewer.followers')}
              </span>
            </div>
            <div>
              <span className="text-sm font-semibold text-content">
                {followStats.followingCount}
              </span>
              <span className="ml-1 text-xs text-content-muted">
                {t('agentWorld.profileViewer.followingCount')}
              </span>
            </div>
          </div>
        </div>
      )}

      {profile.createdAt && (
        <div className="mt-4 border-t border-line pt-4">
          <span className="text-xs text-content-muted">
            {t('agentWorld.profileViewer.joined')} {formatDate(profile.createdAt)}
          </span>
        </div>
      )}
    </div>
  );
}
