// Printed Monumental Edition names verified against the English deck scans linked in
// docs/card-text-audit.json. Engine names remain stable for saved games and commands.
export const printedCardNames: Record<string, string> = {
  'Great Pyramid': 'Great Pyramids',
  'Large Civilization': 'Civilization!',
  'Scientific Lead': 'Academic',
  'Advanced Culture': 'Advanced',
  'Religious Fervor': 'Religious',
  General: 'Warlord',
  'Eureka!': 'Ideas!',
  Defiance: 'Against All Odds',
  'Happy Population': 'Happy',
  'Education Lead': 'Enlightened',
  'Sea Blockade': 'Blockade',
  'Optimized Storage': 'Well Stocked',
  'Naval Assault': 'Amphibious Assault',
  Wealth: 'Rich',
  'Large Fleet': 'Great Navy',
  'Ore Supplies': 'Ore!',
  'Large Army': 'Great Army',
  'Wood Supplies': 'Wood!',
  Fortifications: 'Fortified',
  'Food Supplies': 'Food!',
  Architecture: 'City Planner',
  'Goal Focused': 'Focused',
  Bold: 'Daring',
  'Culture Focus': 'Culture',
  'Legendary Battle': 'Legend',
  Consulate: 'Consulates',
  'Science Focus': 'Science',
  'Trade Focus': 'Merchant',
  Scavenger: 'Marauder',
  Metropolis: 'Citadel',
  Seafarers: 'Seafarer',
  'Barbarian Conquest': 'Assimilator',
  Expansionist: 'Dominion',
  'City Founder': 'New Founder',
  Resistance: 'Defiance',
  'Trade Power': 'Trader',
  'Shipping Routes': 'Sea Lanes',
  'Terror Regime': 'Draconian',
  'Diversified Research': 'Balanced',
  'Culture Power': 'Influence',
  'Magnificent Culture': 'Wonderful',
  Outpost: 'Outposts',
  'Star Gazers': 'Scientists',
  'Horse Power': 'Equestrian',
  Traders: 'Mercantile',
  Versatility: 'Tactical Diversity',
  Brutus: 'King Slayer',
  'Ivory Tower': 'Ivory Tide',
  Trample: 'Stampede',
  'High Culture': 'Civilization of Ages',
  'Sea Cleansing': 'Cleanse the Seas',
  'Quick Advance': 'Advance',
  'Hero General': 'Heroic General',
  Spy: 'Spies',
  Ideas: 'Leap of Knowledge',
  'Great Ideas': 'Good Ideas',
  'Cultural Takeover': 'Cultural Awe',
  'City Development': 'City Growth',
  'Production Focus': 'Focused Collection',
  Explorer: 'Explorers',
  'Mass Production': 'Mass Collection',
  'New Plans': 'New Goals',
  Synergies: 'Tech Synergy',
  'Teach Us': 'Teach Us Now!',
  'Technology Trade': 'Tech Exchange',
  'New Ideas': 'Idea Synergy',
  Encircled: 'Routing',
  Surprise: 'Tactical Surprise',
  Siege: 'The Siege',
  'Heavy Resistance': 'Defiant Stand',
  'Improved Defenses': 'Upgraded Defenses',
  'Defensive Formation': 'Prepared Defenses',
  Scout: 'Scouts',
  Pestilence: 'Plague',
  Epidemics: 'Epidemic',
  'A good year': 'A Fine Year',
  'An awesome year': 'A Good Year',
  'A fantastic year': 'A Splendid Year',
  'A successful year': 'A Prosperous Year',
  Vulcan: 'Volcano',
  'Heavy Earthquake': 'Major Earthquake',
  Flood: 'Local Flood',
  'Heavy Flood': 'Major Flood',
  Uprising: 'Revolution: Uprising',
  Envoy: 'Envoys Meet',
  Anarchy: 'Revolution: Anarchy',
  Guillotine: 'Revolution: “Et tu, Brute?”',
  'Scientific Trade': 'Meeting of Academics',
  'Flourishing Trade': 'Brisk Trade',
  Pandemics: 'Epidemic: Pandemic',
  'Famine: Vermin': 'Famine: Pests',
  'Famine: Draught': 'Famine: Drought',
  Fire: 'Wildfire',
  'Great Explorer': 'Great Discoverer',
  'Elder Statesman': 'Great Statesman',
  'Great Seer': 'Great Oracle',
  'Severe famine': 'Severe Famine',
};

export function printedCardName(name: string, kind?: 'objective'): string {
  // The objective and the event share an old name but have different printed titles.
  if (kind === 'objective' && name === 'Migration') return 'Great Migration';
  return printedCardNames[name] ?? name;
}

// Sentence-level replacement excludes generic words: "Ideas" is also a resource,
// and "General" can occur within Heroic General. Structured card references supply
// those names explicitly instead. Keep replacement simultaneous to avoid cascades.
const generic = new Set(['Ideas', 'General', 'Spy', 'Fire', 'Flood', 'Envoy']);
const alternatives = [...new Set([...Object.keys(printedCardNames), ...Object.values(printedCardNames)])]
  .filter((name) => !generic.has(name))
  .sort((a, b) => b.length - a.length)
  .map((name) => name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'));
const pattern = new RegExp(`(?<![\\p{L}\\p{N}_])(?:${alternatives.join('|')})(?![\\p{L}\\p{N}_])`, 'gu');
export function printedCardText(text: string): string {
  return text.replace(pattern, (name) => printedCardName(name));
}

export function printedCardPart<T extends { text: string; card?: unknown; objective?: unknown }>(part: T): T {
  return {
    ...part,
    text: part.objective
      ? printedCardName(part.text, 'objective')
      : part.card
        ? printedCardName(part.text)
        : printedCardText(part.text),
  };
}
