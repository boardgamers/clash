export type Pile = Partial<
  Record<'food' | 'wood' | 'ore' | 'ideas' | 'gold' | 'mood_tokens' | 'culture_tokens' | 'captives', number>
>;
export type Resource = keyof Pile;
export type Move = string | Record<string, unknown>;
export type Terrain =
  'Forest' | 'Fertile' | 'Mountain' | 'Barren' | 'Water' | 'Unexplored' | { Exhausted: string };
export interface City {
  position: string;
  mood_state: string;
  city_pieces?: Partial<
    Record<'academy' | 'market' | 'obelisk' | 'observatory' | 'fortress' | 'port' | 'temple', number>
  > & { wonders?: string[] };
  activations?: number;
}
export interface Player {
  id: number;
  name?: string;
  civilization: string;
  resources?: Pile;
  resource_limit?: Pile;
  cities?: City[];
  units?: {
    position: string;
    pirate?: boolean;
    carrier_id?: number;
    unit_type: string | { Leader: string };
    id: number;
    carried_units?: { unit_type: string | { Leader: string }; id: number }[];
  }[];
  advances?: string[];
  special_advances?: string[];
  action_cards?: number[];
  objective_cards?: number[];
}
export interface Game {
  state: unknown;
  events?: { event_type: string | Record<string, unknown> }[];
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
    origin?: Record<string, unknown>;
    Action?: { balance: string };
    Structure?: { structure: unknown; balance: string; position: string };
    HandCard?: { to: unknown };
    Advance?: { advance: string; balance: string; take_incident_token?: boolean };
    Units?: { units: Record<string, number | string>; balance: string };
    Resources?: { resources: Pile; balance: string };
    MoodChange?: { city: string; mood: string };
  }[];
  combat_stats?: unknown;
}
export interface Choice {
  bonuses?: { source: string; pile: Pile; limit: number; condition?: 'exactlyOneFood' }[];
  position: string;
  pile: Pile;
}
export interface Selection extends Choice {
  times: number;
}
export interface CityView {
  ballcourts?: boolean;
  draftCard?: boolean;
  attackPirates?: boolean;
  shogunateDraft?: boolean;
  piratePort?: boolean;
  capital?: boolean;
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
export interface PublicAdvance {
  id: string;
  name: string;
  description: string;
  group: string;
  order: number;
  borrowed?: boolean;
  borrowedSource?: string;
}
export interface AdvanceView extends PublicAdvance {
  owned: boolean;
  reason: string | null;
  payment: Pile;
  costAmount: number;
  costResources: Resource[];
  costGroups?: { amount: number; resources: Resource[] }[];
  payments: { payment: Pile; action: Move | null }[];
  action: Move | null;
  required: string | null;
  bonus: Pile | null;
  bonusEffects?: { source: string; pile: Pile }[];
  unlocks: string | null;
}
export interface CivilizationAdvance extends PublicAdvance {
  owned: boolean;
  active: boolean;
  requirement: string;
  prerequisites: { id: string; name: string }[];
}
export interface PlayerView {
  leaders?: {
    id: string;
    unit: number;
    position: string;
    name: string;
    abilities: { name: string; description: string }[];
  }[];
  capital?: string;
  index: number;
  name: string;
  civilization: string;
  score: number;
  eventTokens: number;
  advances: PublicAdvance[];
  civilizationAdvances: CivilizationAdvance[];
  scoreParts: { name: string; points: number }[];
  cities: {
    position: string;
    size: number;
    capacity: number;
    mood: string;
    activations: number;
    protection?: number;
    independentPort?: boolean;
    influenceMarker?: number;
  }[];
}
export interface WonderCard {
  action?: Move | null;
  reason?: string | null;
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
  builtWonders?: Pick<WonderCard, 'id' | 'name' | 'description' | 'builtPoints' | 'ownedPoints'>[];
  eventCatalog?: EventInfo[];
  pendingEvent?: { id: number; player: number } | null;
  collectActions?: ActionVariant[];
  happinessActions?: ActionVariant[];
  decision?: Decision | null;
  civilizations?: {
    name: string;
    action: Move;
    advances: { name: string; description: string; requirement: string }[];
    leaders: { name: string; abilities: { name: string; description: string }[] }[];
  }[];
  actionCards?: ActionCard[];
  specialActions?: {
    name: string;
    description: string;
    position: string | null;
    action: Move;
    cost?: Pile;
    free?: boolean;
    activatesCity?: string | null;
  }[];
  influence?: {
    name: string;
    position: string;
    origin: string;
    origins?: { position: string; settlers: boolean; reroll: boolean; payment: Pile; action: Move }[];
    variant: string;
    payment: Pile;
    action: Move;
  }[];
  units?: UnitView[];
  movementLeft?: number;
  nomadCities?: string[];
  seaRoutes?: string[][];
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
    objectives: {
      name: string;
      description: string;
      timing: 'Instant' | 'Status phase';
      progress?: { label: string; current: number; target: number }[];
      conditionMet?: boolean;
      scoringAge?: number;
    }[];
  }[];
  wonderCards: WonderCard[];
  explorationDecision?: {
    start: string;
    destination: string | null;
    choices: { rotation: number; tiles: [string, Terrain][]; action: Move }[];
  } | null;
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
  effects?: { source: string; description: string }[];
  after: Pile;
  moodWillDecrease: boolean;
}
export interface Bridge {
  webQuery: (state: string, seat: number, query: string) => string;
  webView: (state: string, seat?: number) => string;
  webCollectPreview: (state: string, seat: number, city: string, choices: string) => string;
  webRecruitPreview: (state: string, seat: number, city: string, units: string) => string;
}
export type UnitKind = 'Settler' | 'Infantry' | 'Cavalry' | 'Elephant' | 'Ship';
export type RecruitSelection = Partial<
  Record<'settlers' | 'infantry' | 'cavalry' | 'elephants' | 'ships', number>
> & { leader?: string | null };
export interface ActionOffer {
  action: Move | null;
  payment: Pile;
  reason: string | null;
}
export interface CityActions {
  leaders?: {
    id: string;
    name: string;
    description: string;
    reason?: string | null;
    payment?: Pile;
    abilities: { name: string; description: string }[];
  }[];
  position: string;
  buildings: {
    name: string;
    owned: boolean;
    required: string;
    payment: Pile;
    payments?: Pile[] | null;
    reason: string | null;
    free?: boolean;
    activateCity?: boolean;
    source?: string | null;
    moodWillDecrease: boolean;
    choices: { position: string | null; action: Move }[];
  }[];
  recruits: {
    type: UnitKind;
    payment: Pile;
    basePayment?: Pile;
    costOptions?: string[];
    reason: string | null;
    available: number;
    limit?: number;
  }[];
  happiness: (ActionOffer & { steps: number; mood: string; lawgiver?: boolean })[];
}
export interface SettlerView {
  id: number;
  position: string;
  foundReason: string | null;
  foundFree?: boolean;
  foundAction: Move | null;
  destinations: { position: string; terrain: Terrain; payment: Pile; action: Move }[];
}
export interface RecruitPreview {
  payments?: Pile[];
  payment: Pile;
  basePayment?: Pile;
  costOptions?: string[];
  action: Move;
  moodWillDecrease: boolean;
}
export interface JournalToken {
  icon:
    | Resource
    | 'action'
    | 'wonder'
    | 'objective'
    | 'card'
    | 'research'
    | 'city'
    | 'unit'
    | 'happy'
    | 'neutral'
    | 'angry'
    | 'event';
  value?: string;
  label: string;
  description: string;
  compact?: boolean;
  tone?: 'gain' | 'loss';
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
  player?: number;
  civilization?: string;
  title: string;
  tokens: JournalToken[];
  notes: string[];
  combat?: import('./combat-journal').CombatRound;
  collection?: {
    city?: { size: number; mood: string; structures: string[] };
    tiles: { position: string; pile: Pile; times: number }[];
    effects: { source: string; tokens: JournalToken[] }[];
  };
  setup?: { civilization: string; position?: string };
  event?: {
    triggeredBy?: number;
    info?: EventInfo;
    outcomes: JournalEntry[];
    explanations: { player?: number; text: string; protected?: boolean }[];
    pending?: string;
  };
}
export interface EventInfo {
  id: number;
  name: string;
  rules: string[];
  baseEffect: string | null;
  protectionAdvance: string | null;
  protectionSpecialAdvance: string | null;
  minimumCities?: number | null;
  targets: ('all' | 'active' | 'selected')[];
}
export interface Session {
  automaticPayment?: boolean;
  decisionSelection: number[];
  decisionPosition?: string | null;
  replacements: number[];
  collectVariant: Move;
  tilePanel: boolean;
  cityTab: 'build' | 'recruit' | 'happiness';
  collectionTile: string | null;
  moveTarget: string | null;
  selectedUnits: number[];
  unitPosition?: string | null;
  movingCity?: string | null;
  moveDestinations: MoveDestination[];
  moveDestination: number | null;
  cardsOpen: boolean;
  abilitiesOpen: boolean;
  abilityChoice?: string | null;
  abilityCity?: string | null;
  seaRoutes: boolean;
  seaRouteStart: string | null;
  game: Game | null;
  view: View | null;
  seat?: number;
  city: string | null;
  focus: string | null;
  mode: 'overview' | 'collect' | 'research' | 'city' | 'settlers';
  selectedSettler: number | null;
  destination: string | null;
  explorationRotation: number | null;
  explorationPreview: number | null;
  recruits: RecruitSelection;
  ballcourts?: boolean;
  draftCard?: boolean;
  attackPirates?: boolean;
  shogunateDraft?: boolean;
  piratePort?: boolean;
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
  unitBadges: boolean;
}
export interface MapPick {
  kind: 'tile' | 'city' | 'unit' | 'units' | 'decision';
  player?: number;
  unit?: number;
  decisionIndex?: number;
}
export interface UnitView {
  movementNotes?: string[];
  pirate?: boolean;
  id: number;
  type: string | { Leader: string };
  position: string;
  carrier: number | null;
}
export interface ActionVariant {
  payment?: Pile;
  value: Move;
  name: string;
  free: boolean;
  surcharge?: Pile;
}
export interface HappinessPreview {
  action: Move | null;
  payment: Pile;
  reason: string | null;
}
export interface MoveDestination {
  terrainNotes?: string[];
  label?: string;
  pirateCarrier?: number;
  position: string;
  terrain: Terrain;
  payment: Pile;
  carrier: number | null;
  attack: boolean;
  action: Move;
}
export interface ActionCard {
  id: number;
  name: string;
  description: string;
  free: boolean;
  cost: Pile;
  tactics: { name: string; description: string } | null;
  reason: string | null;
  action: Move | null;
}
export interface Decision {
  eventContext?: {
    name: string;
    rules: string[];
    raid: string | null;
    placement: string | null;
    card: {
      name: string;
      description: string;
      cost: Pile;
      free: boolean;
      later: boolean;
      firstOffer: boolean;
    } | null;
  } | null;
  advanceSelection?: boolean;
  advanceMode?: 'free' | 'paid' | 'borrow' | null;
  name: string;
  description: string;
  min: number;
  max: number;
  reward: boolean;
  endOfAge: boolean;
  options: {
    value: unknown;
    name: string;
    description: string;
    position: string | null;
    mapTarget?:
      | { kind: 'unit'; player: number; unit: number; unitType: UnitView['type'] }
      | { kind: 'structure'; structure: string | { Building: string } | { Wonder: string } };
    terrain?: Terrain;
    card?:
      | { kind: 'objective'; objectives: View['objectiveCards'][number]['objectives'] }
      | { kind: 'action'; name: string; description: string; tactics: ActionCard['tactics'] }
      | null;
  }[];
  fields: {
    name: string;
    optional: boolean;
    resources: Resource[];
    initial: Pile;
    cost: Pile;
    choices?: Pile[] | null;
  }[];
}
export const resourceNames: Record<Resource, string> = {
  food: 'Food',
  wood: 'Wood',
  ore: 'Ore',
  ideas: 'Ideas',
  gold: 'Gold',
  mood_tokens: 'Mood',
  culture_tokens: 'Culture',
  captives: 'Captives',
};
export const resources = Object.keys(resourceNames) as Resource[];
export const playerColors = ['#5086af', '#b96c4c', '#6d9173', '#ae92b6'];
const accessiblePlayerColors = ['#0072b2', '#d55e00', '#009e73', '#cc79a7', '#746800', '#333333'];
export const playerSymbol = (index: number) => ['●', '▲', '■', '◆', '✚', '✕'][index] ?? String(index + 1);
export const playerColor = (index: number, colorBlind: boolean) =>
  (colorBlind ? accessiblePlayerColors : playerColors)[index] ?? '#bfa986';
