export type Pile = Partial<
  Record<'food' | 'wood' | 'ore' | 'ideas' | 'gold' | 'mood_tokens' | 'culture_tokens', number>
>;
export type Resource = keyof Pile;
export type Move = string | Record<string, unknown>;
export type Terrain =
  'Forest' | 'Fertile' | 'Mountain' | 'Barren' | 'Water' | 'Unexplored' | { Exhausted: string };
export interface City {
  position: string;
  mood_state: string;
  city_pieces?: Record<string, unknown>;
  activations?: number;
}
export interface Player {
  id: number;
  name?: string;
  civilization: string;
  resources?: Pile;
  resource_limit?: Pile;
  cities?: City[];
  units?: { position: string; unit_type: string; id: number }[];
  advances?: string[];
  action_cards?: number[];
  objective_cards?: number[];
}
export interface Game {
  state: unknown;
  players: Player[];
  map: { tiles: [string, Terrain][] };
  current_player_index: number;
  actions_left: number;
  age: number;
  round: number;
  log?: {
    age: number;
    rounds: { round: number; turns: { turn_type: unknown; actions?: LoggedAction[] }[] }[];
  }[];
  log_index: number;
}
export interface LoggedAction {
  action?: Move;
  log?: string[];
  items?: {
    player: number;
    Structure?: { structure: unknown; balance: string; position: string };
    HandCard?: { to: unknown };
  }[];
  combat_stats?: unknown;
}
export interface Choice {
  position: string;
  pile: Pile;
}
export interface Selection extends Choice {
  times: number;
}
export interface CityView {
  position: string;
  size: number;
  capacity: number;
  maxPerTile: number;
  maxRange2: number;
  mood: string;
  activations: number;
  canActivate: boolean;
  activationMood: string;
  activationCapacity: number;
  reason: string | null;
  choices: Choice[];
}
export interface AdvanceView {
  id: string;
  name: string;
  description: string;
  owned: boolean;
  reason: string | null;
  payment: Pile;
  action: Move | null;
  group: string;
  order: number;
  required: string | null;
  bonus: Pile | null;
  unlocks: string | null;
}
export interface PlayerView {
  index: number;
  name: string;
  civilization: string;
  score: number;
  scoreParts: { name: string; points: number }[];
  cities: { position: string; size: number; capacity: number; mood: string; activations: number }[];
}
export interface WonderCard {
  id: string;
  name: string;
  description: string;
  cost: Pile;
  requiredAdvance: string;
  requiredAdvanceOwned: boolean;
  builtPoints: number;
  ownedPoints: number;
}
export type CardDraw =
  { kind: 'wonder'; card: WonderCard } | { kind: 'objective'; card: View['objectiveCards'][number] };
export interface View {
  activePlayer: number;
  canPlay: boolean;
  supportedPhase: boolean;
  players: PlayerView[];
  cities: CityView[];
  cityActions: CityActions[];
  settlers: SettlerView[];
  stopMovement: Move | null;
  choiceDecision?: { name: string; choices: { name: string; pile?: Pile; action: Move }[] } | null;
  advances: AdvanceView[];
  objectiveCards: {
    id: number;
    objectives: { name: string; description: string; timing: 'Instant' | 'Status phase' }[];
  }[];
  wonderCards: WonderCard[];
  objectiveDecision: {
    name: string;
    description: string;
    points: number;
    cards: { id: number; name: string; action: Move }[];
    skip: Move | null;
  } | null;
  canUndo: boolean;
  canEndTurn: boolean;
}
export interface CollectionPreview {
  action: Move;
  total: Pile;
  waste: Pile;
  after: Pile;
  moodWillDecrease: boolean;
}
export interface Bridge {
  webView: (state: string, seat?: number) => string;
  webCollectPreview: (state: string, seat: number, city: string, choices: string) => string;
  webRecruitPreview: (state: string, seat: number, city: string, units: string) => string;
}
export type UnitKind = 'Settler' | 'Infantry' | 'Cavalry' | 'Elephant' | 'Ship';
export type RecruitSelection = Partial<
  Record<'settlers' | 'infantry' | 'cavalry' | 'elephants' | 'ships', number>
>;
export interface ActionOffer {
  action: Move | null;
  payment: Pile;
  reason: string | null;
}
export interface CityActions {
  position: string;
  buildings: {
    name: string;
    owned: boolean;
    required: string;
    payment: Pile;
    reason: string | null;
    moodWillDecrease: boolean;
    choices: { position: string | null; action: Move }[];
  }[];
  recruits: { type: UnitKind; payment: Pile; reason: string | null; available: number }[];
  happiness: (ActionOffer & { steps: number; mood: string })[];
}
export interface SettlerView {
  id: number;
  position: string;
  foundReason: string | null;
  foundAction: Move | null;
  destinations: { position: string; terrain: Terrain; payment: Pile; action: Move }[];
}
export interface RecruitPreview {
  payment: Pile;
  action: Move;
  moodWillDecrease: boolean;
}
export interface JournalEntry {
  id: string;
  age: number;
  round: number;
  text: string;
  kind:
    | 'setup'
    | 'collect'
    | 'research'
    | 'objective'
    | 'end-turn'
    | 'move'
    | 'build'
    | 'recruit'
    | 'combat'
    | 'card'
    | 'event';
  setup?: { player: string; civilization: string; position?: string };
}
export interface Session {
  game: Game | null;
  view: View | null;
  seat?: number;
  city: string | null;
  focus: string | null;
  mode: 'overview' | 'collect' | 'research' | 'city' | 'settlers';
  selectedSettler: number | null;
  destination: string | null;
  recruits: RecruitSelection;
  recruitPreview: RecruitPreview | null;
  selection: Selection[];
  preview: CollectionPreview | null;
  error: string;
  pending: boolean;
  tab: 'journal' | 'chat';
  activityOpen: boolean;
  help: boolean;
  objectivesOpen: boolean;
  wondersOpen: boolean;
  scorePlayer: number | null;
  cardDraws: CardDraw[];
  dark: boolean;
  unread: number;
  avatars: string[];
  reducedMotion: boolean;
  colorBlind: boolean;
  sound: boolean;
  locale: string;
  selectedAdvance: string | null;
  toast: string;
  topDown: boolean;
}
export const resourceNames: Record<Resource, string> = {
  food: 'Food',
  wood: 'Wood',
  ore: 'Ore',
  ideas: 'Ideas',
  gold: 'Gold',
  mood_tokens: 'Mood',
  culture_tokens: 'Culture',
};
export const resources = Object.keys(resourceNames) as Resource[];
export const playerColors = ['#5086af', '#b96c4c', '#6d9173', '#ae92b6'];
const accessiblePlayerColors = ['#0072b2', '#d55e00', '#009e73', '#cc79a7', '#746800', '#333333'];
export const playerSymbol = (index: number) => ['●', '▲', '■', '◆', '✚', '✕'][index] ?? String(index + 1);
export const playerColor = (index: number, colorBlind: boolean) =>
  (colorBlind ? accessiblePlayerColors : playerColors)[index] ?? '#bfa986';
