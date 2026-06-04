import { configureStore } from '@reduxjs/toolkit';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { Provider } from 'react-redux';
import { MemoryRouter } from 'react-router-dom';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { I18nProvider } from '../../../lib/i18n/I18nContext';
import type { Locale } from '../../../lib/i18n/types';
import localeReducer from '../../../store/localeSlice';
import CustomInferencePage from './CustomInferencePage';

const navigateMock = vi.fn();
const setDraftMock = vi.fn();
const clearSessionMock = vi.fn().mockResolvedValue(undefined);

vi.mock('react-router-dom', async importOriginal => {
  const actual = await importOriginal<typeof import('react-router-dom')>();
  return { ...actual, useNavigate: () => navigateMock };
});

vi.mock('../../../components/settings/panels/AIPanel', () => ({
  default: () => <div data-testid="ai-panel">AI Panel</div>,
}));

vi.mock('../../../providers/CoreStateProvider', () => ({
  useCoreState: () => ({
    snapshot: { sessionToken: 'header.payload.local' },
    clearSession: clearSessionMock,
  }),
}));

vi.mock('../OnboardingContext', () => ({
  useOnboardingContext: () => ({
    draft: { connectedSources: [] },
    setDraft: setDraftMock,
    completeAndExit: vi.fn(),
  }),
}));

function renderPage() {
  const store = configureStore({
    reducer: { locale: localeReducer },
    preloadedState: { locale: { current: 'en' as Locale } },
  });

  return render(
    <Provider store={store}>
      <MemoryRouter>
        <I18nProvider>
          <CustomInferencePage />
        </I18nProvider>
      </MemoryRouter>
    </Provider>
  );
}

describe('CustomInferencePage', () => {
  beforeEach(() => {
    navigateMock.mockReset();
    setDraftMock.mockReset();
    clearSessionMock.mockClear();
  });

  it('forces configure mode and hides the default/configure chooser for local sessions', () => {
    renderPage();

    expect(screen.getByTestId('ai-panel')).toBeInTheDocument();
    expect(
      screen.queryByTestId('onboarding-custom-inference-step-default')
    ).not.toBeInTheDocument();
    expect(
      screen.queryByTestId('onboarding-custom-inference-step-configure')
    ).not.toBeInTheDocument();
  });

  it('clears the session and navigates back to the welcome page from the first step', async () => {
    renderPage();

    fireEvent.click(screen.getByRole('button', { name: 'Back' }));

    await waitFor(() => expect(navigateMock).toHaveBeenCalledWith('/'));
    expect(clearSessionMock).toHaveBeenCalledTimes(1);
  });
});
