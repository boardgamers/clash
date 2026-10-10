import { playerSymbolGlyph } from '@boardgamers/protocol/player-symbols';
export type Pile = Partial<
  Record<'food' | 'wood' | 'ore' | 'ideas' | 'gold' | 'mood_tokens' | 'culture_tokens' | 'captives', number>
>;
export type Resource = keyof Pile;
export type Move = string | Record<string, unknown>;
export type Terrain =
  'Forest' | 'Fertile' | 'Mountain' | 'Barren' | 'Water' | 'Unexplored' | { Exhausted: string };
export interface City {
  position: string;
  port_position?: string | null;
  mood_state: string;
  city_pieces?: Partial<
    Record<'academy' | 'market' | 'obelisk' | 'observatory' | 'fortress' | 'port' | 'temple', number>
  > & { wonders?: string[] };
  activations?: number;
  nomad_mountain?: boolean;
}
export interface Player {
  id: number;
  settings?: { skipRazeCity?: boolean };
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
export interface PublicEffect {
  player: number;
  kind: 'action' | 'objective' | 'wonder' | 'completed';
  label: string;
  key?: string;
  cursor?: number;
}
export interface BoardFrame {
  combat?: import('./active-combat').ActiveCombat;
  cursor: number;
  actor: number | null;
  ended_turn: boolean;
  title: string;
  age: number;
  round: number;
  tiles: [string, Terrain][];
  players: Player[];
  effects?: PublicEffect[];
}
export interface Playback {
  steps?: number[];
  frame: BoardFrame | null;
  index: number;
  total: number;
  start: number;
  end: number;
  range: 'all' | 'last-turn' | 'catch-up';
  automatic: boolean;
  playing: boolean;
  animate: boolean;
}
export interface Game {
  successful_cultural_influence?: boolean;
  options?: { length?: 'Standard' | 'Epic'; variant?: 'Standard' | 'Builder' };
  permanent_effects?: unknown[];
  board_history?: { id: string; frames: BoardFrame[] };
  state: unknown;
  events?: { event_type: string | Record<string, unknown> }[];
  players: Player[];
  map: {
    tiles: [string, Terrain][];
    unexplored_blocks?: { position: { top_tile: string; rotation: number } }[];
  };
  current_player_index: number;
  starting_player_index?: number;
  dropped_players?: number[];
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
  player?: number;
  action?: Move;
  log?: string[];
  items?: {
    player: number;
    origin?: Record<string, unknown>;
    Text?: string;
    Move?: { units: Record<string, number | string>; start: string; destination: string };
    Explore?: { tiles: [string, unknown][] };
    CombatRound?: {
      round: number;
      attackers: Record<string, number | string>;
      defenders: Record<string, number | string>;
      defending_player: number;
    };
    CombatRoll?: {
      rolls: {
        value: number;
        unit_type: string | { Leader: string };
        bonus: boolean;
        combat_bonus?: number;
      }[];
      combat_value: number;
      hits: number;
      combat_modifiers?: string[];
    };
    InfluenceCultureAttempt?: { position: string; starting_city_position: string; structure: unknown };
    Action?: { balance: string; amount?: number };
    Structure?: { structure: unknown; balance: string; position: string };
    HandCard?: { card?: unknown; from?: unknown; to: unknown };
    Advance?: {
      advance: string;
      balance: string;
      take_incident_token?: boolean;
      incident_token?: 'NoChange' | { Take: number };
    };
    Units?: { units: Record<string, number | string>; balance: string; position?: string };
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
export interface CollectionBonus {
  source: string;
  minimum: Pile;
  pile: Pile;
}
export interface CityView {
  collectionBonuses?: CollectionBonus[];
  ballcourts?: boolean;
  draftCard?: boolean;
  attackPirates?: boolean;
  shogunateDraft?: boolean;
  shogunateDraftCost?: number;
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
  triggersEvent?: boolean;
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
  completedObjectives?: { name: string; description: string; points: number }[];
  civilizationLeaders?: {
    id: string;
    name: string;
    recruited: boolean;
    reason?: string | null;
    abilities: { name: string; description: string }[];
  }[];
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
export interface InfluenceContext {
  stage: 'payment' | 'range' | 'reroll' | 'boost';
  source: string | null;
  target: string;
  name: string;
  roll: number | null;
  rollBonus: number;
  threshold: number;
}
export interface ActiveEffect {
  name: string;
  scope: string;
  players: number[];
  rules: string[];
}
export interface View {
  activeEffects?: ActiveEffect[];
  cardCatalog?: Pick<ActionCard, 'id' | 'name' | 'description' | 'free' | 'tactics'>[];
  influenceContext?: InfluenceContext | null;
  waitingFor?: { player: number; action: string; source: string | null } | null;
  logOriginNames?: Record<string, string>;
  activePlayers?: number[];
  civilizationDraft?: { ready: boolean[]; chosen: string | null; waiting: boolean } | null;
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
    origins?: {
      position: string;
      settlers: boolean;
      reroll: boolean;
      payment: Pile;
      action: Move;
      free?: boolean;
      actionPayment?: Pile;
      rollBonus?: number;
      preventBoost?: boolean;
    }[];
    free?: boolean;
    actionPayment?: Pile;
    rollBonus?: number;
    preventBoost?: boolean;
    variant: string;
    payment: Pile;
    action: Move;
  }[];
  units?: UnitView[];
  movementLeft?: number;
  nomadCities?: string[];
  seaRoutes?: string[][];
  barbarianGuide?: {
    player: number;
    spawn: string[];
    reinforce: string[];
    moves: { from: string; to: string }[];
  }[];
  pirateSpawns?: { player: number; first: string[]; second: string[] }[];
  activePlayer: number;
  canPlay: boolean;
  supportedPhase: boolean;
  players: PlayerView[];
  cities: CityView[];
  cityActions: CityActions[];
  settlers: SettlerView[];
  stopMovement: Move | null;
  choiceDecision?: {
    name: string;
    binary?: boolean;
    preview?: { name: string; rules: string[]; affected?: string; wonder?: WonderCard } | null;
    choices: { name: string; description?: string | null; pile?: Pile; action: Move }[];
  } | null;
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
  wonderCatalog?: WonderCard[];
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
  canRedo?: boolean;
  canEndTurn: boolean;
  endTurnTradeWarning?: { waste: Pile; routes: number } | null;
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
  /** Local prediction only; the server validates and executes every move. */
  tryMove?: (state: string, move: string, seat: number) => string;
  stripSecret?: (state: string, seat?: number) => string;
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
  landingTargets?: string[];
  disembarkCarriers?: number[];
  analysis?: boolean;
  playback?: Playback | null;
  publicEffects?: PublicEffect[];
  battles?: import('./battle-playback').BattleCue[];
  battleAnimate?: boolean;
  automaticPayment?: boolean;
  decisionSelection: number[];
  decisionPosition?: string | null;
  replacements: number[];
  collectVariant: Move;
  happinessSteps?: Record<string, number>;
  happinessVariant?: number;
  happinessLawgiver?: string | null;
  happinessCity?: string | null;
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
  /** Cultural influence target mode, entered from the abilities menu. */
  influenceMode?: boolean;
  influencePosition?: string | null;
  influenceTarget?: string | null;
  influenceOrigin?: string | null;
  seaRoutes: boolean;
  pirateSpawns: boolean;
  pirateSpawnPlayer?: number | null;
  threatGuide?: 'pirates' | 'barbarians';
  seaRouteStart: string | null;
  game: Game | null;
  view: View | null;
  seat?: number;
  city: string | null;
  focus: string | null;
  mode: 'overview' | 'collect' | 'research' | 'city' | 'settlers' | 'happiness';
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
  scoreTab?: 'score' | 'advances' | 'cities' | 'leaders' | 'objectives';
  scoreObjective?: string | null;
  cardDraws: CardDraw[];
  dark: boolean;
  unread: number;
  avatars: string[];
  reducedMotion: boolean;
  colorBlind: boolean;
  playerColors?: string[];
  playerSymbols?: string[];
  playerBadges?: ({ url: string; label: string } | undefined)[];
  sound: boolean;
  locale: string;
  selectedAdvance: string | null;
  toast: string;
  homeAtBottom: boolean;
  topDown: boolean;
  strategyMap: boolean;
  unitBadges: boolean;
  replayAutoplay: boolean;
  availableOnly: boolean;
  confirmMoves: boolean;
  skipRazeCity: boolean;
}
export interface MapPick {
  kind: 'tile' | 'city' | 'unit' | 'units' | 'decision';
  player?: number;
  unit?: number;
  decisionIndex?: number;
  /** A specific city structure model: `CityCenter`, `Building:Temple` or `Wonder:GreatGardens`. */
  structure?: string;
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
  breaksDiplomacy?: boolean;
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
  origin?: Record<string, unknown>;
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
  tacticsSelection?: boolean;
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
export const playerSymbol = (index: number, preferred: readonly string[] = []) =>
  playerSymbolGlyph(preferred[index], ['●', '▲', '■', '◆', '✚', '✕'][index] ?? String(index + 1));
export const playerColor = (index: number, colorBlind: boolean, preferred: readonly string[] = []) =>
  (colorBlind ? accessiblePlayerColors[index] : preferred[index] || playerColors[index]) ?? '#bfa986';
