const accents: Record<string, string> = {
  China: '#28766d',
  Greece: '#3868ab',
  Rome: '#a84a4e',
  Egypt: '#ab8136',
  India: '#98614f',
  Babylonia: '#5265a2',
  Persia: '#76558b',
  Vikings: '#39788e',
  Carthage: '#8a527e',
  Phoenicia: '#8a527e',
  Celts: '#4c805e',
  Huns: '#87634d',
  Japan: '#a84a4e',
  Maya: '#338279',
  Aztecs: '#458362',
};

export const civilizationAccent = (name?: string) => accents[name ?? ''] ?? '#39788e';
