/**
 * Coverage guard for the new `<div role="button">` row wrapper + onKeyDown
 * handler in `Notifications.tsx` (Bug A of #2279). The wrapper replaces a
 * `<button>` because rows now contain `NotificationLinkPill` (also a
 * `<button>`), and nested interactive elements are invalid HTML.
 *
 * Scoped to behavioural assertions: keyboard activation dispatches
 * `markRead` and navigates. The body-rendering path (`<openhuman-link>`
 * parsing, pill, XSS guards) is covered once by `NotificationCard.test.tsx`
 * via the shared `NotificationBody` component.
 */
import { configureStore } from '@reduxjs/toolkit';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { Provider } from 'react-redux';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';

import notificationsReducer, {
  type NotificationCategory,
  type NotificationItem,
} from '../../store/notificationSlice';
import Notifications from '../Notifications';

const { navigate } = vi.hoisted(() => ({ navigate: vi.fn() }));

vi.mock('react-router-dom', async () => {
  const actual = await vi.importActual<typeof import('react-router-dom')>('react-router-dom');
  return { ...actual, useNavigate: () => navigate };
});

vi.mock('../../lib/notificationRouter', () => ({ resolveSystemRoute: () => '/some-route' }));

vi.mock('../../components/notifications/NotificationCenter', () => ({
  default: () => <div data-testid="notification-center-stub" />,
}));

vi.mock('../../lib/i18n/I18nContext', () => ({ useT: () => ({ t: (k: string) => k }) }));

function makeItem(
  id: string,
  body: string,
  overrides: Partial<NotificationItem> = {}
): NotificationItem {
  return {
    id,
    title: 'A title',
    body,
    category: 'system' as NotificationCategory,
    timestamp: Date.now(),
    read: false,
    ...overrides,
  };
}

function renderPage(items: NotificationItem[]) {
  const store = configureStore({
    reducer: { notifications: notificationsReducer },
    preloadedState: {
      notifications: {
        items,
        preferences: {
          messages: true,
          agents: true,
          skills: true,
          system: true,
          meetings: true,
          reminders: true,
          important: true,
        },
        integrationItems: [],
        integrationUnreadCount: 0,
        integrationLoading: false,
        integrationError: null,
      },
    },
  });
  return {
    store,
    ...render(
      <Provider store={store}>
        <MemoryRouter>
          <Notifications />
        </MemoryRouter>
      </Provider>
    ),
  };
}

describe('Notifications page row wrapper', () => {
  it('renders <openhuman-link> body via the shared NotificationBody', () => {
    renderPage([
      makeItem('n-1', '<openhuman-link path="community/discord">Discord</openhuman-link>'),
    ]);

    const bodyEl = screen.getByTestId('notification-item-body');
    // Pill rendered (so the body uses the shared component), raw tag absent.
    expect(within(bodyEl).getByRole('button', { name: /Discord/i })).toBeInTheDocument();
    expect(bodyEl.textContent ?? '').not.toContain('<openhuman-link');
  });

  it('activates a row via Enter and Space keys', () => {
    const { store } = renderPage([makeItem('n-1', 'plain body')]);

    // The row wrapper is the only role=button inside the row (plain body, no pill).
    const row = screen.getByTestId('notification-item');
    const wrapper = within(row).getByRole('button');

    fireEvent.keyDown(wrapper, { key: 'Enter' });
    expect(navigate).toHaveBeenCalledWith('/some-route');
    expect(store.getState().notifications.items[0].read).toBe(true);

    navigate.mockClear();
    fireEvent.keyDown(wrapper, { key: ' ' });
    expect(navigate).toHaveBeenCalledWith('/some-route');
  });

  it('does NOT activate on other keys (Tab, letters)', () => {
    renderPage([makeItem('n-1', 'plain body')]);
    const row = screen.getByTestId('notification-item');
    const wrapper = within(row).getByRole('button');

    fireEvent.keyDown(wrapper, { key: 'Tab' });
    fireEvent.keyDown(wrapper, { key: 'a' });
    expect(navigate).not.toHaveBeenCalled();
  });

  // Bubbling guard: Enter/Space on a focused inner pill must NOT also activate
  // the row. CodeRabbit catch on PR #2339.
  it('does NOT activate a row when keydown bubbles from the inner pill', () => {
    const { store } = renderPage([
      makeItem('n-1', '<openhuman-link path="community/discord">Discord</openhuman-link>'),
    ]);

    const bodyEl = screen.getByTestId('notification-item-body');
    const pill = within(bodyEl).getByRole('button', { name: /Discord/i });
    fireEvent.keyDown(pill, { key: 'Enter' });
    fireEvent.keyDown(pill, { key: ' ' });
    expect(navigate).not.toHaveBeenCalled();
    expect(store.getState().notifications.items[0].read).toBe(false);
  });
});

// #3715: category-chip filter row above the system-events list.
describe('Notifications page category filter', () => {
  it('hides the chip row entirely when there are no notifications', () => {
    renderPage([]);
    expect(screen.queryByTestId('notification-category-filter')).not.toBeInTheDocument();
  });

  it('renders an "All" chip plus one chip per category present (no dead chips)', () => {
    renderPage([
      makeItem('n-1', 'a', { category: 'messages' }),
      makeItem('n-2', 'b', { category: 'agents' }),
      makeItem('n-3', 'c', { category: 'messages' }),
    ]);

    const filter = screen.getByTestId('notification-category-filter');
    const chips = within(filter).getAllByRole('tab');
    // All + messages + agents — and NOT the absent categories.
    expect(chips.map(c => c.textContent)).toEqual([
      'alerts.filterAll',
      'notifications.category.messages',
      'notifications.category.agents',
    ]);
    expect(
      within(filter).queryByText('notifications.category.system')
    ).not.toBeInTheDocument();
  });

  it('filters the list to the selected category and back to all', () => {
    renderPage([
      makeItem('n-1', 'a', { category: 'messages' }),
      makeItem('n-2', 'b', { category: 'agents' }),
    ]);

    // Default: All — both rows visible.
    expect(screen.getAllByTestId('notification-item')).toHaveLength(2);

    const filter = screen.getByTestId('notification-category-filter');
    fireEvent.click(within(filter).getByText('notifications.category.messages'));

    // Narrowed to the single messages row.
    const rows = screen.getAllByTestId('notification-item');
    expect(rows).toHaveLength(1);
    expect(rows[0].textContent).toContain('notifications.category.messages');

    // Clicking "All" restores the full feed.
    fireEvent.click(within(filter).getByText('alerts.filterAll'));
    expect(screen.getAllByTestId('notification-item')).toHaveLength(2);
  });
});
