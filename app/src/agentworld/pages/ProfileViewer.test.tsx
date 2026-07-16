/**
 * Tests for ProfileViewer — the Agent World public profile viewer (#4931).
 *
 * The viewer renders an ARBITRARY handle's profile via `graphql.profile`, with a
 * follow/unfollow button (`follows.follow`/`unfollow`, follow-state from
 * `follows.following`) and a copy-link affordance. apiClient + wallet are mocked;
 * all handles/ids are generic placeholders. These behaviours do not exist before
 * this change (the route + component are new), so the file is the regression.
 */
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { beforeEach, describe, expect, test, vi } from 'vitest';

import { type GqlProfile } from '../../lib/agentworld/invokeApiClient';
import { fetchWalletStatus } from '../../services/walletApi';
import { apiClient } from '../AgentWorldShell';
import ProfileViewer from './ProfileViewer';

vi.mock('../AgentWorldShell', () => ({
  apiClient: {
    graphql: { profile: vi.fn() },
    follows: { following: vi.fn(), follow: vi.fn(), unfollow: vi.fn(), stats: vi.fn() },
  },
}));
vi.mock('../../services/walletApi', () => ({ fetchWalletStatus: vi.fn() }));

const graphqlProfile = vi.mocked(apiClient.graphql.profile);
const followsFollowing = vi.mocked(apiClient.follows.following);
const followsFollow = vi.mocked(apiClient.follows.follow);
const followsUnfollow = vi.mocked(apiClient.follows.unfollow);
const followsStats = vi.mocked(apiClient.follows.stats);
const walletStatus = vi.mocked(fetchWalletStatus);

const PROFILE_ADDR = 'ProfiLeSoLanaAddr00000000001';
const VIEWER_ADDR = 'ViewerSoLanaAddr00000000002';

function makeProfile(overrides: Partial<GqlProfile> = {}): GqlProfile {
  return {
    cryptoId: PROFILE_ADDR,
    actorType: 'agent',
    displayName: 'Alice Agent',
    bio: 'An autonomous test agent.',
    private: false,
    createdAt: '2026-01-02T00:00:00Z',
    updatedAt: '2026-01-02T00:00:00Z',
    verified: true,
    attestations: [],
    agentCard: null,
    identities: [
      {
        username: 'alice',
        cryptoId: PROFILE_ADDR,
        publicKey: 'pk',
        registeredAt: '2026-01-02T00:00:00Z',
        expiresAt: '2027-01-02T00:00:00Z',
        status: 'active',
        updatedAt: '2026-01-02T00:00:00Z',
        primary: true,
      },
    ],
    ...overrides,
  };
}

function walletWith(address: string | null) {
  return { accounts: address ? [{ chain: 'solana', address }] : [] } as unknown as Awaited<
    ReturnType<typeof fetchWalletStatus>
  >;
}

function renderViewer(username = 'alice') {
  return render(
    <MemoryRouter initialEntries={[`/agent-world/profiles/${username}`]}>
      <Routes>
        <Route path="/agent-world/profiles/:username" element={<ProfileViewer />} />
      </Routes>
    </MemoryRouter>
  );
}

beforeEach(() => {
  vi.clearAllMocks();
  graphqlProfile.mockResolvedValue(makeProfile());
  followsFollowing.mockResolvedValue({ following: [] });
  followsFollow.mockResolvedValue({ follower: VIEWER_ADDR, followee: PROFILE_ADDR, createdAt: '' });
  followsUnfollow.mockResolvedValue(undefined);
  followsStats.mockResolvedValue({ agentId: PROFILE_ADDR, followerCount: 3, followingCount: 5 });
  walletStatus.mockResolvedValue(walletWith(VIEWER_ADDR));
  Object.defineProperty(navigator, 'clipboard', {
    value: { writeText: vi.fn().mockResolvedValue(undefined) },
    configurable: true,
  });
});

describe('ProfileViewer', () => {
  test('renders an arbitrary handle profile (not the wallet owner)', async () => {
    renderViewer('alice');
    // Looked up by the route param, not the wallet.
    await waitFor(() => expect(graphqlProfile).toHaveBeenCalledWith('alice'));
    expect(await screen.findByText('@alice')).toBeInTheDocument();
    expect(screen.getByText('An autonomous test agent.')).toBeInTheDocument();
    // Follower stats render.
    expect(screen.getByText('3')).toBeInTheDocument();
  });

  test('shows a not-found state when the profile does not exist', async () => {
    graphqlProfile.mockResolvedValue(null);
    renderViewer('ghost');
    expect(await screen.findByText(/profile not found/i)).toBeInTheDocument();
  });

  test('follow button follows then unfollows another user', async () => {
    const user = userEvent.setup();
    renderViewer('alice');

    // Button appears once the wallet resolves and follow-state loads.
    const followBtn = await screen.findByRole('button', { name: 'Follow' });
    await waitFor(() => expect(followBtn).toBeEnabled());

    await user.click(followBtn);
    expect(followsFollow).toHaveBeenCalledWith(PROFILE_ADDR);
    const followingBtn = await screen.findByRole('button', { name: 'Following' });

    await user.click(followingBtn);
    expect(followsUnfollow).toHaveBeenCalledWith(PROFILE_ADDR);
    expect(await screen.findByRole('button', { name: 'Follow' })).toBeInTheDocument();
  });

  test('pre-selects the following state from the viewer follow graph', async () => {
    followsFollowing.mockResolvedValue({
      following: [{ follower: VIEWER_ADDR, followee: PROFILE_ADDR, createdAt: '' }],
    });
    renderViewer('alice');
    // Already-following → button shows Following without a click.
    expect(await screen.findByRole('button', { name: 'Following' })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Follow' })).not.toBeInTheDocument();
  });

  test('hides the follow button and marks self when viewing own profile', async () => {
    walletStatus.mockResolvedValue(walletWith(PROFILE_ADDR));
    renderViewer('alice');
    expect(await screen.findByText(/this is your profile/i)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Follow' })).not.toBeInTheDocument();
    // Follow-state must never be fetched for one's own profile.
    expect(followsFollowing).not.toHaveBeenCalled();
  });

  test('copy-link affordance copies the shareable deep link', async () => {
    const user = userEvent.setup();
    renderViewer('alice');
    const copyBtn = await screen.findByTestId('profile-copy-link');
    await user.click(copyBtn);
    await waitFor(() =>
      expect(
        navigator.clipboard.writeText as unknown as ReturnType<typeof vi.fn>
      ).toHaveBeenCalled()
    );
    const copiedArg = (navigator.clipboard.writeText as unknown as ReturnType<typeof vi.fn>).mock
      .calls[0][0] as string;
    expect(copiedArg).toContain('#/agent-world/profiles/alice');
  });
});
