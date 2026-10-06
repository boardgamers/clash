export interface PieceStyle {
  roof: 'gable' | 'roman' | 'swept' | 'pagoda' | 'flat' | 'stepped' | 'dome' | 'timber' | 'thatch' | 'tent';
  helmet:
    | 'crest'
    | 'legion'
    | 'lamellar'
    | 'kabuto'
    | 'cloth'
    | 'tiara'
    | 'turban'
    | 'nasal'
    | 'cap'
    | 'feather'
    | 'horned';
  shield: 'round' | 'rectangular' | 'oval' | 'wicker';
  ship: 'galley' | 'junk' | 'longship' | 'reed' | 'dhow' | 'merchant' | 'canoe';
  wall: string;
  roofColor: string;
  trim: string;
  wood: string;
  metal: string;
  cloth: string;
}

// Stylized visual vocabulary, independent of player colors and game rules.
// Each faction changes geometry as well as materials; type silhouettes remain shared.
export const civilizationPieceStyles: Record<string, PieceStyle> = {
  Barbarians: {
    roof: 'thatch',
    helmet: 'horned',
    shield: 'round',
    ship: 'merchant',
    wall: '#b7a17a',
    roofColor: '#947750',
    trim: '#887454',
    wood: '#6f523b',
    metal: '#84908c',
    cloth: '#bd9b76',
  },
  Greece: {
    roof: 'gable',
    helmet: 'crest',
    shield: 'round',
    ship: 'galley',
    wall: '#f1e5cc',
    roofColor: '#b46846',
    trim: '#527f9b',
    wood: '#856140',
    metal: '#bd9854',
    cloth: '#f1e5cc',
  },
  Rome: {
    roof: 'roman',
    helmet: 'legion',
    shield: 'rectangular',
    ship: 'galley',
    wall: '#e0c6a4',
    roofColor: '#9f4738',
    trim: '#a94f41',
    wood: '#73503b',
    metal: '#929b9e',
    cloth: '#b45949',
  },
  China: {
    roof: 'swept',
    helmet: 'lamellar',
    shield: 'rectangular',
    ship: 'junk',
    wall: '#e7d9bd',
    roofColor: '#447e6c',
    trim: '#a14b38',
    wood: '#714b35',
    metal: '#617d7c',
    cloth: '#dcc391',
  },
  Japan: {
    roof: 'pagoda',
    helmet: 'kabuto',
    shield: 'rectangular',
    ship: 'junk',
    wall: '#eee5d4',
    roofColor: '#4b5b66',
    trim: '#9f4539',
    wood: '#554535',
    metal: '#4c5862',
    cloth: '#c6ab86',
  },
  Egypt: {
    roof: 'flat',
    helmet: 'cloth',
    shield: 'oval',
    ship: 'reed',
    wall: '#dec18a',
    roofColor: '#bc955f',
    trim: '#338f99',
    wood: '#967044',
    metal: '#b29354',
    cloth: '#f3ead3',
  },
  Babylonia: {
    roof: 'stepped',
    helmet: 'tiara',
    shield: 'wicker',
    ship: 'reed',
    wall: '#b79369',
    roofColor: '#687eae',
    trim: '#42629a',
    wood: '#75583f',
    metal: '#b49a63',
    cloth: '#b8a1cd',
  },
  Persia: {
    roof: 'flat',
    helmet: 'tiara',
    shield: 'wicker',
    ship: 'galley',
    wall: '#d5c8af',
    roofColor: '#89a8aa',
    trim: '#638696',
    wood: '#77563d',
    metal: '#b2a16c',
    cloth: '#a57b91',
  },
  India: {
    roof: 'dome',
    helmet: 'turban',
    shield: 'round',
    ship: 'dhow',
    wall: '#d6b390',
    roofColor: '#b9865e',
    trim: '#cc9d57',
    wood: '#835940',
    metal: '#b8a26a',
    cloth: '#e9c16c',
  },
  Vikings: {
    roof: 'timber',
    helmet: 'nasal',
    shield: 'round',
    ship: 'longship',
    wall: '#99734d',
    roofColor: '#655944',
    trim: '#c5a36a',
    wood: '#634a36',
    metal: '#839095',
    cloth: '#9d6c4b',
  },
  Celts: {
    roof: 'thatch',
    helmet: 'cap',
    shield: 'oval',
    ship: 'merchant',
    wall: '#bda47d',
    roofColor: '#b49a53',
    trim: '#6c8260',
    wood: '#75563a',
    metal: '#a19666',
    cloth: '#8d9b6f',
  },
  Huns: {
    roof: 'tent',
    helmet: 'cap',
    shield: 'round',
    ship: 'merchant',
    wall: '#dacdb3',
    roofColor: '#b3a083',
    trim: '#92664b',
    wood: '#715440',
    metal: '#798485',
    cloth: '#927456',
  },
  Maya: {
    roof: 'stepped',
    helmet: 'feather',
    shield: 'round',
    ship: 'canoe',
    wall: '#e0dac4',
    roofColor: '#a65c46',
    trim: '#4c9c95',
    wood: '#855936',
    metal: '#63726d',
    cloth: '#dccbb0',
  },
  Aztecs: {
    roof: 'stepped',
    helmet: 'feather',
    shield: 'round',
    ship: 'canoe',
    wall: '#a2a69a',
    roofColor: '#c26648',
    trim: '#4b9086',
    wood: '#755135',
    metal: '#4c5350',
    cloth: '#c58f61',
  },
  Phoenicia: {
    roof: 'flat',
    helmet: 'cloth',
    shield: 'round',
    ship: 'galley',
    wall: '#e4d3b4',
    roofColor: '#a880a0',
    trim: '#986693',
    wood: '#886344',
    metal: '#b29a66',
    cloth: '#b190ba',
  },
  Carthage: {
    roof: 'roman',
    helmet: 'crest',
    shield: 'oval',
    ship: 'galley',
    wall: '#d4bd9a',
    roofColor: '#778f8e',
    trim: '#6f8d8b',
    wood: '#654b3e',
    metal: '#b5a075',
    cloth: '#e1d3b5',
  },
};
const fallback: PieceStyle = {
  roof: 'thatch',
  helmet: 'cap',
  shield: 'round',
  ship: 'merchant',
  wall: '#b7a17a',
  roofColor: '#947750',
  trim: '#887454',
  wood: '#6f523b',
  metal: '#84908c',
  cloth: '#bd9b76',
};
export const pieceStyle = (civilization?: string): PieceStyle =>
  civilizationPieceStyles[civilization ?? ''] ?? fallback;
