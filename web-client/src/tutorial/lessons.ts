import type { TutorialOptions, TutorialStep } from '@boardgamers/protocol/tutorial';
import type { Move } from '../types.ts';

export interface TutorialEngine {
  tryMove(game: string, action: string, player: number): string;
  stripSecret(game: string, player?: number): string;
  webView(game: string, player?: number): string;
  webQuery(game: string, player: number, query: string): string;
  currentPlayer(game: string): number;
}
export interface State {
  game: string;
  done: string[];
}
export type Action =
  { step: string; kind: 'move'; move: Move } | { step: string; kind: 'answer'; answer: string };
export interface Offer {
  label: string;
  action: Action;
}
type Kind =
  | 'collect'
  | 'storage'
  | 'math'
  | 'engineering'
  | 'end'
  | 'happiness'
  | 'walk'
  | 'stop'
  | 'found'
  | 'explore'
  | 'orientation'
  | 'embark'
  | 'sail'
  | 'land'
  | 'influence'
  | 'range'
  | 'boost'
  | 'incident'
  | 'protect'
  | 'leader'
  | 'lighthouse'
  | 'draft'
  | 'objective'
  | 'undo';
interface LessonStep {
  id: string;
  title: string;
  text: string;
  kind?: Kind;
  answers?: string[];
  correct?: string;
  hint?: string;
}
export const sections = [
  {
    id: 'basics',
    title: 'The basics',
    description: 'Take your first turn, improve your cities and plan research.',
  },
  {
    id: 'expansion',
    title: 'Expansion',
    description: 'Move groups, explore regions and transport units by sea.',
  },
  {
    id: 'interaction',
    title: 'Interaction',
    description: 'Resolve cultural influence and incidents one decision at a time.',
  },
  { id: 'mastery', title: 'Mastery', description: 'Understand leaders, wonders and objective timing.' },
  {
    id: 'platform',
    title: 'Platform tools',
    description: 'Read the journal, review actions and understand undo.',
  },
];
export const chapters: {
  id: string;
  title: string;
  section: string;
  description: string;
  version: number;
  steps: LessonStep[];
}[] = [
  {
    id: 'first-turn',
    title: 'Your first turn',
    section: 'basics',
    description: 'Learn the three-action turn through collection and research.',
    version: 1,
    steps: [
      {
        id: 'turn',
        title: 'Your turn, three actions',
        text: 'The active civilization is marked in turn order. You have three actions. Opening a panel or selecting a piece does not spend an action.',
      },
      {
        id: 'collect',
        title: 'Collect once',
        text: 'Collect from your happy city. Its size is 1, so its collection capacity is 2. The city becomes activated.',
        kind: 'collect',
      },
      {
        id: 'research',
        title: 'Research Storage',
        text: 'Research normally costs two resources in any mix of food, ideas and gold. Storage increases your food limit. Research also removes an event marker.',
        kind: 'storage',
      },
      {
        id: 'pass',
        title: 'Finish your turn',
        text: 'One action remains. You may end your turn without using every action. End the turn now.',
        kind: 'end',
      },
    ],
  },
  {
    id: 'city-economy',
    title: 'A productive city',
    section: 'basics',
    description: 'Understand happy-city capacity, terrain and storage.',
    version: 1,
    steps: [
      {
        id: 'capacity',
        title: 'Size and capacity',
        text: 'This happy size-2 city can collect up to 3 resources. A neutral city of the same size collects up to 2. Buildings increase size; happiness increases capacity.',
      },
      {
        id: 'collection',
        title: 'Choose useful yields',
        text: 'Food storage is already full. Collect useful resources and inspect the overflow warning before confirming. Forest provides wood; mountains provide ore.',
        kind: 'collect',
      },
      {
        id: 'overflow',
        title: 'Storage overflow',
        text: 'What happens to resources beyond the storage limit?',
        answers: ['The excess is lost', 'The excess becomes gold', 'It carries over to the next turn'],
        correct: 'The excess is lost',
        hint: 'Collection does not raise storage limits. Mood and culture tokens have no storage limit.',
      },
    ],
  },
  {
    id: 'activation-happiness',
    title: 'Activation and happiness',
    section: 'basics',
    description: 'Improve multiple cities in one action without activating them.',
    version: 1,
    steps: [
      {
        id: 'mood',
        title: 'Read the activation marker',
        text: 'Your neutral city has already been activated this turn. Another activation lowers its mood. Increasing happiness is a separate action and does not activate cities.',
      },
      {
        id: 'improve',
        title: 'Improve both cities',
        text: 'Improve the neutral city to Happy and the angry city to Neutral in one action. Each step costs mood tokens equal to the city size.',
        kind: 'happiness',
      },
      {
        id: 'collect',
        title: 'Activate the improved city',
        text: 'Collect from your original city. Its second activation lowers its mood from Happy to Neutral. Compare the mood icon with the activation count.',
        kind: 'collect',
      },
      {
        id: 'question',
        title: 'Two different mechanics',
        text: 'Does increasing happiness activate a city?',
        answers: ['No, it changes mood only', 'Yes, it always activates the city'],
        correct: 'No, it changes mood only',
      },
    ],
  },
  {
    id: 'research-paths',
    title: 'Research with a plan',
    section: 'basics',
    description: 'Follow prerequisites and discover advances that become free.',
    version: 1,
    steps: [
      {
        id: 'math',
        title: 'Research Math first',
        text: 'Math makes Engineering and Roads free of resource cost. The tree shows a “Free with Math” hint before Math is unlocked. Research Math now.',
        kind: 'math',
      },
      {
        id: 'engineering',
        title: 'Claim the free research',
        text: 'Research Engineering. It costs no resources because you own Math, but it still uses a research action and an event marker.',
        kind: 'engineering',
      },
      {
        id: 'question',
        title: 'Free resources or free action?',
        text: 'What does Math waive for Engineering?',
        answers: ['The resource payment', 'The action and every prerequisite'],
        correct: 'The resource payment',
      },
      {
        id: 'civs',
        title: 'Civilization advances',
        text: 'Civilization advances unlock when their prerequisites are met. Inspect your civilization below the regular research tree, then inspect other civilizations and their leaders.',
      },
    ],
  },
  {
    id: 'movement-founding',
    title: 'Move, explore, found',
    section: 'expansion',
    description: 'Separate movement groups from actions and founding.',
    version: 1,
    steps: [
      {
        id: 'walk',
        title: 'Move the settler',
        text: 'A Move action can move up to three groups. Move the first settler to the nearby forest. Army movement requires Tactics; settlers do not.',
        kind: 'walk',
      },
      {
        id: 'stop',
        title: 'Finish the Move action',
        text: 'The Move action remains open for other groups. Finish movement before founding. A single unit cannot join two groups in the same Move action.',
        kind: 'stop',
      },
      {
        id: 'found',
        title: 'Found a separate city',
        text: 'Found a city on the settler’s empty land tile. Founding spends a separate action and replaces the settler with a settlement.',
        kind: 'found',
      },
      {
        id: 'explore',
        title: 'Explore a region',
        text: 'Move the second settler into unexplored land. Exploration reveals a four-hex region, rather than one isolated hex.',
        kind: 'explore',
      },
      {
        id: 'orientation',
        title: 'Choose the revealed region',
        text: 'Choose the orientation offered by the engine. The revealed terrain must allow the exploring unit to enter.',
        kind: 'orientation',
      },
    ],
  },
  {
    id: 'sea-transport',
    title: 'Ships and passengers',
    section: 'expansion',
    description: 'Board, sail and land with the right movement timing.',
    version: 1,
    steps: [
      {
        id: 'embark',
        title: 'Board the ship',
        text: 'Each ship can carry two land units. Board the nearby ship with your settler. Boarding uses that land group’s move.',
        kind: 'embark',
      },
      {
        id: 'sail',
        title: 'Sail with the passenger',
        text: 'Move the ship to the next sea tile. Its passenger travels with it, within the same Move action.',
        kind: 'sail',
      },
      {
        id: 'timing',
        title: 'Landing after boarding',
        text: 'Can this newly boarded settler land during the same Move action?',
        answers: ['No, it needs another Move action', 'Yes, boarding gives an extra move'],
        correct: 'No, it needs another Move action',
      },
      {
        id: 'stop',
        title: 'This Move action has finished',
        text: 'All eligible groups have moved, so the engine has finished this Move action. Units already aboard before a Move action may sail and land in that action.',
      },
      {
        id: 'land',
        title: 'Land on empty land',
        text: 'Use another Move action to land the passenger on adjacent empty land.',
        kind: 'land',
      },
    ],
  },
  {
    id: 'cultural-influence',
    title: 'Cultural influence',
    section: 'interaction',
    description: 'Pay for range, roll, then decide whether to boost the result.',
    version: 1,
    steps: [
      {
        id: 'target',
        title: 'Choose source and target',
        text: 'Target the enemy Temple from your city. Influence changes a building’s color; it does not transfer control of the city.',
        kind: 'influence',
      },
      {
        id: 'range',
        title: 'Pay for range before rolling',
        text: 'The target lies beyond your city’s normal range. Pay the required culture tokens to extend range. The roll is still hidden at this stage.',
        kind: 'range',
      },
      {
        id: 'boost',
        title: 'Read the roll and shortfall',
        text: 'The roll is below 5. You may pay culture tokens for the remaining shortfall, or decline and keep them. Pay for success in this lesson.',
        kind: 'boost',
      },
      {
        id: 'ownership',
        title: 'Building color and city owner',
        text: 'Who can still use the influenced building?',
        answers: ['The city’s owner', 'Only the influencing civilization'],
        correct: 'The city’s owner',
        hint: 'You gain the building’s point; the city’s owner still uses it. Only one successful influence is allowed each turn.',
      },
    ],
  },
  {
    id: 'incidents-hostiles',
    title: 'Incidents and hostile units',
    section: 'interaction',
    description: 'Read affected cities and resolve protection choices.',
    version: 1,
    steps: [
      {
        id: 'trigger',
        title: 'Trigger the incident',
        text: 'Your next advance uses the final event marker. Research Storage and read the incident that appears.',
        kind: 'incident',
      },
      {
        id: 'protect',
        title: 'Protect affected cities',
        text: 'Read which cities are affected. Myths can offer mood-token payments to protect them. Choose the offered protection; incidents do not always affect just one city.',
        kind: 'protect',
      },
      {
        id: 'hostiles',
        title: 'Recognize hostile units',
        text: 'Pirates carry flags and affect sea collection and Trade Routes. Barbarians wear horned helmets, and their cities use a helmet flag. Their placement and movement guides explain incident effects; they are not extra player actions.',
      },
      {
        id: 'question',
        title: 'Read the target list',
        text: 'Should you assume every incident targets exactly one city?',
        answers: ['No, read this incident’s affected cities', 'Yes, incidents always target one city'],
        correct: 'No, read this incident’s affected cities',
      },
    ],
  },
  {
    id: 'cards-leaders',
    title: 'Cards, leaders and free actions',
    section: 'mastery',
    description: 'Recruit a leader and distinguish payments from action costs.',
    version: 1,
    steps: [
      {
        id: 'leader',
        title: 'Recruit a leader',
        text: 'Recruit Caesar in your city. This costs one mood token, one culture token and one action, and activates the city. A leader is an army unit and needs Tactics to move.',
        kind: 'leader',
      },
      {
        id: 'free',
        title: 'A free action can still cost resources',
        text: 'Does “free action” always mean there is no resource payment?',
        answers: ['No, pay any listed resource cost', 'Yes, every payment is waived'],
        correct: 'No, pay any listed resource cost',
        hint: 'Great Prophet grants a free construction action but still requires the Temple payment. Read each effect’s exact wording.',
      },
      {
        id: 'draft',
        title: 'Different kinds of draft',
        text: 'Draft recruits one unit by military conscription. Shogunate card drafting and card play have separate allowances. Neither is the civilization-selection draft.',
      },
    ],
  },
  {
    id: 'wonders-ownership',
    title: 'Wonders and ownership',
    section: 'mastery',
    description: 'Use Great Lighthouse and inspect builder versus owner points.',
    version: 1,
    steps: [
      {
        id: 'activate',
        title: 'Activate Great Lighthouse’s city',
        text: 'Collect from the city containing Great Lighthouse. Its optional ship follows this activation; it is not a second free city activation.',
        kind: 'collect',
      },
      {
        id: 'ship',
        title: 'Choose the optional ship',
        text: 'Choose the legal sea tile for the free ship. The city’s activation count and your remaining actions do not change again.',
        kind: 'lighthouse',
      },
      {
        id: 'score',
        title: 'Builder points and ownership',
        text: 'How many wonder points does merely owning a captured Great Pyramid give?',
        answers: ['0 wonder points', '5.1 wonder points'],
        correct: '0 wonder points',
        hint: 'The civilization that built Great Pyramid receives 5.1 points. Ownership alone gives 0 wonder points.',
      },
      {
        id: 'conditions',
        title: 'Read each wonder’s trigger',
        text: 'Great Mausoleum benefits its owner. Great Explorer requires its qualifying exploration. A wonder’s ability must satisfy its printed conditions; ownership is not a general permission to activate it freely.',
      },
    ],
  },
  {
    id: 'objectives-ages',
    title: 'Objectives and the end of an age',
    section: 'mastery',
    description: 'Meet the exact objective condition and distinguish claim timing.',
    version: 1,
    steps: [
      {
        id: 'draft1',
        title: 'Meet the first condition',
        text: 'Your Draft objective requires recruiting with Draft twice this turn. Recruit one infantry using Draft in the legal city.',
        kind: 'draft',
      },
      {
        id: 'draft2',
        title: 'Meet the complete condition',
        text: 'Use Draft a second time. One qualifying recruitment alone was not enough for this objective.',
        kind: 'draft',
      },
      {
        id: 'claim',
        title: 'Claim the eligible objective',
        text: 'Claim Draft when the engine offers it. During-play objectives can be claimed when their condition is met; age-end objectives wait for their check.',
        kind: 'objective',
      },
      {
        id: 'age',
        title: 'The final age',
        text: 'Does the final age give another free advance and new cards after objective checks?',
        answers: ['No, the game ends before those rewards', 'Yes, those rewards always happen'],
        correct: 'No, the game ends before those rewards',
      },
    ],
  },
  {
    id: 'platform-tools',
    title: 'Read, review and recover',
    section: 'platform',
    description: 'Use rule references and understand the boundary of undo.',
    version: 1,
    steps: [
      {
        id: 'journal',
        title: 'Follow a journal reference',
        text: 'Open the journal and select Storage to inspect its rules. Coordinates can locate places on the map. Last turn reviews actual actions and skips empty phase changes.',
      },
      {
        id: 'undo',
        title: 'Undo the harmless research',
        text: 'Undo the research in this isolated example. Undo cannot cross an information reveal. Visual replay and analysis are separate tools.',
        kind: 'undo',
      },
      {
        id: 'analysis',
        title: 'Analysis is an independent branch',
        text: 'Does trying an action in analysis change the live game?',
        answers: ['No, analysis is independent', 'Yes, analysis submits live moves'],
        correct: 'No, analysis is independent',
      },
      {
        id: 'settings',
        title: 'Platform preferences',
        text: 'Available actions only filters the choices shown; it does not make decisions for you. Colorblind display and map orientation are available in platform settings.',
      },
    ],
  },
];

const canonical = (value: unknown): string =>
  JSON.stringify(value, (_key, item) =>
    item && typeof item === 'object' && !Array.isArray(item)
      ? Object.fromEntries(Object.entries(item).sort(([a], [b]) => a.localeCompare(b)))
      : item,
  );
export function createLesson(id: string, engine: TutorialEngine, initialGame: string) {
  const chapter = chapters.find((chapter) => chapter.id === id);
  if (!chapter) throw new Error(`Unknown Clash of Cultures chapter: ${id}`);
  const view = (state: State): any => JSON.parse(engine.webView(engine.stripSecret(state.game, 0), 0));
  const query = (state: State, input: unknown): any =>
    JSON.parse(engine.webQuery(engine.stripSecret(state.game, 0), 0, JSON.stringify(input)));
  function moves(state: State, kind: Kind): { label: string; move: Move }[] {
    const v = view(state),
      city =
        v.cities?.find((c: any) => c.player === 0)?.position ??
        JSON.parse(state.game).players[0].cities[0].position;
    const one = (label: string, move: Move | null | undefined) => (move ? [{ label, move }] : []);
    if (['storage', 'math', 'engineering', 'incident'].includes(kind)) {
      const advance = kind === 'math' ? 'Math' : kind === 'engineering' ? 'Engineering' : 'Storage';
      return one(`Research ${advance}`, v.advances.find((a: any) => a.id === advance)?.action);
    }
    if (kind === 'collect') {
      const c = v.cities.find((c: any) => c.position === city);
      const selections = c.choices
        .filter((c: any) => !c.gain?.food)
        .slice(0, 2)
        .map((c: any) => ({ ...c, times: 1 }));
      if (!selections.length) selections.push({ ...c.choices[0], times: 1 });
      return one(
        'Collect resources',
        query(state, { kind: 'collect', city, selections, variant: 'Collect' }).action,
      );
    }
    if (kind === 'happiness')
      return one(
        'Improve both cities',
        query(state, {
          kind: 'happiness',
          cities: JSON.parse(state.game).players[0].cities.map((c: any) => [c.position, 1]),
          variant: v.happinessActions[0].value,
          lawgiver: false,
        }).action,
      );
    if (kind === 'end') return one('End turn', v.canEndTurn ? { Playing: 'EndTurn' } : null);
    if (kind === 'stop') return one('Finish movement', v.stopMovement);
    if (kind === 'undo') return one('Undo the last action', v.canUndo ? 'Undo' : null);
    if (kind === 'found')
      return v.settlers
        .filter((s: any) => s.foundAction)
        .map((s: any) => ({ label: `Found city at ${s.position}`, move: s.foundAction }));
    if (['walk', 'explore'].includes(kind))
      return v.settlers
        .flatMap((s: any) =>
          s.destinations
            .filter((d: any) => (kind === 'walk' ? d.terrain === 'Forest' : d.terrain === 'Unexplored'))
            .map((d: any) => ({ label: `Move to ${d.position}`, move: d.action })),
        )
        .slice(0, 3);
    if (kind === 'orientation')
      return (v.explorationDecision?.choices ?? []).map((c: any) => ({
        label: `Choose orientation ${c.rotation}`,
        move: c.action,
      }));
    if (['embark', 'sail', 'land'].includes(kind)) {
      const unit = v.units.find((u: any) => (kind === 'sail' ? u.type === 'Ship' : u.type === 'Settler'));
      if (!unit) return [];
      return query(state, { kind: 'movement', units: [unit.id], city: null })
        .destinations.filter((d: any) =>
          kind === 'embark'
            ? d.position === 'D1'
            : kind === 'sail'
              ? d.position === 'D3'
              : d.position === 'E3',
        )
        .map((d: any) => ({ label: `Move to ${d.position}`, move: d.action }));
    }
    if (kind === 'influence') {
      const target = v.influence?.find((t: any) => t.position === 'C1' && t.name === 'Temple');
      return one('Influence the Temple', target?.origins?.find((o: any) => o.position === 'A1')?.action);
    }
    if (['range', 'boost', 'protect'].includes(kind)) {
      const field = v.decision?.fields?.[0];
      if (!field) return [];
      const payment = field.cost;
      return one('Pay the listed cost', { Response: { Payment: [payment ?? field.cost] } });
    }
    if (kind === 'lighthouse') return one('Place the free ship', { Response: { SelectPositions: ['D1'] } });
    if (kind === 'leader')
      return one(
        'Recruit Caesar',
        query(state, { kind: 'recruit', city, units: { leader: 'Caesar' }, replaced: [] }).action,
      );
    if (kind === 'draft')
      return one(
        'Recruit one infantry with Draft',
        query(state, {
          kind: 'recruit',
          city: 'A1',
          units: { infantry: 1 },
          replaced: [],
          payment: { mood_tokens: 1 },
        }).action,
      );
    if (kind === 'objective')
      return (v.objectiveDecision?.cards ?? [])
        .filter((o: any) => o.name?.includes('Draft'))
        .map((o: any) => ({ label: o.name, move: o.action }));
    return [];
  }
  function offers(step: LessonStep, state: State): Offer[] {
    if (step.answers) {
      const shift = [...step.id].reduce((sum, char) => sum + char.charCodeAt(0), 0) % step.answers.length;
      return [...step.answers.slice(shift), ...step.answers.slice(0, shift)].map((answer) => ({
        label: answer,
        action: { kind: 'answer', answer, step: step.id },
      }));
    }
    return step.kind
      ? moves(state, step.kind).map(({ label, move }) => ({
          label,
          action: { kind: 'move', move, step: step.id },
        }))
      : [];
  }
  const steps: (TutorialStep<State, Action> & { offers: (state: State) => Offer[] })[] = chapter.steps.map(
    (step) => ({
      ...step,
      target: step.kind ? 'actions' : undefined,
      offers: (state) => offers(step, state),
      complete: step.kind || step.answers ? (state) => state.done.includes(step.id) : undefined,
      validateMove: (_state, action) =>
        action.step !== step.id
          ? 'Follow the current lesson step.'
          : action.kind === 'answer' && action.answer !== step.correct
            ? (step.hint ?? 'Read the explanation and try again.')
            : undefined,
    }),
  );
  function resolveIncidentOpponents(game: string): string {
    for (let guard = 0; guard < 20; guard++) {
      const player = engine.currentPlayer(game);
      if (player === 0) return game;
      const publicState = engine.stripSecret(game, player);
      const decision = JSON.parse(engine.webView(publicState, player)).decision;
      if (!decision) return game;
      const values = decision.options.slice(0, decision.min).map((option: any) => option.value);
      const payments = decision.fields.map(
        (field: any) =>
          field.choices?.find((pile: any) => Object.keys(pile).length === 0) ??
          field.choices?.[0] ??
          field.cost,
      );
      const offer = JSON.parse(
        engine.webQuery(publicState, player, JSON.stringify({ kind: 'decision', values, payments })),
      );
      game = engine.tryMove(game, JSON.stringify(offer.action), player);
    }
    throw new Error('Follow the current lesson step.');
  }
  const lesson: TutorialOptions<State, Action> & { steps: typeof steps } = {
    game: 'clash',
    id,
    version: chapter.version,
    steps,
    initialState: () => ({ game: initialGame, done: [] }),
    move: (state, action) => {
      const step = chapter.steps.find((step) => step.id === action.step);
      if (!step || !offers(step, state).some((offer) => canonical(offer.action) === canonical(action)))
        throw new Error('Use one of the actions offered for this lesson step.');
      return {
        game:
          action.kind === 'move'
            ? id === 'incidents-hostiles'
              ? resolveIncidentOpponents(engine.tryMove(state.game, JSON.stringify(action.move), 0))
              : engine.tryMove(state.game, JSON.stringify(action.move), 0)
            : state.game,
        done: [...state.done, step.id],
      };
    },
    completion: {
      title: 'Chapter complete',
      text: 'You have completed this lesson. Restart to try it again, or continue to the next chapter.',
    },
  };
  return lesson;
}
