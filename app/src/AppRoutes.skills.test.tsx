/**
 * AppRoutes — desktop skills sub-routes.
 *
 * Phase 2 of the /skills IA restructure: verify the routing change
 * exposed three distinct paths
 *   /skills      → SkillsDashboard (new landing)
 *   /skills/run  → Skills (existing runner-host page)
 *   /skills/new  → SkillNew (new authoring page)
 * with /skills/new and /skills/run taking precedence over /skills
 * so the prefix match doesn't shadow them.
 *
 * Stubs every routed component so we don't drag the full Redux +
 * provider tree along — we're testing the router wiring, not the
 * pages themselves.
 */
import { render, screen } from '@testing-library/react';
import { MemoryRouter } from 'react-router-dom';
import { afterEach, describe, expect, it, vi } from 'vitest';

vi.mock('./lib/platform', () => ({ getIsMobile: () => false }));
vi.mock('./components/ProtectedRoute', () => ({
  default: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));
vi.mock('./components/PublicRoute', () => ({
  default: ({ children }: { children: React.ReactNode }) => <>{children}</>,
}));
vi.mock('./components/DefaultRedirect', () => ({
  default: () => <div data-testid="page-default">default</div>,
}));

vi.mock('./pages/Skills', () => ({
  default: () => <div data-testid="page-skills-runner">runner</div>,
}));
vi.mock('./pages/SkillsDashboard', () => ({
  default: () => <div data-testid="page-skills-dashboard">dashboard</div>,
}));
vi.mock('./pages/SkillNew', () => ({
  default: () => <div data-testid="page-skills-new">new</div>,
}));

// Stub every other route so the Routes tree mounts without pulling
// real pages (which import heavy provider tree / RTK slices).
vi.mock('./pages/Welcome', () => ({ default: () => <div>welcome</div> }));
vi.mock('./pages/WebCallbackPage', () => ({ default: () => <div>callback</div> }));
vi.mock('./pages/onboarding/Onboarding', () => ({ default: () => <div>onboarding</div> }));
vi.mock('./pages/Home', () => ({ default: () => <div>home</div> }));
vi.mock('./features/human/HumanPage', () => ({ default: () => <div>human</div> }));
vi.mock('./pages/Intelligence', () => ({ default: () => <div>intelligence</div> }));
vi.mock('./pages/Accounts', () => ({ default: () => <div>chat</div> }));
vi.mock('./pages/Channels', () => ({ default: () => <div>channels</div> }));
vi.mock('./pages/Invites', () => ({ default: () => <div>invites</div> }));
vi.mock('./pages/Notifications', () => ({ default: () => <div>notifications</div> }));
vi.mock('./pages/Rewards', () => ({ default: () => <div>rewards</div> }));
vi.mock('./pages/Settings', () => ({ default: () => <div>settings</div> }));
vi.mock('./AppRoutesIOS', () => ({ default: () => <div>ios</div> }));

const AppRoutes = (await import('./AppRoutes')).default;

const renderAt = (path: string) =>
  render(
    <MemoryRouter initialEntries={[path]}>
      <AppRoutes />
    </MemoryRouter>
  );

describe('AppRoutes — /skills IA restructure', () => {
  afterEach(() => vi.clearAllMocks());

  it('/skills renders the new dashboard, not the runner', () => {
    renderAt('/skills');
    expect(screen.getByTestId('page-skills-dashboard')).toBeInTheDocument();
    expect(screen.queryByTestId('page-skills-runner')).not.toBeInTheDocument();
  });

  it('/skills/run renders the existing runner-host page', () => {
    renderAt('/skills/run');
    expect(screen.getByTestId('page-skills-runner')).toBeInTheDocument();
    expect(screen.queryByTestId('page-skills-dashboard')).not.toBeInTheDocument();
  });

  it('/skills/new renders the new authoring page (prefix match does not shadow)', () => {
    renderAt('/skills/new');
    expect(screen.getByTestId('page-skills-new')).toBeInTheDocument();
    expect(screen.queryByTestId('page-skills-dashboard')).not.toBeInTheDocument();
    expect(screen.queryByTestId('page-skills-runner')).not.toBeInTheDocument();
  });
});
