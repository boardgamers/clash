import {
  Wheat,
  Warehouse,
  Droplets,
  Beef,
  Pickaxe,
  Hammer,
  HeartPulse,
  Route,
  Fish,
  Ship,
  Shield,
  Map,
  BookOpen,
  School,
  GraduationCap,
  Brain,
  Swords,
  Castle,
  Anvil,
  Users,
  Sun,
  Flame,
  Church,
  Landmark,
  Handshake,
  Coins,
  Palette,
  Dumbbell,
  Drama,
  Calculator,
  Telescope,
  Vote,
  Scale,
  Heart,
  Store,
  Flag,
  Crown,
  Shovel,
  Scroll,
  ShieldCheck,
  Sparkles,
  Zap,
} from 'lucide-svelte';
import type { PublicAdvance } from './types';

// Reading aids must preserve ownership, timing, costs and limits, not just the benefit.
// The engine's complete rule text remains available on selection.
const advances = {
  Farming: [Wheat, 'Your cities can collect food from grassland and wood from forests.'],
  Storage: [Warehouse, 'Your food storage limit increases from 2 to 7.'],
  Irrigation: [Droplets, 'Your cities can collect food from barren land. You ignore Famine events.'],
  Husbandry: [
    Beef,
    'Once per turn, your collecting city can use one land tile 2 land spaces away, or two such tiles if you have Roads.',
  ],
  Mining: [Pickaxe, 'Your cities can collect ore from mountains.'],
  Engineering: [Hammer, 'Immediately draw 1 wonder card. You can construct wonders in your happy cities.'],
  Sanitation: [
    HeartPulse,
    'Each Recruit action: pay 1 mood token instead of resources for one settler. You ignore Pestilence and Epidemics events.',
  ],
  Roads: [
    Route,
    'Moving to or from your cities: pay 1 food + 1 ore per land unit or group to move up to 2 spaces and ignore terrain penalties. Cannot explore, embark or disembark.',
  ],
  Fishing: [Fish, 'Each Collect action: your city can collect food from one adjacent sea tile.'],
  Navigation: [
    Ship,
    'Your ships can sail around the map edge to the next sea space in either direction. Unexplored regions block further travel.',
  ],
  WarShips: [
    Shield,
    'Cancel 1 hit against you in round 1 of any naval battle, or when your army disembarks directly into a land battle.',
  ],
  Cartography: [
    Map,
    'Gain 1 idea per Move action in which you move a ship, plus 1 culture token if you use Navigation.',
  ],
  Writing: [BookOpen, 'Immediately draw 1 action card and 1 objective card.'],
  PublicEducation: [
    School,
    'Once per turn, gain 1 idea when one of your cities with an Academy collects resources.',
  ],
  FreeEducation: [
    GraduationCap,
    'After buying another advance with at least 1 gold or idea, you may pay 1 extra idea to gain 1 mood token.',
  ],
  Philosophy: [Brain, 'Gain 1 idea immediately, then 1 idea each time you gain a Science advance.'],
  Tactics: [
    Swords,
    'You can move your army units and play action cards for their tactics effects in combat.',
  ],
  Siegecraft: [
    Castle,
    'Before attacking a city with a Fortress, pay 2 wood to cancel its extra die and/or 2 ore to cancel its hit protection.',
  ],
  SteelWeapons: [
    Anvil,
    'Before a land battle, pay 1 ore for +2 combat value each round, or +1 if the enemy has Steel Weapons. With Metallurgy, pay no ore against enemies without Steel Weapons.',
  ],
  Draft: [Users, 'Each Recruit action: pay 1 mood token instead of resources for one infantry unit.'],
  Myths: [
    Sun,
    'When an event would lower your cities’ mood, pay 1 mood token per city you want to protect. Does not apply to Pirates.',
  ],
  Rituals: [Flame, 'During Increase Happiness, each resource you spend can replace 1 mood token.'],
  Priesthood: [
    Church,
    'Once per turn, research one Science advance without paying resources. The research action cost still applies.',
  ],
  StateReligion: [Landmark, 'Once per turn, omit the food cost when you construct a Temple.'],
  Bartering: [
    Handshake,
    'Once per turn, discard an action card to gain 1 gold or 1 culture token without using an action.',
  ],
  TradeRoutes: [
    Route,
    'At the start of your turn, pair your settlers or ships with non-angry cities of other players within 2 revealed spaces. Each unit and city can form only one route. Gain 1 food per route, up to 4.',
  ],
  Taxes: [
    Coins,
    'Once per turn, spend 1 action + 1 mood token to gain any mix of food, wood and ore: 1 resource per city you own. Your cities do not activate.',
  ],
  Currency: [Coins, 'Your Trade Routes and Taxes can yield gold instead of food.'],
  Arts: [
    Palette,
    'Once per turn, pay 1 culture token to take an Influence Culture action without spending an action.',
  ],
  Sports: [
    Dumbbell,
    'Spend 1 action to raise the mood of one of your cities: 1 culture token per step, regardless of city size.',
  ],
  Monuments: [
    Landmark,
    'Immediately draw 1 wonder card. Your cities with wonders are immune to opponents’ Cultural Influence attempts.',
  ],
  Theaters: [
    Drama,
    'Once per turn, exchange 1 mood token for 1 culture token, or vice versa, without using an action.',
  ],
  Math: [
    Calculator,
    'Research Engineering and Roads without paying resources. The research action cost still applies.',
  ],
  Astronomy: [
    Telescope,
    'Research Navigation and Cartography without paying resources. The research action cost still applies.',
  ],
  Medicine: [
    HeartPulse,
    'After each Recruit action, recover 1 resource you paid (not mood or culture tokens). You must afford the full cost first.',
  ],
  Metallurgy: [
    Anvil,
    'Steel Weapons costs no ore against enemies without Steel Weapons. When collecting 2+ ore, replace 1 ore with 1 gold.',
  ],
  Voting: [
    Vote,
    'Take an Increase Happiness action without spending an action. Pay the normal mood cost plus 1 extra mood token total, regardless of how many cities you improve.',
  ],
  SeparationOfPower: [
    Scale,
    'Opponents cannot spend culture tokens to boost influence range or rolls against your happy cities.',
  ],
  CivilLiberties: [
    Heart,
    'Spend 1 action to gain 3 mood tokens. Your Draft now costs 2 mood tokens for its one infantry unit per Recruit action.',
  ],
  FreeEconomy: [
    Store,
    'Pay 1 mood token to activate one of your cities and collect without spending an action. This must be your only Collect action that turn.',
  ],
  Nationalism: [
    Flag,
    'After a Recruit action that includes at least one army unit or ship, gain 1 mood or 1 culture token in total.',
  ],
  Totalitarianism: [
    Shield,
    'Opponents cannot spend culture tokens to boost influence range or rolls against your cities containing your army units.',
  ],
  AbsolutePower: [
    Crown,
    'Once per turn, pay 2 mood tokens to gain 1 extra action. Using this ability costs no action.',
  ],
  ForcedLabor: [
    Shovel,
    'Once per turn, pay 1 mood token without using an action to treat all your angry cities as neutral for the rest of your turn. Each can still activate only once.',
  ],
  Dogma: [
    Scroll,
    'Whenever you construct a Temple, gain 1 Theocracy advance for free. Your idea limit becomes 2; discard any excess immediately. Capturing a Temple does not trigger this.',
  ],
  Devotion: [
    ShieldCheck,
    'Opponents cannot spend culture tokens to boost influence range or rolls against your cities with Temples.',
  ],
  Conversion: [
    Sparkles,
    'Add +1 to your Influence Culture rolls. Gain 1 culture token after each successful attempt.',
  ],
  Fanaticism: [
    Zap,
    'Attacking or defending a city with a Temple: +2 combat value in round 1. After losing a battle, place 1 infantry from your supply in one of your cities.',
  ],
} as const;

const civilizationIcons: Record<string, typeof Wheat> = {
  Aqueduct: Droplets,
  RomanRoads: Route,
  Captivi: Swords,
  Provinces: Flag,
  Study: BookOpen,
  Sparta: Shield,
  HellenisticCulture: Palette,
  CityStates: Landmark,
  RiceCultivation: Wheat,
  Expansion: Map,
  Fireworks: Sparkles,
  ImperialArmy: Swords,
  ShipConstruction: Hammer,
  Longships: Ship,
  Raiding: Swords,
  RuneStones: Scroll,
  Canals: Droplets,
  CodeOfLaws: Scale,
  StarCatalogues: Telescope,
  Ziggurats: Church,
  IndianElephants: Shield,
  Proselytism: Users,
  Prosperity: Handshake,
  PeaceAndPoetry: Scroll,
};

export const groupIcons: Record<string, typeof Wheat> = {
  Agriculture: Wheat,
  Construction: Hammer,
  Seafaring: Ship,
  Education: GraduationCap,
  Warfare: Swords,
  Spirituality: Sun,
  Economy: Coins,
  Culture: Palette,
  Science: Telescope,
  Democracy: Vote,
  Autocracy: Crown,
  Theocracy: Church,
};
export function researchPresentation(advance: Pick<PublicAdvance, 'id' | 'group' | 'description'>) {
  const entry = advances[advance.id as keyof typeof advances];
  return {
    icon: entry?.[0] ?? civilizationIcons[advance.id] ?? groupIcons[advance.group] ?? BookOpen,
    summary:
      advance.id === 'Philosophy' && advance.description.includes('3 ideas')
        ? advance.description
        : (entry?.[1] ?? advance.description),
  };
}
