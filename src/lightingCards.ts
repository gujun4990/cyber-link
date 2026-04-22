export type LightingKind = 'mainLight' | 'ambientLight' | 'doorSignLight';

export interface LightingCard {
  kind: LightingKind;
  label: string;
  active: boolean;
  subLabel: string;
}

export interface LightingState {
  ambientLightAvailable: boolean;
  mainLightAvailable: boolean;
  doorSignLightAvailable: boolean;
  ambientLightOn: boolean;
  mainLightOn: boolean;
  doorSignLightOn: boolean;
}

const lightingCardDefinitions: Array<{
  kind: LightingKind;
  label: string;
  availableKey: keyof Pick<
    LightingState,
    'mainLightAvailable' | 'ambientLightAvailable' | 'doorSignLightAvailable'
  >;
  activeKey: keyof Pick<LightingState, 'mainLightOn' | 'ambientLightOn' | 'doorSignLightOn'>;
}> = [
  {
    kind: 'mainLight',
    label: '主照明系统',
    availableKey: 'mainLightAvailable',
    activeKey: 'mainLightOn',
  },
  {
    kind: 'ambientLight',
    label: '氛围灯',
    availableKey: 'ambientLightAvailable',
    activeKey: 'ambientLightOn',
  },
  {
    kind: 'doorSignLight',
    label: '门牌灯',
    availableKey: 'doorSignLightAvailable',
    activeKey: 'doorSignLightOn',
  },
];

export function buildLightingCards(state: LightingState): LightingCard[] {
  return lightingCardDefinitions
    .filter((definition) => state[definition.availableKey])
    .map((definition) => ({
      kind: definition.kind,
      label: definition.label,
      active: state[definition.activeKey],
      subLabel: state[definition.activeKey] ? '已开启' : '已关闭',
    }));
}
