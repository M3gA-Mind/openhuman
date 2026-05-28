import {
  Fit,
  Layout,
  useRive,
  useViewModel,
  useViewModelInstance,
  useViewModelInstanceBoolean,
  useViewModelInstanceColor,
  useViewModelInstanceNumber,
  useViewModelInstanceString,
} from '@rive-app/react-webgl2';
import { type FC, useEffect } from 'react';

import type { MascotFace } from './Ghosty';
import type { VisemeId } from './visemes';

export const OPENHUMAN_RIV_SRC = '/mascots/tiny_mascot.riv';

// tiny_mascot.riv has two state machines: the outer one drives pose/body/color,
// while LipSyncSM drives the mouth animation layer. Both must run simultaneously.
const OPENHUMAN_STATE_MACHINES = ['Main State Machine', 'LipSyncSM'];

// viseme is a number property in the Rive VM (0=REST, 1=A, 2=E, 3=I, 4=O, 5=U, 6=M, 7=F)
const VISEME_TO_NUM: Record<VisemeId, number> = {
  REST: 0,
  A: 1,
  E: 2,
  I: 3,
  O: 4,
  U: 5,
  M: 6,
  F: 7,
};

export interface RiveMascotProps {
  face?: MascotFace;
  size?: number | string;
  primaryColor?: number;
  secondaryColor?: number;
  viseme?: VisemeId;
  /** Path to the .riv asset. Defaults to the OpenHuman mascot. */
  src?: string;
}

const SPEAKING_FACES: ReadonlySet<MascotFace> = new Set(['speaking', 'happy']);

const FACE_TO_POSE: Record<MascotFace, string> = {
  idle: 'idle',
  normal: 'idle',
  sleep: 'sleeping',
  listening: 'idle',
  thinking: 'thinking',
  confused: 'thinking',
  speaking: 'idle',
  happy: 'idle',
  concerned: 'idle',
};

const RIVE_LAYOUT = new Layout({ fit: Fit.Contain });

export const RiveMascot: FC<RiveMascotProps> = ({
  face = 'idle',
  size = '100%',
  primaryColor,
  secondaryColor,
  viseme = 'REST',
  src = OPENHUMAN_RIV_SRC,
}) => {
  const { rive, RiveComponent } = useRive({
    src,
    stateMachines: OPENHUMAN_STATE_MACHINES,
    autoplay: true,
    layout: RIVE_LAYOUT,
  });

  const viewModel = useViewModel(rive, { useDefault: true });
  const vmInstance = useViewModelInstance(viewModel, { useDefault: true, rive });
  const { setValue: setMouthOpen } = useViewModelInstanceBoolean('mouthOpen', vmInstance);
  const { setValue: setPose } = useViewModelInstanceString('pose', vmInstance);
  const { setValue: setVisemeNum } = useViewModelInstanceNumber('viseme', vmInstance);
  const { setValue: setPrimaryColor } = useViewModelInstanceColor('primaryColor', vmInstance);
  const { setValue: setSecondaryColor } = useViewModelInstanceColor('secondaryColor', vmInstance);

  useEffect(() => {
    const speaking = SPEAKING_FACES.has(face!);
    setMouthOpen(speaking);
    setPose(FACE_TO_POSE[face!] ?? 'idle');
    // Also directly play/stop the talking animation in case VM bindings aren't wired
    if (rive) {
      if (speaking) {
        rive.play('talking9');
      } else {
        rive.stop('talking9');
      }
    }
  }, [face, rive, setMouthOpen, setPose]);

  useEffect(() => {
    // When speaking with REST (no real viseme data), default to A to ensure visible mouth movement
    const visemeNum =
      SPEAKING_FACES.has(face!) && viseme === 'REST' ? 1 : (VISEME_TO_NUM[viseme] ?? 0);
    setVisemeNum(visemeNum);
  }, [face, viseme, setVisemeNum]);

  useEffect(() => {
    if (primaryColor !== undefined) setPrimaryColor(primaryColor);
  }, [primaryColor, setPrimaryColor]);

  useEffect(() => {
    if (secondaryColor !== undefined) setSecondaryColor(secondaryColor);
  }, [secondaryColor, setSecondaryColor]);

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
