import {
  Fit,
  Layout,
  useRive,
  useViewModel,
  useViewModelInstance,
  useViewModelInstanceNumber,
} from '@rive-app/react-webgl2';
import { type FC, useEffect } from 'react';

import type { MascotFace } from './Ghosty';
import type { VisemeId } from './visemes';

export const TOSHI_RIV_SRC = '/mascots/toshi_mascot.riv';

// toshi_mascot.riv: MasterSM drives body (auto) + mouth (mouthState) concurrently.
// mouthState is a number property in the VM; LipSyncSM is a separate mouth-only
// alternative — only run MasterSM per the designer guide.
const TOSHI_STATE_MACHINE = 'MasterSM';

// Toshi has 6 mouth states (0–5). M/F have no dedicated shape — map to closest.
const VISEME_TO_MOUTH_STATE: Record<VisemeId, number> = {
  REST: 0, // neutral closed (mouth_happy)
  A: 1, // wide open — "ah"
  E: 2, // wide flat — "eh/ee"
  I: 3, // wide smile — "ih"
  O: 4, // round open — "oh"
  U: 5, // small round — "oo"
  M: 0, // bilabial closed → neutral
  F: 2, // labiodental slight opening → approximate with E
};

export interface ToshiMascotProps {
  face?: MascotFace;
  size?: number | string;
  viseme?: VisemeId;
}

const SPEAKING_FACES: ReadonlySet<MascotFace> = new Set(['speaking', 'happy']);

const TOSHI_LAYOUT = new Layout({ fit: Fit.Contain });

export const ToshiMascot: FC<ToshiMascotProps> = ({
  face = 'idle',
  size = '100%',
  viseme = 'REST',
}) => {
  const { rive, RiveComponent } = useRive({
    src: TOSHI_RIV_SRC,
    stateMachines: TOSHI_STATE_MACHINE,
    autoplay: true,
    layout: TOSHI_LAYOUT,
  });

  const viewModel = useViewModel(rive, { useDefault: true });
  const vmInstance = useViewModelInstance(viewModel, { useDefault: true, rive });
  const { setValue: setMouthState } = useViewModelInstanceNumber('mouthState', vmInstance);

  useEffect(() => {
    // When speaking with REST (no TTS viseme data yet), default to A so mouth opens
    const state =
      SPEAKING_FACES.has(face!) && viseme === 'REST' ? 1 : (VISEME_TO_MOUTH_STATE[viseme] ?? 0);
    setMouthState(state);
  }, [face, viseme, setMouthState]);

  return (
    <div
      style={{
        width: typeof size === 'number' ? `${size}px` : size,
        height: typeof size === 'number' ? `${size}px` : size,
      }}
      data-face={face}>
      <RiveComponent />
    </div>
  );
};
